use colored::Colorize;
use std::collections::HashMap;
use tabled::{Table, settings::Style};
use crate::utils::{format_rate, get_process_name};

#[derive(Debug, Clone, tabled::Tabled)]
struct AppBandwidth {
    #[tabled(rename = "Application")]
    application: String,
    #[tabled(rename = "Download")]
    download: String,
    #[tabled(rename = "Upload")]
    upload: String,
    #[tabled(rename = "Total")]
    total: String,
}

#[derive(Debug, Default, Clone)]
struct RawBandwidth {
    bytes_in: u64,
    bytes_out: u64,
}

fn parse_alert_bytes(alert: &str) -> Option<u64> {
    let alert = alert.trim().to_uppercase();
    if let Some(num) = alert.strip_suffix("GB") {
        num.trim().parse::<f64>().ok().map(|n| (n * 1_000_000_000.0) as u64)
    } else if let Some(num) = alert.strip_suffix("MB") {
        num.trim().parse::<f64>().ok().map(|n| (n * 1_000_000.0) as u64)
    } else if let Some(num) = alert.strip_suffix("KB") {
        num.trim().parse::<f64>().ok().map(|n| (n * 1_000.0) as u64)
    } else {
        alert.parse::<u64>().ok()
    }
}


/// Read per-process bandwidth from /proc/net or platform-specific tools.
fn read_bandwidth() -> HashMap<String, RawBandwidth> {
    let mut app_bw: HashMap<String, RawBandwidth> = HashMap::new();

    // Try nettop on macOS
    if let Ok(output) = std::process::Command::new("nettop")
        .args(["-P", "-L", "1", "-J", "bytes_in,bytes_out", "-x"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 3 {
                let name = parts[0].trim().split('.').next().unwrap_or(parts[0].trim()).to_string();
                let bytes_in: u64 = parts[1].trim().parse().unwrap_or(0);
                let bytes_out: u64 = parts[2].trim().parse().unwrap_or(0);
                let entry = app_bw.entry(name).or_default();
                entry.bytes_in += bytes_in;
                entry.bytes_out += bytes_out;
            }
        }
    }

    // Fallback: on Linux, read from /proc/net/dev and correlate with process info
    if app_bw.is_empty() {
        // Use ss + /proc approach: get per-socket stats
        if let Ok(output) = std::process::Command::new("ss")
            .args(["-tunap"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 6 {
                    continue;
                }
                let recv_q: u64 = parts[2].parse().unwrap_or(0);
                let send_q: u64 = parts[3].parse().unwrap_or(0);

                let pid_info = parts.get(6).unwrap_or(&"");
                let pid = if pid_info.contains("pid=") {
                    pid_info
                        .split("pid=")
                        .nth(1)
                        .and_then(|s| s.split(',').next())
                        .unwrap_or("-")
                        .to_string()
                } else {
                    continue;
                };

                let app_name = get_process_name(&pid);
                let entry = app_bw.entry(app_name).or_default();
                entry.bytes_in += recv_q;
                entry.bytes_out += send_q;
            }
        }

        // Also try to get interface-level totals
        if app_bw.is_empty() {
            if let Ok(content) = std::fs::read_to_string("/proc/net/dev") {
                for line in content.lines().skip(2) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 10 {
                        let iface = parts[0].trim_end_matches(':');
                        if iface == "lo" {
                            continue;
                        }
                        let bytes_in: u64 = parts[1].parse().unwrap_or(0);
                        let bytes_out: u64 = parts[9].parse().unwrap_or(0);
                        let entry = app_bw.entry(format!("({})", iface)).or_default();
                        entry.bytes_in += bytes_in;
                        entry.bytes_out += bytes_out;
                    }
                }
            }
        }
    }

    app_bw
}

fn get_default_interface() -> String {
    // macOS
    if let Ok(output) = std::process::Command::new("route")
        .args(["-n", "get", "default"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("interface:") {
                return line.split(':').nth(1).unwrap_or("unknown").trim().to_string();
            }
        }
    }
    // Linux
    if let Ok(output) = std::process::Command::new("ip")
        .args(["route", "show", "default"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = stdout.lines().next() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(idx) = parts.iter().position(|&p| p == "dev") {
                if let Some(iface) = parts.get(idx + 1) {
                    return iface.to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

pub async fn run(
    top: Option<usize>,
    app_filter: Option<String>,
    alert: Option<String>,
    watch: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let alert_bytes = alert.as_deref().and_then(parse_alert_bytes);

    loop {
        if watch {
            print!("\x1B[2J\x1B[H");
        }

        let bw_data = read_bandwidth();
        let mut entries: Vec<(String, RawBandwidth)> = bw_data.into_iter().collect();

        // Apply app filter
        if let Some(ref app) = app_filter {
            let app_lower = app.to_lowercase();
            entries.retain(|(name, _)| name.to_lowercase().contains(&app_lower));
        }

        // Sort by total bandwidth descending
        entries.sort_by(|a, b| {
            let total_a = a.1.bytes_in + a.1.bytes_out;
            let total_b = b.1.bytes_in + b.1.bytes_out;
            total_b.cmp(&total_a)
        });

        let limit = top.unwrap_or(10);
        let total_down: u64 = entries.iter().map(|(_, b)| b.bytes_in).sum();
        let total_up: u64 = entries.iter().map(|(_, b)| b.bytes_out).sum();

        // Group remaining as "Other"
        let (shown, rest) = if entries.len() > limit {
            let (s, r) = entries.split_at(limit);
            (s.to_vec(), Some(r.to_vec()))
        } else {
            (entries, None)
        };

        let mut display_rows: Vec<AppBandwidth> = shown
            .iter()
            .map(|(name, bw)| AppBandwidth {
                application: name.clone(),
                download: format_rate(bw.bytes_in as f64),
                upload: format_rate(bw.bytes_out as f64),
                total: format_rate((bw.bytes_in + bw.bytes_out) as f64),
            })
            .collect();

        if let Some(rest) = rest {
            let rest_in: u64 = rest.iter().map(|(_, b)| b.bytes_in).sum();
            let rest_out: u64 = rest.iter().map(|(_, b)| b.bytes_out).sum();
            display_rows.push(AppBandwidth {
                application: format!("Other ({} apps)", rest.len()),
                download: format_rate(rest_in as f64),
                upload: format_rate(rest_out as f64),
                total: format_rate((rest_in + rest_out) as f64),
            });
        }

        println!();
        println!("{}", "Real-time Bandwidth Usage:".bold());
        println!();

        if display_rows.is_empty() {
            println!("  No bandwidth data available.");
            println!("  (This may require elevated privileges on some systems)");
        } else {
            let table = Table::new(&display_rows)
                .with(Style::modern())
                .to_string();
            println!("{}", table);
            println!();
            println!(
                "Total:  {} {}   {} {}",
                "↓".cyan(),
                format_rate(total_down as f64),
                "↑".green(),
                format_rate(total_up as f64)
            );
        }

        let iface = get_default_interface();
        println!();
        println!("Network Interface: {}", iface.cyan());

        // Alert check
        if let Some(threshold) = alert_bytes {
            let total = total_down + total_up;
            if total > threshold {
                println!();
                println!(
                    "{}",
                    format!(
                        "  ALERT: Bandwidth usage ({}) exceeds threshold!",
                        format_rate(total as f64)
                    )
                    .red()
                    .bold()
                );
            }
        }

        if !watch {
            break;
        }

        println!();
        println!("{}", "Refreshing every 2s... (Ctrl+C to stop)".dimmed());
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    println!();
    Ok(())
}
