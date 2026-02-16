use colored::Colorize;
use std::collections::HashMap;
use tabled::{Table, settings::Style};
use serde::Serialize;
use crate::utils::get_process_name;

#[derive(Debug, Clone, Serialize, tabled::Tabled)]
struct Connection {
    #[tabled(rename = "PID")]
    pid: String,
    #[tabled(rename = "Application")]
    application: String,
    #[tabled(rename = "Remote Address")]
    remote_address: String,
    #[tabled(rename = "Protocol")]
    protocol: String,
    #[tabled(rename = "State")]
    state: String,
}

fn is_local_address(addr: &str) -> bool {
    addr.starts_with("127.")
        || addr.starts_with("0.0.0.0")
        || addr.starts_with("[::1]")
        || addr.starts_with("*:")
        || addr.starts_with("localhost")
        || addr.starts_with("192.168.")
        || addr.starts_with("10.")
        || addr.starts_with("172.16.")
}

fn parse_connections() -> Vec<Connection> {
    let mut connections = Vec::new();

    // Try lsof first (works on macOS and Linux)
    if let Ok(output) = std::process::Command::new("lsof")
        .args(["-i", "-n", "-P"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 9 {
                continue;
            }

            let app = parts[0].to_string();
            let pid = parts[1].to_string();

            // Parse the connection details
            let name_field = parts.last().unwrap_or(&"");
            let state = if parts.len() > 9 {
                parts[parts.len() - 1].to_string()
            } else {
                "".to_string()
            };

            // The connection info is usually in column 8 (NAME)
            let conn_info = if state == "(ESTABLISHED)" || state == "(LISTEN)" || state == "(CLOSE_WAIT)" {
                parts[parts.len() - 2].to_string()
            } else {
                name_field.to_string()
            };

            // Determine protocol
            let proto_field = parts.get(7).unwrap_or(&"");
            let protocol = if proto_field.contains("TCP") {
                if conn_info.contains(":443") {
                    "TCP/HTTPS".to_string()
                } else if conn_info.contains(":80") {
                    "TCP/HTTP".to_string()
                } else {
                    "TCP".to_string()
                }
            } else if proto_field.contains("UDP") {
                "UDP".to_string()
            } else {
                proto_field.to_string()
            };

            // Extract remote address from the connection info
            let remote = if conn_info.contains("->") {
                conn_info.split("->").nth(1).unwrap_or(&conn_info).to_string()
            } else {
                conn_info.to_string()
            };

            let state_clean = state
                .trim_start_matches('(')
                .trim_end_matches(')')
                .to_string();
            let state_abbr = match state_clean.as_str() {
                "ESTABLISHED" => "ESTAB",
                "LISTEN" => "LISTEN",
                "CLOSE_WAIT" => "CLOSE_W",
                "TIME_WAIT" => "TIME_W",
                other => other,
            }
            .to_string();

            connections.push(Connection {
                pid,
                application: app,
                remote_address: remote,
                protocol,
                state: state_abbr,
            });
        }
    }

    // Fallback to ss / netstat on Linux if lsof returned nothing
    if connections.is_empty() {
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

                let state = parts[1].to_string();
                let local_addr = parts[4].to_string();
                let remote_addr = parts[5].to_string();
                let protocol = parts[0].to_uppercase();

                // Extract PID from the last column
                let pid_info = parts.get(6).unwrap_or(&"");
                let pid = if pid_info.contains("pid=") {
                    pid_info
                        .split("pid=")
                        .nth(1)
                        .and_then(|s| s.split(',').next())
                        .unwrap_or("-")
                        .to_string()
                } else {
                    "-".to_string()
                };

                let app_name = get_process_name(&pid);

                let state_abbr = match state.as_str() {
                    "ESTAB" => "ESTAB",
                    "LISTEN" => "LISTEN",
                    "CLOSE-WAIT" => "CLOSE_W",
                    "TIME-WAIT" => "TIME_W",
                    other => other,
                }
                .to_string();

                let _ = local_addr; // used implicitly for context

                connections.push(Connection {
                    pid,
                    application: app_name,
                    remote_address: remote_addr,
                    protocol,
                    state: state_abbr,
                });
            }
        }
    }

    connections
}

pub async fn run(
    app_filter: Option<String>,
    external_only: bool,
    watch: bool,
    interval: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // Clear screen in watch mode
        if watch {
            print!("\x1B[2J\x1B[H");
        }

        let mut connections = parse_connections();

        // Apply filters
        if let Some(ref app) = app_filter {
            let app_lower = app.to_lowercase();
            connections.retain(|c| c.application.to_lowercase().contains(&app_lower));
        }

        if external_only {
            connections.retain(|c| !is_local_address(&c.remote_address));
        }

        // Count totals before truncating display
        let total = connections.len();
        let external_count = connections.iter().filter(|c| !is_local_address(&c.remote_address)).count();
        let local_count = total - external_count;

        // Deduplicate by aggregating similar connections
        let mut seen: HashMap<String, Connection> = HashMap::new();
        for conn in &connections {
            let key = format!("{}:{}:{}", conn.pid, conn.remote_address, conn.state);
            seen.entry(key).or_insert_with(|| conn.clone());
        }
        let mut display_conns: Vec<Connection> = seen.into_values().collect();
        display_conns.sort_by(|a, b| a.application.cmp(&b.application));

        // Limit display
        let shown = display_conns.len().min(30);
        display_conns.truncate(shown);

        println!();
        println!("{}", "Active Network Connections:".bold());
        println!();

        if display_conns.is_empty() {
            println!("  No active connections found.");
        } else {
            let table = Table::new(&display_conns)
                .with(Style::modern())
                .to_string();
            println!("{}", table);
            println!();
            println!(
                "Total connections: {} ({} shown)",
                total.to_string().bold(),
                shown
            );
            println!(
                "External: {} | Local: {}",
                external_count.to_string().cyan(),
                local_count.to_string().dimmed()
            );
        }

        if !watch {
            break;
        }

        println!();
        println!(
            "{}",
            format!("Refreshing every {}s... (Ctrl+C to stop)", interval).dimmed()
        );
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
    }

    println!();
    Ok(())
}
