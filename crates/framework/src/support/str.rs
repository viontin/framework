//! String manipulation utilities.

/// Truncate a string to a maximum length, adding "..." if truncated.
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut result = s[..max_len].to_string();
        result.push_str("...");
        result
    }
}

/// Convert a string to kebab-case.
pub fn kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            result.push('-');
        }
        result.push(c.to_ascii_lowercase());
    }
    result.replace("_", "-").replace(" ", "-")
}

/// Convert a string to snake_case.
pub fn snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
    }
    result.replace("-", "_").replace(" ", "_")
}

/// Convert a string to PascalCase.
pub fn pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize = true;
    for c in s.chars() {
        if c == '_' || c == '-' || c == ' ' {
            capitalize = true;
        } else if capitalize {
            result.push(c.to_ascii_uppercase());
            capitalize = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert a string to camelCase.
pub fn camel_case(s: &str) -> String {
    let pascal = pascal_case(s);
    let mut result = pascal;
    if let Some(c) = result.get_mut(..1) {
        c.make_ascii_lowercase();
    }
    result
}

/// Generate a URL-friendly slug.
pub fn slug(s: &str) -> String {
    let mut result = String::new();
    for c in s.to_lowercase().chars() {
        match c {
            'a'..='z' | '0'..='9' => result.push(c),
            ' ' | '_' | '-' => {
                if !result.ends_with('-') { result.push('-'); }
            }
            _ => {} // strip other chars
        }
    }
    result.trim_matches('-').to_string()
}

/// Check if a string starts with a given prefix (case-insensitive).
pub fn starts_with_ignore_case(s: &str, prefix: &str) -> bool {
    s.to_lowercase().starts_with(&prefix.to_lowercase())
}

/// Repeat a string n times.
pub fn repeat(s: &str, n: usize) -> String {
    s.repeat(n)
}

/// Pad a string to the left with a character.
pub fn pad_left(s: &str, width: usize, pad: char) -> String {
    if s.len() >= width { s.to_string() }
    else { format!("{}{}", repeat(&pad.to_string(), width - s.len()), s) }
}

/// Pad a string to the right with a character.
pub fn pad_right(s: &str, width: usize, pad: char) -> String {
    if s.len() >= width { s.to_string() }
    else { format!("{}{}", s, repeat(&pad.to_string(), width - s.len())) }
}

/// Random string of given length (alphanumeric).
pub fn random(length: usize) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let mut state = seed as u64;
    let mut result = String::with_capacity(length);
    for _ in 0..length {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let idx = (state >> 33) as usize % chars.len();
        result.push(chars[idx]);
    }
    result
}

/// Pluralize a word (simple English rules).
pub fn pluralize(s: &str) -> String {
    if s.ends_with('s') || s.ends_with('x') || s.ends_with('z') {
        format!("{}es", s)
    } else if s.ends_with('y') && s.len() > 1 {
        let pre = &s[..s.len() - 1];
        match pre.chars().last() {
            Some(c) if "aeiou".contains(c) => format!("{}s", s),
            _ => format!("{}ies", pre),
        }
    } else if s.ends_with("ch") || s.ends_with("sh") {
        format!("{}es", s)
    } else {
        format!("{}s", s)
    }
}
