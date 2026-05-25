//! Mail implementations — LogTransport, ArrayTransport, SmtpTransport, Mail facade.

pub mod log;
pub mod array;
#[cfg(feature = "smtp")]
pub mod smtp;

pub use log::LogTransport;
pub use array::ArrayTransport;
#[cfg(feature = "smtp")]
pub use smtp::SmtpTransport;

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

impl Default for Envelope {
    fn default() -> Self {
        Envelope {
            from: None, to: Vec::new(), cc: Vec::new(), bcc: Vec::new(),
            subject: String::new(), html_body: None, text_body: None,
            attachments: Vec::new(),
        }
    }
}

pub trait Mailer: fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn send(&self, envelope: &Envelope) -> Result<(), String>;
    fn is_usable(&self) -> bool { true }
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

    #[cfg(feature = "smtp")]
    pub fn smtp(relay: &str, username: &str, password: &str) -> Self {
        Mail::new(crate::mail::SmtpTransport::new(relay, username, password))
    }
}
