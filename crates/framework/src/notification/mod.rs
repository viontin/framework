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

// ── Webhook Channels (Slack, Discord, Telegram) ──

/// Generic webhook channel for Slack, Discord, Telegram notifications.
/// POSTs a JSON payload to a webhook URL.
#[derive(Debug)]
pub struct WebhookChannel {
    name: &'static str,
    url: String,
}

impl WebhookChannel {
    pub fn new(name: &'static str, url: impl Into<String>) -> Self {
        WebhookChannel { name, url: url.into() }
    }

    /// Slack webhook — `https://hooks.slack.com/services/...`
    pub fn slack(url: impl Into<String>) -> Self {
        WebhookChannel { name: "slack", url: url.into() }
    }

    /// Discord webhook — `https://discord.com/api/webhooks/...`
    pub fn discord(url: impl Into<String>) -> Self {
        WebhookChannel { name: "discord", url: url.into() }
    }

    /// Telegram bot — POSTs to `https://api.telegram.org/bot<token>/sendMessage`
    pub fn telegram(token: impl Into<String>, chat_id: impl Into<String>) -> Self {
        WebhookChannel {
            name: "telegram",
            url: format!(
                "https://api.telegram.org/bot{}/sendMessage?chat_id={}",
                token.into(),
                chat_id.into()
            ),
        }
    }

    fn post_json(&self, body: &str) -> Result<(), String> {
        #[cfg(feature = "http-client")]
        {
            let resp = ureq::post(&self.url)
                .set("Content-Type", "application/json")
                .send_string(body)
                .map_err(|e| format!("Webhook request failed: {}", e))?;
            if resp.status() >= 400 {
                return Err(format!("Webhook returned {}", resp.status()));
            }
            return Ok(());
        }
        #[cfg(not(feature = "http-client"))]
        {
            let _ = (body, &self.url);
            Err("http-client feature not enabled — cannot send webhook".into())
        }
    }

    fn format_payload(&self, message: &str) -> String {
        match self.name {
            "slack" => format!(r#"{{"text":"{}"}}"#, message.replace('"', "\\\"")),
            "discord" => format!(r#"{{"content":"{}"}}"#, message.replace('"', "\\\"")),
            "telegram" => format!(r#"{{"text":"{}","parse_mode":"HTML"}}"#, message.replace('"', "\\\"")),
            _ => format!(r#"{{"message":"{}"}}"#, message.replace('"', "\\\"")),
        }
    }
}

impl Channel for WebhookChannel {
    fn name(&self) -> &str { self.name }
    fn send(&self, notifiable: &dyn Notifiable, notification: &dyn Notification) -> Result<(), String> {
        let msg = match self.name {
            "slack" => notification.to_slack(notifiable),
            "discord" | "telegram" => notification.to_database(notifiable),
            _ => None,
        };
        if let Some(body) = msg {
            let payload = self.format_payload(&body);
            self.post_json(&payload)
        } else {
            Ok(())
        }
    }
}
