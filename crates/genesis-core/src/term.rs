//! Terminal utilities for colored output and formatting.

use colored::*;
use std::io::{self, IsTerminal};

/// Check if we're running in a controlling terminal.
pub fn in_controlling_terminal() -> bool {
    io::stdout().is_terminal()
}

/// Get terminal width in columns.
pub fn terminal_width() -> usize {
    term_size::dimensions().map(|(w, _)| w).unwrap_or(80)
}

/// Format a string with Genesis color codes.
///
/// Supports codes like #R{text}, #G{text}, #Y{text}, #B{text}, #M{text}, #C{text}, #W{text}, #K{text}
pub fn colorize(input: &str) -> String {
    let mut result = input.to_string();

    let color_codes = [
        ("#R{", "}"),
        ("#G{", "}"),
        ("#Y{", "}"),
        ("#B{", "}"),
        ("#M{", "}"),
        ("#C{", "}"),
        ("#W{", "}"),
        ("#K{", "}"),
    ];

    for (start, end) in color_codes {
        while let Some(start_pos) = result.find(start) {
            if let Some(end_pos) = result[start_pos..].find(end) {
                let end_pos = start_pos + end_pos;
                let text = &result[start_pos + start.len()..end_pos];

                // Apply color based on the code
                let colored = match start {
                    "#R{" => text.red().to_string(),
                    "#G{" => text.green().to_string(),
                    "#Y{" => text.yellow().to_string(),
                    "#B{" => text.blue().to_string(),
                    "#M{" => text.magenta().to_string(),
                    "#C{" => text.cyan().to_string(),
                    "#W{" => text.white().to_string(),
                    "#K{" => text.black().to_string(),
                    _ => text.to_string(),
                };

                result.replace_range(start_pos..=end_pos, &colored);
            } else {
                break;
            }
        }
    }

    result
}

/// Wrap text to fit terminal width.
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 > width {
            if !current_line.is_empty() {
                lines.push(current_line.clone());
                current_line.clear();
            }
        }

        if !current_line.is_empty() {
            current_line.push(' ');
        }
        current_line.push_str(word);
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

// Note: Progress bars and spinners are implemented in genesis-cli/src/ui/progress.rs
// using the indicatif crate
