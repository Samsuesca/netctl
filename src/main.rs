mod bandwidth;
mod block;
mod connections;
mod display;
mod dns;
mod ping;
mod speed;
pub mod utils;
mod vpn;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "netctl",
    version,
    about = "Network monitoring and management CLI",
    after_help = "\
Common workflows:
  Speed test:           netctl speed
  Active connections:   netctl connections --active
  Bandwidth monitor:    netctl bandwidth --watch
  DNS diagnostics:      netctl dns lookup google.com
  Ping with stats:      netctl ping 8.8.8.8 -c 10
  Check VPN:            netctl vpn status
  Block distractions:   netctl block add twitter.com --duration 2h"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Network speed test (download/upload)
    #[command(long_about = "\
Network speed test (download/upload)

Measures download speed, upload speed, and latency against a remote server.
Supports multiple test servers and can export results to JSON for tracking
over time.

Examples:
  netctl speed                         Run a quick speed test (Cloudflare)
  netctl speed --server google         Use Google as the test server
  netctl speed --detailed              Include jitter and packet loss metrics
  netctl speed --output results.json   Save results to a JSON file
  netctl speed --detailed --output ~/speed-log.json")]
    Speed {
        /// Server to use for the test (cloudflare, google)
        #[arg(long)]
        server: Option<String>,

        /// Show detailed metrics (latency, jitter, packet loss)
        #[arg(long)]
        detailed: bool,

        /// Export results to a JSON file
        #[arg(long)]
        output: Option<String>,
    },

    /// List active network connections by application
    #[command(long_about = "\
List active network connections by application

Shows all active TCP/UDP connections grouped by application, including PIDs,
remote addresses, protocols, and connection states. Supports filtering and
continuous monitoring.

Examples:
  netctl connections                       List all active connections
  netctl connections --external            Show only external (non-local) connections
  netctl connections --app chrome          Filter connections by application name
  netctl connections --watch               Continuously monitor connections
  netctl connections --watch --interval 5  Monitor with a 5-second refresh")]
    Connections {
        /// Filter by application name
        #[arg(long)]
        app: Option<String>,

        /// Show only external (non-local) connections
        #[arg(long)]
        external: bool,

        /// Continuous monitoring mode
        #[arg(long)]
        watch: bool,

        /// Refresh interval in seconds (used with --watch)
        #[arg(long, default_value = "2")]
        interval: u64,
    },

    /// Real-time bandwidth usage per application
    #[command(long_about = "\
Real-time bandwidth usage per application

Displays per-application network bandwidth consumption (download and upload
rates). Uses nettop on macOS and ss/proc on Linux. Can alert when bandwidth
exceeds a threshold.

Examples:
  netctl bandwidth                     Show current bandwidth by app
  netctl bandwidth --top 5             Show only the top 5 consumers
  netctl bandwidth --app spotify       Monitor bandwidth for a specific app
  netctl bandwidth --watch             Continuously monitor (refreshes every 2s)
  netctl bandwidth --alert 50MB        Alert if total bandwidth exceeds 50 MB/s")]
    Bandwidth {
        /// Show top N bandwidth consumers
        #[arg(long)]
        top: Option<usize>,

        /// Monitor a specific application
        #[arg(long)]
        app: Option<String>,

        /// Alert threshold (e.g. "10MB")
        #[arg(long)]
        alert: Option<String>,

        /// Continuous monitoring mode
        #[arg(long)]
        watch: bool,
    },

    /// Connection quality test (ping with statistics)
    #[command(long_about = "\
Connection quality test (ping with statistics)

Sends ICMP ping packets to one or more hosts and reports detailed latency
statistics including min/avg/max, standard deviation, jitter, and packet
loss. Falls back to TCP-based pings when ICMP is unavailable.

Examples:
  netctl ping                              Ping google.com (default)
  netctl ping 8.8.8.8                      Ping a specific IP address
  netctl ping cloudflare.com --count 20    Send 20 ping packets
  netctl ping --hosts 1.1.1.1,8.8.8.8     Ping multiple hosts at once
  netctl ping github.com --count 50        Extended ping for stability test")]
    Ping {
        /// Host to ping
        host: Option<String>,

        /// Number of ping packets to send
        #[arg(long, default_value = "10")]
        count: u32,

        /// Ping multiple hosts (comma-separated)
        #[arg(long)]
        hosts: Option<String>,
    },

    /// Domain blocker / focus mode
    #[command(long_about = "\
Domain blocker / focus mode

Blocks distracting domains by adding entries to /etc/hosts. Supports
temporary blocks with auto-expiry, enabling/disabling all blocks at once,
and persists state across reboots. Requires sudo to modify /etc/hosts.

Examples:
  netctl block --list                              List all blocked domains
  netctl block --add twitter.com,reddit.com        Block multiple domains
  netctl block --add youtube.com --duration 2h     Block for 2 hours only
  netctl block --remove twitter.com                Unblock a domain
  netctl block --disable                           Disable all blocks temporarily")]
    Block {
        /// Add domains to block (comma-separated)
        #[arg(long)]
        add: Option<String>,

        /// Remove a domain from the block list
        #[arg(long)]
        remove: Option<String>,

        /// List all blocked domains
        #[arg(long)]
        list: bool,

        /// Enable all domain blocks
        #[arg(long)]
        enable: bool,

        /// Disable all domain blocks
        #[arg(long)]
        disable: bool,

        /// Duration for temporary blocks (e.g. "2h", "30m")
        #[arg(long)]
        duration: Option<String>,
    },

    /// VPN connection status
    #[command(long_about = "\
VPN connection status

Detects active VPN tunnels by inspecting network interfaces (tun, utun, tap,
ppp, wg, ipsec). Shows connection details including protocol, IP addresses,
DNS servers, and traffic statistics. Supports WireGuard detection.

Examples:
  netctl vpn status                    Check if a VPN is connected
  netctl vpn status --detailed         Show traffic stats and full details
  netctl vpn watch                     Continuously monitor VPN status")]
    Vpn {
        #[command(subcommand)]
        action: VpnAction,
    },

    /// DNS diagnostics and benchmarking
    #[command(long_about = "\
DNS diagnostics and benchmarking

Tools for inspecting and optimizing DNS resolution. Resolve domains, view
configured DNS servers, flush the system DNS cache, or benchmark popular
public resolvers to find the fastest one for your network.

Examples:
  netctl dns resolve github.com        Resolve a domain to IP addresses
  netctl dns servers                   Show currently configured DNS servers
  netctl dns flush                     Flush the system DNS cache
  netctl dns benchmark                 Benchmark Cloudflare, Google, Quad9, etc.")]
    Dns {
        #[command(subcommand)]
        action: DnsAction,
    },
}

#[derive(Subcommand)]
enum VpnAction {
    /// Show VPN connection status
    Status {
        /// Show detailed VPN information
        #[arg(long)]
        detailed: bool,
    },
    /// Monitor VPN connection continuously
    Watch,
}

#[derive(Subcommand)]
enum DnsAction {
    /// Resolve a domain name
    Resolve {
        /// Domain to resolve
        domain: String,
    },
    /// Flush DNS cache
    Flush,
    /// Show current DNS servers
    Servers,
    /// Benchmark DNS resolver performance
    Benchmark,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Speed {
            server,
            detailed,
            output,
        } => speed::run(server, detailed, output).await,

        Commands::Connections {
            app,
            external,
            watch,
            interval,
        } => connections::run(app, external, watch, interval).await,

        Commands::Bandwidth {
            top,
            app,
            alert,
            watch,
        } => bandwidth::run(top, app, alert, watch).await,

        Commands::Ping {
            host,
            count,
            hosts,
        } => ping::run(host, count, hosts).await,

        Commands::Block {
            add,
            remove,
            list,
            enable,
            disable,
            duration,
        } => block::run(add, remove, list, enable, disable, duration),

        Commands::Vpn { action } => match action {
            VpnAction::Status { detailed } => vpn::status(detailed).await,
            VpnAction::Watch => vpn::watch().await,
        },

        Commands::Dns { action } => match action {
            DnsAction::Resolve { domain } => dns::resolve(&domain).await,
            DnsAction::Flush => dns::flush().await,
            DnsAction::Servers => dns::servers().await,
            DnsAction::Benchmark => dns::benchmark().await,
        },
    };

    if let Err(e) = result {
        eprintln!("{}: {}", colored::Colorize::red("Error"), e);
        std::process::exit(1);
    }
}
