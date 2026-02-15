use colored::Colorize;
use std::fs;
use std::path::Path;

const HOSTS_PATH: &str = "/etc/hosts";
const BACKUP_PATH: &str = "/etc/hosts.netctl.bak";
const MARKER_BEGIN: &str = "# >>> netctl block begin";
const MARKER_END: &str = "# <<< netctl block end";
const BLOCK_STATE_PATH: &str = "/tmp/netctl_blocks.json";

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct BlockState {
    domains: Vec<BlockedDomain>,
    enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct BlockedDomain {
    domain: String,
    expires_at: Option<String>, // ISO 8601 timestamp
}

impl BlockState {
    fn load() -> Self {
        if let Ok(data) = fs::read_to_string(BLOCK_STATE_PATH) {
            serde_json::from_str(&data).unwrap_or(BlockState {
                domains: Vec::new(),
                enabled: false,
            })
        } else {
            BlockState {
                domains: Vec::new(),
                enabled: false,
            }
        }
    }

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(BLOCK_STATE_PATH, json)?;
        Ok(())
    }

    fn remove_expired(&mut self) {
        let now = chrono::Local::now();
        self.domains.retain(|d| {
            if let Some(ref exp) = d.expires_at {
                if let Ok(exp_time) = chrono::DateTime::parse_from_rfc3339(exp) {
                    return exp_time > now;
                }
            }
            true // no expiry = keep
        });
    }
}

fn parse_duration(dur: &str) -> Option<chrono::Duration> {
    let dur = dur.trim().to_lowercase();
    if let Some(hours) = dur.strip_suffix('h') {
        hours.parse::<i64>().ok().map(chrono::Duration::hours)
    } else if let Some(mins) = dur.strip_suffix('m') {
        mins.parse::<i64>().ok().map(chrono::Duration::minutes)
    } else if let Some(secs) = dur.strip_suffix('s') {
        secs.parse::<i64>().ok().map(chrono::Duration::seconds)
    } else {
        // Default to hours
        dur.parse::<i64>().ok().map(chrono::Duration::hours)
    }
}

fn backup_hosts() -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(BACKUP_PATH).exists() {
        fs::copy(HOSTS_PATH, BACKUP_PATH)?;
    }
    Ok(())
}

fn apply_blocks(state: &BlockState) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(HOSTS_PATH)?;

    // Remove existing netctl block section
    let mut new_content = String::new();
    let mut inside_block = false;
    for line in content.lines() {
        if line.trim() == MARKER_BEGIN {
            inside_block = true;
            continue;
        }
        if line.trim() == MARKER_END {
            inside_block = false;
            continue;
        }
        if !inside_block {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }

    // Add new block section if enabled and has domains
    if state.enabled && !state.domains.is_empty() {
        new_content.push_str(MARKER_BEGIN);
        new_content.push('\n');
        for domain in &state.domains {
            new_content.push_str(&format!("127.0.0.1 {}\n", domain.domain));
            new_content.push_str(&format!("127.0.0.1 www.{}\n", domain.domain));
        }
        new_content.push_str(MARKER_END);
        new_content.push('\n');
    }

    fs::write(HOSTS_PATH, new_content)?;
    Ok(())
}

fn print_status(state: &BlockState) {
    println!();
    println!("{}", "Domain Blocker".bold());
    println!();

    if state.domains.is_empty() {
        println!("  No domains are currently blocked.");
    } else {
        println!("Currently blocked:");
        let now = chrono::Local::now();
        for domain in &state.domains {
            if let Some(ref exp) = domain.expires_at {
                if let Ok(exp_time) = chrono::DateTime::parse_from_rfc3339(exp) {
                    let remaining = exp_time.signed_duration_since(now);
                    let hours = remaining.num_hours();
                    let mins = remaining.num_minutes() % 60;
                    println!(
                        "  {} {} (expires in {}h {}m)",
                        "T".yellow(),
                        domain.domain,
                        hours,
                        mins
                    );
                    continue;
                }
            }
            println!("  {} {}", "V".green(), domain.domain);
        }
    }

    println!();
    if state.enabled {
        println!("Status: {} Active", "OK".green());
    } else {
        println!("Status: {} Inactive", "--".red());
    }
    println!();
    println!(
        "{}",
        "Blocks are implemented via /etc/hosts".dimmed()
    );
    println!("{}", "Requires sudo to enable/disable".dimmed());
    println!();
}

pub fn run(
    add: Option<String>,
    remove: Option<String>,
    list: bool,
    enable: bool,
    disable: bool,
    duration: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = BlockState::load();
    state.remove_expired();

    let mut modified = false;

    if let Some(domains) = add {
        let expiry = duration.as_deref().and_then(|d| {
            parse_duration(d).map(|dur| (chrono::Local::now() + dur).to_rfc3339())
        });

        for domain in domains.split(',') {
            let domain = domain.trim().to_string();
            if domain.is_empty() {
                continue;
            }
            // Don't add duplicates
            if state.domains.iter().any(|d| d.domain == domain) {
                println!("  {} is already blocked", domain);
                continue;
            }
            println!("  Adding block for {}", domain.cyan());
            state.domains.push(BlockedDomain {
                domain,
                expires_at: expiry.clone(),
            });
        }
        state.enabled = true;
        modified = true;
    }

    if let Some(domain) = remove {
        let domain = domain.trim().to_string();
        let before = state.domains.len();
        state.domains.retain(|d| d.domain != domain);
        if state.domains.len() < before {
            println!("  Removed block for {}", domain.cyan());
            modified = true;
        } else {
            println!("  {} was not in the block list", domain);
        }
    }

    if enable {
        state.enabled = true;
        modified = true;
        println!("  Domain blocking {}", "enabled".green());
    }

    if disable {
        state.enabled = false;
        modified = true;
        println!("  Domain blocking {}", "disabled".red());
    }

    if modified {
        state.save()?;
        // Try to apply blocks (requires sudo/root)
        if let Err(e) = backup_hosts().and_then(|_| apply_blocks(&state)) {
            println!();
            println!(
                "  {}: Could not update /etc/hosts: {}",
                "Warning".yellow(),
                e
            );
            println!("  Run with {} for /etc/hosts modification", "sudo".bold());
            println!("  Block state saved to {} for later application", BLOCK_STATE_PATH);
        }
    }

    if list || (!modified && !enable && !disable) {
        print_status(&state);
    }

    Ok(())
}
