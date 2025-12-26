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
/// Supports codes like #R{red}, #G{green}, etc.
pub fn colorize(input: &str) -> String {
    // TODO: Implement full color code parsing
    input.to_string()
}

// TODO: Implement:
// - Color code parsing (#R{}, #G{}, etc.)
// - Word wrapping
// - Progress bars
// - Spinner animations
