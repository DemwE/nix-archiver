//! Formatting utilities for display

use chrono::{DateTime, Utc};
use std::time::Duration;

/// Formats a number with thousand separators
pub(crate) fn format_number(n: usize) -> String {
    let s = n.to_string();
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();
    
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }
    
    result
}

/// Formats a duration in human-readable format
pub(crate) fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    
    if secs < 60 {
        format!("{:.1}s", d.as_secs_f64())
    } else if secs < 3600 {
        let mins = secs / 60;
        let secs = secs % 60;
        format!("{}m {}s", mins, secs)
    } else {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        let secs = secs % 60;
        format!("{}h {}m {}s", hours, mins, secs)
    }
}

/// Formats Unix timestamp to human-readable date
pub(crate) fn format_unix_timestamp(timestamp: u64) -> String {
    DateTime::<Utc>::from_timestamp(timestamp as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
