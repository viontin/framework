use crate::mail::{Mailer, Envelope};

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
