use crate::mail::{Mailer, Envelope};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{Address, Message, Transport};
use std::str::FromStr;

#[derive(Debug)]
pub struct SmtpTransport {
    creds: Credentials,
    relay: String,
}

impl SmtpTransport {
    pub fn new(relay: &str, username: &str, password: &str) -> Self {
        SmtpTransport {
            relay: relay.into(),
            creds: Credentials::new(username.into(), password.into()),
        }
    }
}

impl Mailer for SmtpTransport {
    fn name(&self) -> &str { "smtp" }

    fn send(&self, envelope: &Envelope) -> Result<(), String> {
        let from = envelope.from.as_deref().unwrap_or("noreply@viontin");

        let mut builder = Message::builder()
            .from(lettre::message::Mailbox::new(
                None,
                Address::from_str(from).map_err(|e| e.to_string())?,
            ));

        for to in &envelope.to {
            builder = builder.to(lettre::message::Mailbox::new(
                None,
                Address::from_str(to).map_err(|e| e.to_string())?,
            ));
        }
        for cc in &envelope.cc {
            builder = builder.cc(lettre::message::Mailbox::new(
                None,
                Address::from_str(cc).map_err(|e| e.to_string())?,
            ));
        }
        builder = builder.subject(&envelope.subject);

        let msg = match (&envelope.html_body, &envelope.text_body) {
            (Some(html), Some(text)) => builder
                .multipart(lettre::message::MultiPart::alternative()
                    .singlepart(lettre::message::SinglePart::plain(text.clone()))
                    .singlepart(lettre::message::SinglePart::html(html.clone()))),
            (Some(html), None) => builder
                .singlepart(lettre::message::SinglePart::html(html.clone())),
            (None, Some(text)) => builder
                .singlepart(lettre::message::SinglePart::plain(text.clone())),
            (None, None) => builder
                .singlepart(lettre::message::SinglePart::plain(String::new())),
        }
        .map_err(|e| e.to_string())?;

        let (host, port) = self.relay.split_once(':').unwrap_or((&self.relay, "587"));
        let port: u16 = port.parse().unwrap_or(587);
        let tls_params = TlsParameters::builder(host.to_string())
            .build_native()
            .map_err(|e| e.to_string())?;

        lettre::SmtpTransport::relay(host)
            .map_err(|e| e.to_string())?
            .port(port)
            .tls(Tls::Required(tls_params))
            .credentials(self.creds.clone())
            .build()
            .send(&msg)
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
