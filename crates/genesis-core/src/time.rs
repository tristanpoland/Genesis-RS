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

// TODO: Implement:
// - Timezone handling
// - Custom format strings
// - Duration parsing
// - Execution timing
