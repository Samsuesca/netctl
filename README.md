# netctl

> Network monitoring and management CLI for macOS

![macOS](https://img.shields.io/badge/macOS-Apple_Silicon-blue)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![License](https://img.shields.io/badge/license-MIT-green)

**netctl** is a command-line tool for monitoring network activity, testing connectivity, managing bandwidth, and debugging network issues on macOS.

---

## Features

- **Network Speed Test**: Upload/download speed measurement
- **Active Connections**: List all network connections by app
- **Bandwidth Monitor**: Real-time bandwidth usage per application
- **Connection Quality**: Latency, packet loss, jitter
- **Firewall/Blocker**: Temporarily block domains (focus mode)
- **VPN Status**: Check VPN connection status
- **DNS Diagnostics**: DNS resolution testing and cache management

---

## Installation

```bash
git clone https://github.com/Samsuesca/netctl.git
cd netctl
cargo build --release
cargo install --path .
```

---

## Usage

### Network Speed Test

```bash
# Quick speed test
netctl speed

# Test with specific server
netctl speed --server cloudflare

# Detailed test (latency, jitter, packet loss)
netctl speed --detailed

# Export results
netctl speed --output speedtest_2026-02-15.json
```

**Output:**
```
┌─────────────────────────────────────────────────────────┐
│                   NETWORK SPEED TEST                    │
├─────────────────────────────────────────────────────────┤
│ Server:          Cloudflare (Bogotá)                    │
│ Ping:            24 ms                                   │
│                                                          │
│ Download:        ⬇️  187.3 Mbps                          │
│ Upload:          ⬆️  45.6 Mbps                           │
│                                                          │
│ Latency:         24 ms (excellent)                       │
│ Jitter:          3 ms                                    │
│ Packet Loss:     0.0%                                    │
│                                                          │
│ Connection:      ✅ Excellent for video calls & streaming│
└─────────────────────────────────────────────────────────┘
```

### Active Connections

```bash
# List all active network connections
netctl connections

# Filter by application
netctl connections --app "Google Chrome"

# Show only external connections
netctl connections --external

# Continuous monitoring
netctl connections --watch --interval 2
```

**Output:**
```
Active Network Connections:

┌─────┬──────────────────────┬─────────────────────┬───────────┬────────┐
│ PID │ Application          │ Remote Address      │ Protocol  │ State  │
├─────┼──────────────────────┼─────────────────────┼───────────┼────────┤
│ 1234│ Google Chrome        │ 142.250.185.46:443  │ TCP/HTTPS │ ESTAB  │
│ 1234│ Google Chrome        │ 172.217.15.78:443   │ TCP/HTTPS │ ESTAB  │
│ 2345│ Visual Studio Code   │ 20.190.151.6:443    │ TCP/HTTPS │ ESTAB  │
│ 3456│ Spotify              │ 35.186.224.25:443   │ TCP/HTTPS │ ESTAB  │
│ 4567│ Docker Desktop       │ 192.168.65.3:2375   │ TCP       │ ESTAB  │
│ 5678│ postgres             │ 127.0.0.1:5432      │ TCP       │ LISTEN │
└─────┴──────────────────────┴─────────────────────┴───────────┴────────┘

Total connections: 47 (6 shown)
External: 23 | Local: 24
```

### Bandwidth Monitor

```bash
# Real-time bandwidth usage
netctl bandwidth

# Show top bandwidth consumers
netctl bandwidth --top 10

# Monitor specific app
netctl bandwidth --app "Docker Desktop"

# Set alert threshold
netctl bandwidth --alert 10MB
```

**Output:**
```
Real-time Bandwidth Usage:

┌──────────────────────────┬────────────┬────────────┬────────────┐
│ Application              │ Download   │ Upload     │ Total      │
├──────────────────────────┼────────────┼────────────┼────────────┤
│ Google Chrome            │ 2.3 MB/s   │ 145 KB/s   │ 2.4 MB/s   │
│ Dropbox                  │ 478 KB/s   │ 1.2 MB/s   │ 1.7 MB/s   │
│ Docker Desktop           │ 234 KB/s   │ 189 KB/s   │ 423 KB/s   │
│ Spotify                  │ 156 KB/s   │ 12 KB/s    │ 168 KB/s   │
│ Visual Studio Code       │ 89 KB/s    │ 45 KB/s    │ 134 KB/s   │
│ Other (12 apps)          │ 123 KB/s   │ 67 KB/s    │ 190 KB/s   │
└──────────────────────────┴────────────┴────────────┴────────────┘

Total: ⬇️  3.4 MB/s  ⬆️  1.7 MB/s

Network Interface: en0 (Wi-Fi)
[████████████████░░░░] 45% utilization
```

### Connection Quality

```bash
# Ping test with stats
netctl ping google.com

# Continuous monitoring
netctl ping google.com --count 100

# Ping multiple hosts
netctl ping --hosts google.com,cloudflare.com,github.com
```

**Output:**
```
Ping Statistics: google.com (142.250.185.46)

Packets: 100 sent, 100 received, 0% loss
Time: 10.2 seconds

Latency:
  Min:     18 ms
  Avg:     24 ms
  Max:     89 ms
  Std Dev: 12 ms
  Jitter:  6 ms

Quality: ✅ Excellent (suitable for real-time apps)
```

### Domain Blocker (Focus Mode)

```bash
# Block distracting domains
netctl block --add twitter.com,facebook.com,youtube.com

# List blocked domains
netctl block --list

# Remove block
netctl block --remove twitter.com

# Enable/disable all blocks
netctl block --enable
netctl block --disable

# Temporary block (auto-remove after duration)
netctl block --add reddit.com --duration 2h
```

**Output:**
```
🚫 Domain Blocker

Currently blocked:
  ✓ twitter.com
  ✓ facebook.com
  ✓ youtube.com
  ⏱️  reddit.com (expires in 1h 34m)

Status: ✅ Active

Blocks are implemented via /etc/hosts
Requires sudo to enable/disable
```

### VPN Status

```bash
# Check VPN connection
netctl vpn status

# Show VPN details
netctl vpn status --detailed

# Monitor VPN connection
netctl vpn watch
```

**Output:**
```
VPN Status:

State:           ✅ Connected
Server:          us-west-2.vpnprovider.com
Protocol:        WireGuard
IP Address:      192.168.1.45 → 203.0.113.42
DNS Servers:     1.1.1.1, 1.0.0.1
Connected:       2h 15m

Tunnel Stats:
  Data Sent:     234 MB
  Data Received: 1.2 GB
  Throughput:    ⬇️  145 KB/s  ⬆️  23 KB/s
```

### DNS Diagnostics

```bash
# Test DNS resolution
netctl dns resolve example.com

# Flush DNS cache
netctl dns flush

# Show current DNS servers
netctl dns servers

# Test DNS performance (multiple resolvers)
netctl dns benchmark
```

**Output:**
```
DNS Benchmark Results:

┌────────────────────────┬──────────────┬────────────┐
│ DNS Server             │ Avg Latency  │ Success    │
├────────────────────────┼──────────────┼────────────┤
│ 1.1.1.1 (Cloudflare)   │ 12 ms        │ 100%       │
│ 8.8.8.8 (Google)       │ 18 ms        │ 100%       │
│ 208.67.222.222 (Cisco) │ 24 ms        │ 100%       │
│ ISP Default            │ 45 ms        │ 98%        │
└────────────────────────┴──────────────┴────────────┘

Recommendation: Use 1.1.1.1 (Cloudflare) for best performance
```

---

## Command Reference

| Command | Description | Options |
|---------|-------------|---------|
| `speed` | Network speed test | `--server`, `--detailed`, `--output` |
| `connections` | Active connections | `--app`, `--external`, `--watch` |
| `bandwidth` | Bandwidth monitor | `--top`, `--app`, `--alert` |
| `ping` | Connection quality | `--count`, `--hosts` |
| `block` | Domain blocker | `--add`, `--remove`, `--list`, `--duration` |
| `vpn` | VPN status | `--detailed`, `watch` |
| `dns` | DNS diagnostics | `resolve`, `flush`, `servers`, `benchmark` |

---

## Use Cases

### API Development

```bash
# Monitor API calls during development
netctl connections --app "node" --watch

# Test API endpoint latency
netctl ping api.example.com

# Check bandwidth usage during testing
netctl bandwidth --app "Postman"
```

### Debugging Network Issues

```bash
# Is the issue DNS or connectivity?
netctl dns resolve api.production.com
netctl ping api.production.com

# Check if VPN is affecting connection
netctl vpn status --detailed

# Monitor connection quality
netctl ping production-server.com --count 100
```

### Focus Mode (Deep Work)

```bash
# Block distractions for 2 hours
sudo netctl block --add twitter.com,youtube.com,reddit.com --duration 2h

# Verify blocks are active
netctl block --list

# Auto-disable after duration
```

### Network Performance Monitoring

```bash
# Daily speed test (add to cron)
netctl speed --output ~/network_logs/speed_$(date +%Y%m%d).json

# Monitor bandwidth during Docker builds
netctl bandwidth --app "Docker Desktop" --watch
```

---

## Technical Stack

**Language**: Rust 2021 edition

**Dependencies**:
- `clap` - CLI parsing
- `reqwest` - HTTP requests (speed test)
- `tokio` - Async runtime
- `surge-ping` or `pnet` - ICMP ping
- `sysinfo` - Process information
- `colored` - Terminal colors
- `tabled` - Table formatting
- `serde` / `serde_json` - Data serialization

---

## Architecture

```
src/
├── main.rs           # CLI entry point
├── speed.rs          # Speed test engine
├── connections.rs    # Active connections (lsof/netstat parsing)
├── bandwidth.rs      # Bandwidth monitor (nettop/system API)
├── ping.rs           # ICMP ping implementation
├── block.rs          # /etc/hosts manipulation
├── vpn.rs            # VPN status detection
├── dns.rs            # DNS diagnostics
└── display.rs        # Formatted output
```

---

## Implementation Notes

### Speed Test

Use HTTP downloads/uploads to measure speed:
- Download: Fetch large file from Cloudflare/Google CDN
- Upload: POST data to speed test server
- Measure throughput and latency

### Active Connections

Parse `lsof -i` or `netstat -an` output:
```bash
lsof -i -n -P | grep ESTABLISHED
```

Map PIDs to app names using `ps`.

### Bandwidth Monitor

Use `nettop` (macOS) or system APIs to track per-process network usage:
```bash
nettop -P -L 1 -J bytes_in,bytes_out
```

### Domain Blocking

Modify `/etc/hosts` (requires sudo):
```
127.0.0.1 twitter.com
127.0.0.1 facebook.com
```

Store original hosts file as backup.

---

## Platform Support

| Platform | Support |
|----------|---------|
| macOS (Apple Silicon) | ✅ Full support |
| macOS (Intel) | ✅ Full support |
| Linux | ⚠️ Partial (different tools) |
| Windows | ❌ Not supported |

---

## Roadmap

- [ ] HTTP/HTTPS proxy detection
- [ ] Network interface switching
- [ ] Packet capture (tcpdump integration)
- [ ] Wi-Fi signal strength monitoring
- [ ] Port scanning (security audit)
- [ ] Historical bandwidth analytics

---

## License

MIT License

---

## Author

**Angel Samuel Suesca Ríos**
suescapsam@gmail.com

---

**Perfect for**: Developers debugging network issues, remote workers monitoring connection quality, anyone needing focus mode.
