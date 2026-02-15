use colored::Colorize;
use std::time::Instant;
use tabled::{Table, settings::Style};

#[derive(tabled::Tabled)]
struct BenchmarkRow {
    #[tabled(rename = "DNS Server")]
    server: String,
    #[tabled(rename = "Avg Latency")]
    avg_latency: String,
    #[tabled(rename = "Success")]
    success: String,
}

/// Resolve a domain using the system resolver and display results.
pub async fn resolve(domain: &str) -> Result<(), Box<dyn std::error::Error>> {
    use dns_lookup::lookup_host;

    println!();
    println!("{} {}...", "Resolving".dimmed(), domain.cyan());
    println!();

    let start = Instant::now();
    match lookup_host(domain) {
        Ok(ips) => {
            let elapsed = start.elapsed();
            println!("{} {} -> ", "DNS Resolution:".bold(), domain.cyan());
            println!();

            let mut ipv4 = Vec::new();
            let mut ipv6 = Vec::new();
            for ip in &ips {
                if ip.is_ipv4() {
                    ipv4.push(ip.to_string());
                } else {
                    ipv6.push(ip.to_string());
                }
            }

            if !ipv4.is_empty() {
                println!("  IPv4:");
                for ip in &ipv4 {
                    println!("    {}", ip.green());
                }
            }
            if !ipv6.is_empty() {
                println!("  IPv6:");
                for ip in &ipv6 {
                    println!("    {}", ip.green());
                }
            }

            println!();
            println!("  Resolved in {:.1} ms", elapsed.as_secs_f64() * 1000.0);
            println!("  Records: {} IPv4, {} IPv6", ipv4.len(), ipv6.len());
        }
        Err(e) => {
            println!(
                "  {} Could not resolve {}: {}",
                "Error:".red(),
                domain,
                e
            );
        }
    }

    println!();
    Ok(())
}

/// Flush the DNS cache (platform-specific).
pub async fn flush() -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("{}", "Flushing DNS cache...".dimmed());

    // macOS
    let macos_result = std::process::Command::new("dscacheutil")
        .args(["-flushcache"])
        .output();

    let _ = std::process::Command::new("killall")
        .args(["-HUP", "mDNSResponder"])
        .output();

    // Linux (systemd-resolved)
    let linux_result = std::process::Command::new("resolvectl")
        .args(["flush-caches"])
        .output();

    // Linux alternative
    let _ = std::process::Command::new("systemd-resolve")
        .args(["--flush-caches"])
        .output();

    let success = macos_result.map(|o| o.status.success()).unwrap_or(false)
        || linux_result.map(|o| o.status.success()).unwrap_or(false);

    if success {
        println!("  {} DNS cache flushed successfully", "OK".green());
    } else {
        println!(
            "  {} Could not flush DNS cache (may require sudo)",
            "!!".yellow()
        );
    }

    println!();
    Ok(())
}

/// Show the currently configured DNS servers.
pub async fn servers() -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("{}", "Current DNS Servers:".bold());
    println!();

    let mut found = false;

    // Read /etc/resolv.conf
    if let Ok(content) = std::fs::read_to_string("/etc/resolv.conf") {
        for line in content.lines() {
            if line.starts_with("nameserver") {
                if let Some(server) = line.split_whitespace().nth(1) {
                    let label = identify_dns_server(server);
                    println!("  {} {}", server.cyan(), label.dimmed());
                    found = true;
                }
            }
        }
    }

    // macOS: scutil --dns
    if !found {
        if let Ok(output) = std::process::Command::new("scutil")
            .args(["--dns"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("nameserver[") {
                    if let Some(server) = line.split(':').nth(1) {
                        let server = server.trim();
                        let label = identify_dns_server(server);
                        println!("  {} {}", server.cyan(), label.dimmed());
                        found = true;
                    }
                }
            }
        }
    }

    if !found {
        println!("  No DNS servers found.");
    }

    println!();
    Ok(())
}

/// Benchmark multiple well-known DNS resolvers.
pub async fn benchmark() -> Result<(), Box<dyn std::error::Error>> {
    let resolvers = vec![
        ("1.1.1.1", "Cloudflare"),
        ("8.8.8.8", "Google"),
        ("208.67.222.222", "Cisco/OpenDNS"),
        ("9.9.9.9", "Quad9"),
    ];

    // Also include system default
    let system_dns = get_system_dns();

    println!();
    println!("{}", "Running DNS benchmark...".dimmed());
    println!();

    let test_domains = vec![
        "google.com",
        "github.com",
        "cloudflare.com",
        "amazon.com",
        "microsoft.com",
    ];

    let mut rows: Vec<BenchmarkRow> = Vec::new();
    let mut best_latency = f64::MAX;
    let mut best_server = String::new();

    for (server_ip, server_name) in &resolvers {
        let (avg, success_rate) =
            benchmark_dns_server(server_ip, &test_domains).await;

        if avg < best_latency && avg > 0.0 {
            best_latency = avg;
            best_server = format!("{} ({})", server_ip, server_name);
        }

        rows.push(BenchmarkRow {
            server: format!("{} ({})", server_ip, server_name),
            avg_latency: if avg > 0.0 {
                format!("{:.0} ms", avg)
            } else {
                "timeout".to_string()
            },
            success: format!("{:.0}%", success_rate),
        });
    }

    // Test system DNS if available
    if let Some(sys_dns) = system_dns {
        let (avg, success_rate) =
            benchmark_dns_server(&sys_dns, &test_domains).await;

        if avg < best_latency && avg > 0.0 {
            let _ = best_latency;
            best_server = format!("{} (System)", sys_dns);
        }

        rows.push(BenchmarkRow {
            server: format!("{} (System)", sys_dns),
            avg_latency: if avg > 0.0 {
                format!("{:.0} ms", avg)
            } else {
                "timeout".to_string()
            },
            success: format!("{:.0}%", success_rate),
        });
    }

    println!("{}", "DNS Benchmark Results:".bold());
    println!();

    let table = Table::new(&rows)
        .with(Style::modern())
        .to_string();
    println!("{}", table);

    println!();
    println!(
        "Recommendation: Use {} for best performance",
        best_server.green()
    );
    println!();

    Ok(())
}

async fn benchmark_dns_server(server: &str, domains: &[&str]) -> (f64, f64) {
    let mut latencies = Vec::new();
    let mut successes = 0;
    let total = domains.len();

    for domain in domains {
        // Use dig/nslookup to query the specific DNS server
        let start = Instant::now();
        let result = std::process::Command::new("dig")
            .args([format!("@{}", server), domain.to_string(), "+short".to_string(), "+time=2".to_string(), "+tries=1".to_string()])
            .output();

        match result {
            Ok(output) => {
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                let stdout = String::from_utf8_lossy(&output.stdout);
                if output.status.success() && !stdout.trim().is_empty() {
                    latencies.push(elapsed);
                    successes += 1;
                }
            }
            Err(_) => {
                // Try nslookup as fallback
                let start2 = Instant::now();
                if let Ok(output) = std::process::Command::new("nslookup")
                    .args([domain, server])
                    .output()
                {
                    let elapsed = start2.elapsed().as_secs_f64() * 1000.0;
                    if output.status.success() {
                        latencies.push(elapsed);
                        successes += 1;
                    }
                }
            }
        }
    }

    let avg = if latencies.is_empty() {
        0.0
    } else {
        latencies.iter().sum::<f64>() / latencies.len() as f64
    };
    let success_rate = (successes as f64 / total as f64) * 100.0;

    (avg, success_rate)
}

fn get_system_dns() -> Option<String> {
    if let Ok(content) = std::fs::read_to_string("/etc/resolv.conf") {
        for line in content.lines() {
            if line.starts_with("nameserver") {
                return line.split_whitespace().nth(1).map(|s| s.to_string());
            }
        }
    }
    None
}

fn identify_dns_server(ip: &str) -> String {
    match ip {
        "1.1.1.1" | "1.0.0.1" => "(Cloudflare)".to_string(),
        "8.8.8.8" | "8.8.4.4" => "(Google)".to_string(),
        "208.67.222.222" | "208.67.220.220" => "(Cisco/OpenDNS)".to_string(),
        "9.9.9.9" | "149.112.112.112" => "(Quad9)".to_string(),
        _ => "(ISP/Custom)".to_string(),
    }
}
