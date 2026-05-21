//! Mail implementations — LogTransport, ArrayTransport, Mail facade.

use std::fmt;

#[derive(Debug, Clone)]
pub struct Attachment {
    pub filename: String, pub content: Vec<u8>, pub mime_type: String,
}
impl Attachment {
    pub fn new(filename: impl Into<String>, content: Vec<u8>, mime: impl Into<String>) -> Self {
        Attachment { filename: filename.into(), content, mime_type: mime.into() }
    }
}

#[derive(Debug, Clone)]
pub struct Envelope {
    pub from: Option<String>, pub to: Vec<String>, pub cc: Vec<String>, pub bcc: Vec<String>,
    pub subject: String, pub html_body: Option<String>, pub text_body: Option<String>,
    pub attachments: Vec<Attachment>,
}

pub trait Mailer: fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn send(&self, envelope: &Envelope) -> Result<(), String>;
    fn is_usable(&self) -> bool { true }
}

/// Log transport — writes emails to the log instead of sending.
/// Useful for development and testing.
#[derive(Debug)]
pub struct LogTransport;

impl Mailer for LogTransport {
    fn name(&self) -> &str { "log" }
    fn send(&self, envelope: &Envelope) -> Result<(), String> {
        println!("[mail] To: {}", envelope.to.join(", "));
        println!("[mail] Subject: {}", envelope.subject);
        if let Some(html) = &envelope.html_body {
            println!("[mail] HTML: {} bytes", html.len());
        }
        if let Some(text) = &envelope.text_body {
            println!("[mail] Text: {}", text);
        }
        Ok(())
    }
}

/// Array transport — collects sent emails in memory for assertion/testing.
#[derive(Debug)]
pub struct ArrayTransport {
    emails: std::sync::Mutex<Vec<Envelope>>,
}

impl ArrayTransport {
    pub fn new() -> Self { ArrayTransport { emails: std::sync::Mutex::new(Vec::new()) } }
    pub fn sent_emails(&self) -> Vec<Envelope> {
        self.emails.lock().map(|e| e.clone()).unwrap_or_default()
    }
    pub fn clear(&self) { if let Ok(mut e) = self.emails.lock() { e.clear(); } }
}

impl Default for ArrayTransport { fn default() -> Self { Self::new() } }

impl Mailer for ArrayTransport {
    fn name(&self) -> &str { "array" }
    fn send(&self, envelope: &Envelope) -> Result<(), String> {
        if let Ok(mut e) = self.emails.lock() { e.push(envelope.clone()); }
        Ok(())
    }
}

/// Mail facade
#[derive(Debug)]
pub struct Mail {
    mailer: Box<dyn Mailer>,
}

impl Mail {
    pub fn new(mailer: impl Mailer + 'static) -> Self { Mail { mailer: Box::new(mailer) } }
    pub fn mailer(&self) -> &dyn Mailer { self.mailer.as_ref() }
    pub fn send(&self, envelope: &Envelope) -> Result<(), String> { self.mailer.send(envelope) }
}
