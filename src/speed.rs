use crate::display;
use colored::Colorize;
use serde::Serialize;
use std::time::Instant;

#[derive(Serialize)]
struct SpeedResult {
    server: String,
    download_mbps: f64,
    upload_mbps: f64,
    latency_ms: f64,
    jitter_ms: Option<f64>,
    packet_loss_pct: Option<f64>,
    timestamp: String,
}

struct ServerInfo {
    name: &'static str,
    location: &'static str,
    download_url: &'static str,
    upload_url: &'static str,
}

fn get_servers() -> Vec<ServerInfo> {
    vec![
        ServerInfo {
            name: "Cloudflare",
            location: "Edge",
            download_url: "https://speed.cloudflare.com/__down?bytes=25000000",
            upload_url: "https://speed.cloudflare.com/__up",
        },
        ServerInfo {
            name: "Google",
            location: "CDN",
            download_url: "https://www.google.com/generate_204",
            upload_url: "https://www.google.com/generate_204",
        },
    ]
}

fn select_server(name: Option<&str>) -> &'static ServerInfo {
    let servers = get_servers();
    // Leak is fine for static-lifetime CLI data
    let servers: &'static Vec<ServerInfo> = Box::leak(Box::new(servers));
    if let Some(name) = name {
        servers
            .iter()
            .find(|s| s.name.to_lowercase() == name.to_lowercase())
            .unwrap_or(&servers[0])
    } else {
        &servers[0]
    }
}

async fn measure_latency(client: &reqwest::Client, url: &str, samples: u32) -> Vec<f64> {
    let mut latencies = Vec::new();
    for _ in 0..samples {
        let start = Instant::now();
        if client.head(url).send().await.is_ok() {
            latencies.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    latencies
}

async fn measure_download(client: &reqwest::Client, url: &str) -> Result<f64, Box<dyn std::error::Error>> {
    // Perform multiple downloads to get a reliable measurement
    let sizes: Vec<u64> = vec![1_000_000, 5_000_000, 10_000_000, 25_000_000];
    let mut best_mbps = 0.0_f64;

    for size in sizes {
        let download_url = if url.contains("cloudflare") {
            format!("https://speed.cloudflare.com/__down?bytes={}", size)
        } else {
            url.to_string()
        };

        let start = Instant::now();
        let resp = client.get(&download_url).send().await?;
        let bytes = resp.bytes().await?;
        let elapsed = start.elapsed().as_secs_f64();

        if elapsed > 0.0 {
            let mbps = (bytes.len() as f64 * 8.0) / (elapsed * 1_000_000.0);
            best_mbps = best_mbps.max(mbps);
        }
    }

    Ok(best_mbps)
}

async fn measure_upload(client: &reqwest::Client, url: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let payload_sizes: Vec<usize> = vec![500_000, 1_000_000, 5_000_000];
    let mut best_mbps = 0.0_f64;

    for size in payload_sizes {
        let data = vec![0u8; size];
        let start = Instant::now();
        let _ = client.post(url).body(data).send().await?;
        let elapsed = start.elapsed().as_secs_f64();

        if elapsed > 0.0 {
            let mbps = (size as f64 * 8.0) / (elapsed * 1_000_000.0);
            best_mbps = best_mbps.max(mbps);
        }
    }

    Ok(best_mbps)
}

pub async fn run(
    server: Option<String>,
    detailed: bool,
    output: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let server_info = select_server(server.as_deref());

    println!();
    println!("{}", "Running network speed test...".dimmed());
    println!();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    // Measure latency
    print!("  Measuring latency... ");
    let latencies = measure_latency(&client, server_info.download_url, 5).await;
    let avg_latency = if latencies.is_empty() {
        0.0
    } else {
        latencies.iter().sum::<f64>() / latencies.len() as f64
    };
    println!("{}", "done".green());

    // Measure download
    print!("  Measuring download speed... ");
    let download_mbps = measure_download(&client, server_info.download_url).await.unwrap_or(0.0);
    println!("{}", "done".green());

    // Measure upload
    print!("  Measuring upload speed... ");
    let upload_mbps = measure_upload(&client, server_info.upload_url).await.unwrap_or(0.0);
    println!("{}", "done".green());

    // Calculate jitter and packet loss if detailed
    let (jitter, packet_loss) = if detailed {
        let extra_latencies = measure_latency(&client, server_info.download_url, 20).await;
        let jitter = if extra_latencies.len() > 1 {
            let diffs: Vec<f64> = extra_latencies
                .windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .collect();
            Some(diffs.iter().sum::<f64>() / diffs.len() as f64)
        } else {
            Some(0.0)
        };
        let total_sent = 20;
        let total_received = extra_latencies.len();
        let loss = ((total_sent - total_received) as f64 / total_sent as f64) * 100.0;
        (jitter, Some(loss))
    } else {
        (None, None)
    };

    // Display results
    println!();
    display::print_header("NETWORK SPEED TEST");
    display::print_row("Server:", &format!("{} ({})", server_info.name, server_info.location));
    display::print_row("Ping:", &format!("{:.0} ms", avg_latency));
    display::print_empty_row();
    display::print_row("Download:", &format!("  {}", display::format_mbps(download_mbps)));
    display::print_row("Upload:", &format!("  {}", display::format_mbps(upload_mbps)));

    if detailed {
        display::print_empty_row();
        let (_, quality_str) = display::quality_assessment(avg_latency);
        display::print_row("Latency:", &format!("{:.0} ms ({})", avg_latency, quality_str));
        if let Some(j) = jitter {
            display::print_row("Jitter:", &format!("{:.0} ms", j));
        }
        if let Some(loss) = packet_loss {
            display::print_row("Packet Loss:", &format!("{:.1}%", loss));
        }
    }

    display::print_empty_row();
    let (label, _) = display::quality_assessment(avg_latency);
    let status_icon = if label == "Excellent" || label == "Good" {
        "OK".green().to_string()
    } else {
        "!!".yellow().to_string()
    };
    display::print_row(
        "Connection:",
        &format!("{} {}", status_icon, display::quality_assessment(avg_latency).1),
    );
    display::print_footer();

    // Export to JSON if requested
    if let Some(path) = output {
        let result = SpeedResult {
            server: format!("{} ({})", server_info.name, server_info.location),
            download_mbps,
            upload_mbps,
            latency_ms: avg_latency,
            jitter_ms: jitter,
            packet_loss_pct: packet_loss,
            timestamp: chrono::Local::now().to_rfc3339(),
        };
        let json = serde_json::to_string_pretty(&result)?;
        std::fs::write(&path, &json)?;
        println!();
        println!("  Results saved to {}", path.green());
    }

    println!();
    Ok(())
}
