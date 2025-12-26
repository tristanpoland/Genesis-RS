//! Time and duration utilities.

use chrono::{DateTime, Utc, Local, Duration};

/// Format a duration in human-readable form.
pub fn pretty_duration(duration: Duration) -> String {
    let secs = duration.num_seconds();

    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        format!("{}h {}m", hours, mins)
    }
}

/// Format timestamp in fuzzy relative time.
pub fn fuzzy_time(timestamp: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(timestamp);

    if diff.num_seconds() < 60 {
        "just now".to_string()
    } else if diff.num_minutes() < 60 {
        format!("{} minutes ago", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{} hours ago", diff.num_hours())
    } else {
        format!("{} days ago", diff.num_days())
    }
}

/// Convert UTC timestamp to local time.
pub fn to_local(timestamp: DateTime<Utc>) -> DateTime<Local> {
    timestamp.with_timezone(&Local)
}

/// Parse duration from string (e.g., "1h30m", "90s", "2d").
pub fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if s.ends_with('s') {
        s[..s.len()-1].parse::<i64>().ok().map(Duration::seconds)
    } else if s.ends_with('m') {
        s[..s.len()-1].parse::<i64>().ok().map(Duration::minutes)
    } else if s.ends_with('h') {
        s[..s.len()-1].parse::<i64>().ok().map(Duration::hours)
    } else if s.ends_with('d') {
        s[..s.len()-1].parse::<i64>().ok().map(Duration::days)
    } else {
        s.parse::<i64>().ok().map(Duration::seconds)
    }
}

/// Measure execution time of a function.
pub fn measure<F, R>(f: F) -> (R, Duration)
where
    F: FnOnce() -> R,
{
    let start = Utc::now();
    let result = f();
    let duration = Utc::now().signed_duration_since(start);
    (result, duration)
}

// Note: Chrono provides comprehensive timezone support via chrono-tz crate
