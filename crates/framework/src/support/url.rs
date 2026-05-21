//! URL encoding/decoding helpers.

/// Decode a URL-encoded string.
pub fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();

    while let Some(c) = chars.next() {
        match c {
            '+' => result.push(' '),
            '%' => {
                let hi = chars.next().and_then(|c| c.to_digit(16));
                let lo = chars.next().and_then(|c| c.to_digit(16));
                match (hi, lo) {
                    (Some(h), Some(l)) => {
                        let byte = (h << 4 | l) as u8;
                        result.push(byte as char);
                    }
                    _ => result.push('%'),
                }
            }
            _ => result.push(c),
        }
    }
    result
}

/// Encode a string for URL usage.
pub fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push_str("%20"),
            _ => result.push_str(&format!("%{:02X}", byte)),
        }
    }
    result
}

/// Parse query string into key-value pairs.
pub fn parse_query(s: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for part in s.split('&') {
        if part.is_empty() { continue; }
        if let Some(eq) = part.find('=') {
            let key = url_decode(&part[..eq]);
            let val = url_decode(&part[eq + 1..]);
            pairs.push((key, val));
        } else {
            pairs.push((url_decode(part), String::new()));
        }
    }
    pairs
}

/// Build a query string from key-value pairs.
pub fn build_query(pairs: &[(&str, &str)]) -> String {
    let mut parts = Vec::new();
    for (k, v) in pairs {
        parts.push(format!("{}={}", url_encode(k), url_encode(v)));
    }
    parts.join("&")
}

/// Check if a string is a valid URL.
pub fn is_valid_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}
