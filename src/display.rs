#![allow(dead_code)]
use colored::Colorize;
pub use crate::utils::format_rate;

/// Print a boxed header section.
pub fn print_header(title: &str) {
    let width = 57;
    let pad_total = width - 2 - title.len();
    let pad_left = pad_total / 2;
    let pad_right = pad_total - pad_left;

    println!(
        "{}",
        format!(
            "\u{256d}{}\u{256e}",
            "\u{2500}".repeat(width)
        )
        .cyan()
    );
    println!(
        "{}",
        format!(
            "\u{2502}{}{}{}\u{2502}",
            " ".repeat(pad_left),
            title,
            " ".repeat(pad_right)
        )
        .cyan()
    );
    println!(
        "{}",
        format!(
            "\u{251c}{}\u{2524}",
            "\u{2500}".repeat(width)
        )
        .cyan()
    );
}

/// Print a row inside a box.
pub fn print_row(label: &str, value: &str) {
    let width = 57;
    let content = format!(" {:<17}{}", label, value);
    let pad = width - content.len();
    println!(
        "{}",
        format!(
            "\u{2502}{}{}\u{2502}",
            content,
            " ".repeat(pad)
        )
        .cyan()
    );
}

/// Print an empty row inside a box.
pub fn print_empty_row() {
    let width = 57;
    println!(
        "{}",
        format!(
            "\u{2502}{}\u{2502}",
            " ".repeat(width)
        )
        .cyan()
    );
}

/// Print the bottom border of a box.
pub fn print_footer() {
    let width = 57;
    println!(
        "{}",
        format!(
            "\u{2570}{}\u{256f}",
            "\u{2500}".repeat(width)
        )
        .cyan()
    );
}


/// Format bytes into a human-readable size string.
pub fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.0} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Format megabits per second.
pub fn format_mbps(mbps: f64) -> String {
    if mbps >= 1000.0 {
        format!("{:.1} Gbps", mbps / 1000.0)
    } else {
        format!("{:.1} Mbps", mbps)
    }
}

/// Assess connection quality based on latency in ms.
pub fn quality_assessment(latency_ms: f64) -> (&'static str, String) {
    if latency_ms < 30.0 {
        (
            "Excellent",
            "Excellent for video calls & streaming"
                .green()
                .to_string(),
        )
    } else if latency_ms < 60.0 {
        ("Good", "Good for most applications".green().to_string())
    } else if latency_ms < 100.0 {
        (
            "Fair",
            "Acceptable, may affect real-time apps"
                .yellow()
                .to_string(),
        )
    } else {
        (
            "Poor",
            "High latency, expect delays".red().to_string(),
        )
    }
}

/// Print a simple progress bar.
pub fn print_progress_bar(fraction: f64, width: usize) -> String {
    let filled = (fraction * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!(
        "[{}{}] {:.0}%",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
        fraction * 100.0
    )
}
