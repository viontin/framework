//! File upload handling — multipart form data parser.
//!
//! Parses `multipart/form-data` request bodies into fields and file uploads.

use std::collections::HashMap;

/// A single uploaded file from a multipart request.
#[derive(Debug, Clone)]
pub struct UploadedFile {
    pub name: String,
    pub filename: String,
    pub content_type: Option<String>,
    pub data: Vec<u8>,
    pub size: usize,
}

impl UploadedFile {
    pub fn extension(&self) -> Option<&str> {
        self.filename.rfind('.').map(|i| &self.filename[i + 1..])
    }

    pub fn mime_type(&self) -> &str {
        self.content_type.as_deref().unwrap_or("application/octet-stream")
    }

    pub fn store(&self, path: &str) -> Result<(), String> {
        let dir = std::path::Path::new(path).parent().unwrap_or(std::path::Path::new("."));
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        std::fs::write(path, &self.data).map_err(|e| e.to_string())
    }
}

/// Parse a multipart/form-data body into fields and uploaded files.
pub fn parse_multipart(body: &[u8], boundary: &str) -> Result<MultipartData, String> {
    let full_boundary = format!("--{}", boundary);
    let end_boundary = format!("--{}--", boundary);
    let body_str = String::from_utf8_lossy(body);

    let mut fields = HashMap::new();
    let mut files = HashMap::new();

    let parts: Vec<&str> = body_str.split(&full_boundary).collect();
    for part in &parts {
        let part = part.trim();
        if part.is_empty() || part == "--" {
            continue;
        }
        if part.starts_with("--") && part.ends_with("--") {
            continue;
        }

        let (headers, content) = match part.find("\r\n\r\n") {
            Some(pos) => (&part[..pos], part[pos + 4..].trim_end_matches('\r').trim_end_matches('\n')),
            None => match part.find("\n\n") {
                Some(pos) => (&part[..pos], part[pos + 2..].trim_end_matches('\r').trim_end_matches('\n')),
                None => continue,
            },
        };

        let mut content_disposition = "";
        let mut content_type: Option<String> = None;
        for line in headers.lines() {
            let line = line.trim();
            if line.to_lowercase().starts_with("content-disposition:") {
                content_disposition = line["content-disposition:".len()..].trim();
            } else if line.to_lowercase().starts_with("content-type:") {
                content_type = Some(line["content-type:".len()..].trim().to_string());
            }
        }

        let mut name = String::new();
        let mut filename = String::new();
        for param in content_disposition.split(';') {
            let param = param.trim();
            if param.starts_with("name=") {
                name = unquote(&param[5..]);
            } else if param.starts_with("filename=") {
                filename = unquote(&param[9..]);
            }
        }

        if name.is_empty() {
            continue;
        }

        if !filename.is_empty() {
            let data_start = find_content_start(body, part, &full_boundary);
            let data_end = find_content_end(body, part, &full_boundary, &end_boundary);
            let file_data = body[data_start..data_end].to_vec();

            files.insert(
                name.to_string(),
                UploadedFile {
                    name: name.to_string(),
                    filename: filename.to_string(),
                    content_type,
                    size: file_data.len(),
                    data: file_data,
                },
            );
        } else {
            fields.insert(name.to_string(), content.to_string());
        }
    }

    Ok(MultipartData { fields, files })
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn find_content_start(body: &[u8], part: &str, _boundary: &str) -> usize {
    if let Some(pos) = String::from_utf8_lossy(body).find(part) {
        let after_headers = part.find("\r\n\r\n").map(|p| p + 4).unwrap_or_else(|| part.find("\n\n").map(|p| p + 2).unwrap_or(0));
        pos + after_headers
    } else {
        0
    }
}

fn find_content_end(body: &[u8], part: &str, boundary: &str, _end_boundary: &str) -> usize {
    let body_str = String::from_utf8_lossy(body);
    if let Some(part_start) = body_str.find(part) {
        let part_end = part_start + part.len();
        let rest = &body_str[part_end..];
        if let Some(next_boundary) = rest.find(boundary) {
            return (part_end + next_boundary).min(body.len());
        }
    }
    body.len()
}

/// Result of parsing a multipart form body.
#[derive(Debug, Clone)]
pub struct MultipartData {
    pub fields: HashMap<String, String>,
    pub files: HashMap<String, UploadedFile>,
}

/// Check if a content-type header indicates multipart/form-data.
pub fn is_multipart(content_type: Option<&str>) -> bool {
    content_type
        .map(|c| c.to_lowercase().starts_with("multipart/form-data"))
        .unwrap_or(false)
}

/// Extract the boundary from a content-type header.
pub fn extract_boundary(content_type: &str) -> Option<String> {
    for part in content_type.split(';') {
        let part = part.trim();
        if part.to_lowercase().starts_with("boundary=") {
            return Some(unquote(&part[9..]));
        }
    }
    None
}
