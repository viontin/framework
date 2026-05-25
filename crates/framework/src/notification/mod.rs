use std::collections::HashMap;
use std::fmt;
use crate::mail::Envelope;

pub trait Notifiable: fmt::Debug + Send + Sync {
    fn route_notification(&self, channel: &str) -> Option<String>;
}

pub trait Notification: fmt::Debug + Send + Sync {
    fn channels(&self) -> Vec<&'static str>;
    fn to_mail(&self, _notifiable: &dyn Notifiable) -> Option<String> { None }
    fn to_database(&self, _notifiable: &dyn Notifiable) -> Option<String> { None }
    fn to_slack(&self, _notifiable: &dyn Notifiable) -> Option<String> { None }
}

pub trait Channel: fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn send(&self, notifiable: &dyn Notifiable, notification: &dyn Notification) -> Result<(), String>;
}

#[derive(Debug)]
pub struct MailChannel { mailer: crate::mail::Mail, }

impl MailChannel {
    pub fn new(mailer: crate::mail::Mail) -> Self { MailChannel { mailer } }
}

impl Channel for MailChannel {
    fn name(&self) -> &str { "mail" }
    fn send(&self, notifiable: &dyn Notifiable, notification: &dyn Notification) -> Result<(), String> {
        if let Some(body) = notification.to_mail(notifiable) {
            let envelope = Envelope {
                from: None, to: vec![notifiable.route_notification("mail").unwrap_or_default()],
                cc: Vec::new(), bcc: Vec::new(), subject: "Notification".into(),
                html_body: Some(body), text_body: None, attachments: Vec::new(),
            };
            self.mailer.send(&envelope)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct DatabaseChannel { storage: std::sync::Mutex<Vec<(String, String)>>, }

impl DatabaseChannel {
    pub fn new() -> Self { DatabaseChannel { storage: std::sync::Mutex::new(Vec::new()) } }
    pub fn all(&self) -> Vec<(String, String)> { self.storage.lock().map(|s| s.clone()).unwrap_or_default() }
}
impl Default for DatabaseChannel { fn default() -> Self { Self::new() } }

impl Channel for DatabaseChannel {
    fn name(&self) -> &str { "database" }
    fn send(&self, notifiable: &dyn Notifiable, notification: &dyn Notification) -> Result<(), String> {
        if let Some(data) = notification.to_database(notifiable) {
            let route = notifiable.route_notification("database").unwrap_or_default();
            if let Ok(mut s) = self.storage.lock() { s.push((route, data)); }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Notif { channels: HashMap<&'static str, Box<dyn Channel>>, }

impl Notif {
    pub fn new() -> Self { Notif { channels: HashMap::new() } }
    pub fn add_channel(&mut self, name: &'static str, channel: Box<dyn Channel>) {
        self.channels.insert(name, channel);
    }
    pub fn send(&self, notifiable: &dyn Notifiable, notification: &dyn Notification) -> Result<(), String> {
        for ch_name in notification.channels() {
            if let Some(channel) = self.channels.get(ch_name) {
                channel.send(notifiable, notification)?;
            }
        }
        Ok(())
    }
}
impl Default for Notif { fn default() -> Self { Self::new() } }
