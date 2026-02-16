/// Look up a process name by PID using the system `ps` command.
///
/// Returns "Unknown" if the PID is empty, "-", or cannot be resolved.
pub fn get_process_name(pid: &str) -> String {
    if pid.is_empty() || pid == "-" {
        return "Unknown".to_string();
    }
    let output = std::process::Command::new("ps")
        .args(["-p", pid, "-o", "comm="])
        .output();
    match output {
        Ok(out) => {
            let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if name.is_empty() {
                "Unknown".to_string()
            } else {
                // Extract just the binary name from the path
                name.rsplit('/').next().unwrap_or(&name).to_string()
            }
        }
        Err(_) => "Unknown".to_string(),
    }
}

/// Format bytes per second into a human-readable rate string.
pub fn format_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1_000_000_000.0 {
        format!("{:.1} GB/s", bytes_per_sec / 1_000_000_000.0)
    } else if bytes_per_sec >= 1_000_000.0 {
        format!("{:.1} MB/s", bytes_per_sec / 1_000_000.0)
    } else if bytes_per_sec >= 1_000.0 {
        format!("{:.0} KB/s", bytes_per_sec / 1_000.0)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}
