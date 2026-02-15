use colored::Colorize;

#[derive(Debug)]
struct VpnInfo {
    connected: bool,
    interface: Option<String>,
    server: Option<String>,
    protocol: Option<String>,
    local_ip: Option<String>,
    vpn_ip: Option<String>,
    dns_servers: Vec<String>,
    connected_since: Option<String>,
    bytes_sent: Option<u64>,
    bytes_received: Option<u64>,
}

fn detect_vpn() -> VpnInfo {
    let mut info = VpnInfo {
        connected: false,
        interface: None,
        server: None,
        protocol: None,
        local_ip: None,
        vpn_ip: None,
        dns_servers: Vec::new(),
        connected_since: None,
        bytes_sent: None,
        bytes_received: None,
    };

    // Check for common VPN interfaces
    let vpn_interfaces = detect_vpn_interfaces();
    if let Some((iface, proto)) = vpn_interfaces {
        info.connected = true;
        info.interface = Some(iface.clone());
        info.protocol = Some(proto);

        // Get VPN interface IP
        info.vpn_ip = get_interface_ip(&iface);

        // Get local IP from primary interface
        info.local_ip = get_local_ip();

        // Get DNS servers
        info.dns_servers = get_dns_servers();

        // Try to get traffic stats for the interface
        let (sent, recv) = get_interface_stats(&iface);
        info.bytes_sent = Some(sent);
        info.bytes_received = Some(recv);
    }

    // Check for WireGuard specifically
    if !info.connected {
        if let Some(wg_info) = detect_wireguard() {
            info.connected = true;
            info.protocol = Some("WireGuard".to_string());
            info.server = Some(wg_info.1);
            info.vpn_ip = get_interface_ip(&wg_info.0);
            info.interface = Some(wg_info.0);
            info.local_ip = get_local_ip();
            info.dns_servers = get_dns_servers();
        }
    }

    info
}

fn detect_vpn_interfaces() -> Option<(String, String)> {
    // Check ifconfig / ip for VPN-related interfaces
    let tun_interfaces = ["utun", "tun", "tap", "ppp", "wg", "ipsec", "gif"];

    // Try ip link on Linux
    if let Ok(output) = std::process::Command::new("ip")
        .args(["link", "show"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            for prefix in &tun_interfaces {
                if line.contains(prefix) && line.contains("UP") {
                    let name = line
                        .split(':')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .split('@')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    let proto = if name.starts_with("wg") {
                        "WireGuard"
                    } else if name.starts_with("tun") || name.starts_with("utun") {
                        "OpenVPN/IKEv2"
                    } else if name.starts_with("ppp") {
                        "PPTP/L2TP"
                    } else {
                        "VPN"
                    };
                    return Some((name, proto.to_string()));
                }
            }
        }
    }

    // Try ifconfig on macOS
    if let Ok(output) = std::process::Command::new("ifconfig").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut current_iface = String::new();
        for line in stdout.lines() {
            if !line.starts_with('\t') && !line.starts_with(' ') && line.contains(':') {
                current_iface = line.split(':').next().unwrap_or("").to_string();
            }
            for prefix in &tun_interfaces {
                if current_iface.starts_with(prefix) && line.contains("inet ") {
                    let proto = if current_iface.starts_with("wg") {
                        "WireGuard"
                    } else if current_iface.starts_with("utun") || current_iface.starts_with("tun")
                    {
                        "OpenVPN/IKEv2"
                    } else if current_iface.starts_with("ppp") {
                        "PPTP/L2TP"
                    } else {
                        "VPN"
                    };
                    return Some((current_iface.clone(), proto.to_string()));
                }
            }
        }
    }

    None
}

fn detect_wireguard() -> Option<(String, String)> {
    if let Ok(output) = std::process::Command::new("wg").args(["show"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            let iface = stdout
                .lines()
                .next()
                .and_then(|l| l.split(':').next())
                .unwrap_or("wg0")
                .trim()
                .to_string();
            let endpoint = stdout
                .lines()
                .find(|l| l.contains("endpoint"))
                .and_then(|l| l.split(':').nth(1))
                .unwrap_or("unknown")
                .trim()
                .to_string();
            return Some((iface, endpoint));
        }
    }
    None
}

fn get_interface_ip(iface: &str) -> Option<String> {
    // Linux
    if let Ok(output) = std::process::Command::new("ip")
        .args(["addr", "show", iface])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("inet ") {
                return trimmed
                    .split_whitespace()
                    .nth(1)
                    .map(|s| s.split('/').next().unwrap_or(s).to_string());
            }
        }
    }
    // macOS
    if let Ok(output) = std::process::Command::new("ifconfig")
        .arg(iface)
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("inet ") {
                return trimmed.split_whitespace().nth(1).map(|s| s.to_string());
            }
        }
    }
    None
}

fn get_local_ip() -> Option<String> {
    // Try to get the primary (non-VPN) IP
    if let Ok(output) = std::process::Command::new("hostname").args(["-I"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        return stdout.split_whitespace().next().map(|s| s.to_string());
    }
    // macOS fallback
    if let Ok(output) = std::process::Command::new("ipconfig")
        .args(["getifaddr", "en0"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !stdout.is_empty() {
            return Some(stdout);
        }
    }
    None
}

fn get_dns_servers() -> Vec<String> {
    let mut servers = Vec::new();

    // Read /etc/resolv.conf
    if let Ok(content) = std::fs::read_to_string("/etc/resolv.conf") {
        for line in content.lines() {
            if line.starts_with("nameserver") {
                if let Some(server) = line.split_whitespace().nth(1) {
                    servers.push(server.to_string());
                }
            }
        }
    }

    // macOS: scutil --dns
    if servers.is_empty() {
        if let Ok(output) = std::process::Command::new("scutil")
            .args(["--dns"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("nameserver[") {
                    if let Some(server) = line.split(':').nth(1) {
                        let server = server.trim().to_string();
                        if !servers.contains(&server) {
                            servers.push(server);
                        }
                    }
                }
            }
        }
    }

    servers
}

fn get_interface_stats(iface: &str) -> (u64, u64) {
    // Linux: /sys/class/net/<iface>/statistics/
    let tx_path = format!("/sys/class/net/{}/statistics/tx_bytes", iface);
    let rx_path = format!("/sys/class/net/{}/statistics/rx_bytes", iface);

    let sent = std::fs::read_to_string(&tx_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let recv = std::fs::read_to_string(&rx_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    // macOS: netstat -I <iface> -b
    if sent == 0 && recv == 0 {
        if let Ok(output) = std::process::Command::new("netstat")
            .args(["-I", iface, "-b"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().nth(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                // Columns vary, but bytes in/out are typically near the end
                if parts.len() >= 7 {
                    let ibytes: u64 = parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
                    let obytes: u64 = parts.get(9).and_then(|s| s.parse().ok()).unwrap_or(0);
                    return (obytes, ibytes);
                }
            }
        }
    }

    (sent, recv)
}

fn format_bytes(bytes: u64) -> String {
    crate::display::format_bytes(bytes)
}

fn print_vpn_status(info: &VpnInfo, detailed: bool) {
    println!();
    println!("{}", "VPN Status:".bold());
    println!();

    if info.connected {
        println!(
            "State:           {} Connected",
            "OK".green()
        );
        if let Some(ref server) = info.server {
            println!("Server:          {}", server);
        }
        if let Some(ref proto) = info.protocol {
            println!("Protocol:        {}", proto);
        }
        if let Some(ref iface) = info.interface {
            println!("Interface:       {}", iface);
        }

        let local = info.local_ip.as_deref().unwrap_or("unknown");
        let vpn = info.vpn_ip.as_deref().unwrap_or("unknown");
        println!("IP Address:      {} -> {}", local.dimmed(), vpn.cyan());

        if !info.dns_servers.is_empty() {
            println!("DNS Servers:     {}", info.dns_servers.join(", "));
        }

        if let Some(ref since) = info.connected_since {
            println!("Connected:       {}", since);
        }

        if detailed {
            println!();
            println!("{}:", "Tunnel Stats".bold());
            if let Some(sent) = info.bytes_sent {
                println!("  Data Sent:     {}", format_bytes(sent));
            }
            if let Some(recv) = info.bytes_received {
                println!("  Data Received: {}", format_bytes(recv));
            }
        }
    } else {
        println!(
            "State:           {} Not Connected",
            "--".red()
        );
        println!();
        println!("{}", "No active VPN tunnel detected.".dimmed());
        println!(
            "{}",
            "Checked: tun/utun/tap/ppp/wg/ipsec interfaces".dimmed()
        );
    }

    println!();
}

pub async fn status(detailed: bool) -> Result<(), Box<dyn std::error::Error>> {
    let info = detect_vpn();
    print_vpn_status(&info, detailed);
    Ok(())
}

pub async fn watch() -> Result<(), Box<dyn std::error::Error>> {
    loop {
        print!("\x1B[2J\x1B[H");
        let info = detect_vpn();
        print_vpn_status(&info, true);
        println!("{}", "Refreshing every 5s... (Ctrl+C to stop)".dimmed());
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
