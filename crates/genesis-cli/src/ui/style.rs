use colored::Colorize;

/// Format a title line for progress sections.
pub fn section(title: &str) -> String {
    format!("{} {}", "==>".bright_blue().bold(), title.bold())
}

/// Format informational text.
pub fn info(message: &str) -> String {
    format!("{} {}", "[i]".cyan().bold(), message)
}

/// Format success text.
pub fn success(message: &str) -> String {
    format!("{} {}", "✓".green().bold(), message)
}

/// Format warning text.
pub fn warning(message: &str) -> String {
    format!("{} {}", "!".yellow().bold(), message)
}

/// Format error text (not panic, just stylized text).
pub fn error(message: &str) -> String {
    format!("{} {}", "✗".red().bold(), message)
}
