use crate::mail::{Mailer, Envelope};

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
