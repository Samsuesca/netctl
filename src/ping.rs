use colored::Colorize;
use std::time::Instant;

struct PingStats {
    host: String,
    ip: String,
    sent: u32,
    received: u32,
    latencies: Vec<f64>,
}

impl PingStats {
    fn loss_pct(&self) -> f64 {
        if self.sent == 0 {
            return 0.0;
        }
        ((self.sent - self.received) as f64 / self.sent as f64) * 100.0
    }

    fn min(&self) -> f64 {
        self.latencies.iter().cloned().fold(f64::INFINITY, f64::min)
    }

    fn max(&self) -> f64 {
        self.latencies.iter().cloned().fold(0.0_f64, f64::max)
    }

    fn avg(&self) -> f64 {
        if self.latencies.is_empty() {
            return 0.0;
        }
        self.latencies.iter().sum::<f64>() / self.latencies.len() as f64
    }

    fn std_dev(&self) -> f64 {
        if self.latencies.len() < 2 {
            return 0.0;
        }
        let avg = self.avg();
        let variance = self.latencies.iter().map(|l| (l - avg).powi(2)).sum::<f64>()
            / (self.latencies.len() - 1) as f64;
        variance.sqrt()
    }

    fn jitter(&self) -> f64 {
        if self.latencies.len() < 2 {
            return 0.0;
        }
        let diffs: Vec<f64> = self
            .latencies
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .collect();
        diffs.iter().sum::<f64>() / diffs.len() as f64
    }
}

/// Resolve a hostname to its IP address.
fn resolve_host(host: &str) -> Option<String> {
    use dns_lookup::lookup_host;
    match lookup_host(host) {
        Ok(ips) => ips.into_iter().find(|ip| ip.is_ipv4()).map(|ip| ip.to_string()),
        Err(_) => None,
    }
}

/// Perform ping using the system `ping` command and parse output.
async fn do_ping(host: &str, count: u32) -> PingStats {
    let ip = resolve_host(host).unwrap_or_else(|| host.to_string());

    let mut stats = PingStats {
        host: host.to_string(),
        ip: ip.clone(),
        sent: count,
        received: 0,
        latencies: Vec::new(),
    };

    // Use system ping command - works on both macOS and Linux
    let output = std::process::Command::new("ping")
        .args(["-c", &count.to_string(), "-W", "2", host])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                // Parse lines like: "64 bytes from ...: icmp_seq=1 ttl=117 time=24.3 ms"
                if line.contains("time=") {
                    if let Some(time_part) = line.split("time=").nth(1) {
                        let ms_str = time_part.split_whitespace().next().unwrap_or("0");
                        // Handle "time=24.3" (no space before ms on some systems)
                        let ms_str = ms_str.trim_end_matches("ms");
                        if let Ok(ms) = ms_str.parse::<f64>() {
                            stats.latencies.push(ms);
                            stats.received += 1;
                        }
                    }
                }
            }
        }
        Err(_) => {
            // Fall back to manual TCP-based ping if system ping is unavailable
            for _ in 0..count {
                let target = format!("{}:80", &ip);
                let start = Instant::now();
                match tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    tokio::net::TcpStream::connect(&target),
                )
                .await
                {
                    Ok(Ok(_)) => {
                        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                        stats.latencies.push(elapsed);
                        stats.received += 1;
                    }
                    _ => {}
                }
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }
    }

    stats
}

fn print_ping_stats(stats: &PingStats) {
    println!();
    println!(
        "{} {} ({})",
        "Ping Statistics:".bold(),
        stats.host.cyan(),
        stats.ip.dimmed()
    );
    println!();
    println!(
        "Packets: {} sent, {} received, {:.0}% loss",
        stats.sent,
        stats.received,
        stats.loss_pct()
    );

    if stats.latencies.is_empty() {
        println!();
        println!("{}", "  No responses received.".red());
        return;
    }

    let total_time: f64 = stats.latencies.iter().sum();
    println!("Time: {:.1} seconds", total_time / 1000.0);

    println!();
    println!("{}:", "Latency".bold());
    println!("  Min:     {:.0} ms", stats.min());
    println!("  Avg:     {:.0} ms", stats.avg());
    println!("  Max:     {:.0} ms", stats.max());
    println!("  Std Dev: {:.0} ms", stats.std_dev());
    println!("  Jitter:  {:.0} ms", stats.jitter());

    println!();
    let avg = stats.avg();
    let quality = if avg < 30.0 {
        format!("{} Excellent (suitable for real-time apps)", "OK".green())
    } else if avg < 60.0 {
        format!("{} Good (suitable for most applications)", "OK".green())
    } else if avg < 100.0 {
        format!("{} Fair (may affect real-time apps)", "!!".yellow())
    } else {
        format!("{} Poor (high latency)", "!!".red())
    };
    println!("Quality: {}", quality);
}

pub async fn run(
    host: Option<String>,
    count: u32,
    hosts: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let targets: Vec<String> = if let Some(hosts_str) = hosts {
        hosts_str.split(',').map(|s| s.trim().to_string()).collect()
    } else if let Some(h) = host {
        vec![h]
    } else {
        // Default targets
        vec!["google.com".to_string()]
    };

    for target in &targets {
        println!();
        println!("{} {}...", "Pinging".dimmed(), target.cyan());
        let stats = do_ping(target, count).await;
        print_ping_stats(&stats);
    }

    println!();
    Ok(())
}
