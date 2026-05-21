//! Styling utilities for terminal output.
//!
//! Wraps ANSI escape codes for common text styling.
//! Currently these are used internally by `Output`. This module
//! provides direct access for custom formatting.

/// Wrap text in ANSI style codes.
pub fn style(text: &str, code: &str) -> String {
    format!("\x1b[{}m{}\x1b[0m", code, text)
}

/// Bold text.
pub fn bold(text: &str) -> String {
    style(text, "1")
}

/// Dim / muted text.
pub fn dim(text: &str) -> String {
    style(text, "2")
}

/// Italic text.
pub fn italic(text: &str) -> String {
    style(text, "3")
}

/// Underlined text.
pub fn underline(text: &str) -> String {
    style(text, "4")
}

// ── Foreground colors ──

pub fn red(text: &str) -> String {
    style(text, "31")
}

pub fn green(text: &str) -> String {
    style(text, "32")
}

pub fn yellow(text: &str) -> String {
    style(text, "33")
}

pub fn blue(text: &str) -> String {
    style(text, "34")
}

pub fn magenta(text: &str) -> String {
    style(text, "35")
}

pub fn cyan(text: &str) -> String {
    style(text, "36")
}

pub fn white(text: &str) -> String {
    style(text, "37")
}

pub fn grey(text: &str) -> String {
    style(text, "90")
}

// ── Background colors ──

pub fn bg_red(text: &str) -> String {
    style(text, "41")
}

pub fn bg_green(text: &str) -> String {
    style(text, "42")
}

pub fn bg_blue(text: &str) -> String {
    style(text, "44")
}

pub fn bg_cyan(text: &str) -> String {
    style(text, "46")
}
