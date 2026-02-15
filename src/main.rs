mod bandwidth;
mod block;
mod connections;
mod display;
mod dns;
mod ping;
mod speed;
mod vpn;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "netctl", version, about = "Network monitoring and management CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Network speed test (download/upload)
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
    Vpn {
        #[command(subcommand)]
        action: VpnAction,
    },

    /// DNS diagnostics and benchmarking
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
