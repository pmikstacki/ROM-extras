use crate::{EmailError, EmailNotification, PreparedEmail, address};
use lettre::{
    Address, Message,
    message::{Mailbox, header::ContentType},
};
use rom::Delivery;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fmt,
    time::{Duration, SystemTime},
};
/// Immutable host-approved sender, exact recipient set, Message-ID domain and encoded byte limit.
/// Keep the same profile for a frozen Runtime delivery across attempts and recovery.
pub struct EmailProfile {
    sender: String,
    sender_address: Address,
    recipients: BTreeSet<String>,
    domain: String,
    wire_limit: usize,
}
impl EmailProfile {
    /// Approve 1–1024 unique exact recipients and a 1–1048576-byte final MIME limit.
    /// No DNS lookup, domain discovery, message submission or credential access occurs.
    pub fn new(
        sender: &str,
        recipients: &[&str],
        message_id_domain: &str,
        wire_limit: usize,
    ) -> Result<Self, EmailError> {
        if recipients.is_empty()
            || recipients.len() > 1024
            || !(1..=1048576).contains(&wire_limit)
            || !address::domain(message_id_domain)
        {
            return Err(EmailError::InvalidProfile);
        }
        let sender_address = address::mailbox(sender).map_err(|_| EmailError::InvalidProfile)?;
        let mut set = BTreeSet::new();
        for to in recipients {
            address::mailbox(to).map_err(|_| EmailError::InvalidProfile)?;
            if !set.insert((*to).to_string()) {
                return Err(EmailError::InvalidProfile);
            }
        }
        Ok(Self {
            sender: sender.into(),
            sender_address,
            recipients: set,
            domain: message_id_domain.into(),
            wire_limit,
        })
    }
    /// Prepare deterministic MIME bytes and envelope for one public Runtime delivery.
    /// The digest-based Message-ID is stable, but SMTP does not guarantee deduplication.
    pub fn prepare(
        &self,
        delivery: Delivery<EmailNotification>,
    ) -> Result<PreparedEmail, EmailError> {
        rom_delivery_core::validate_delivery_identity(&delivery.id)
            .map_err(|_| EmailError::InvalidIdentity)?;
        let p = delivery.payload;
        if !self.recipients.contains(&p.to) {
            return Err(EmailError::RecipientDenied);
        }
        let to = address::mailbox(&p.to)?;
        let id = format!(
            "<{:x}@{}>",
            Sha256::digest(delivery.id.as_bytes()),
            self.domain
        );
        let date = SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(p.created_unix_seconds))
            .ok_or(EmailError::InvalidPayload)?;
        let message = Message::builder()
            .from(Mailbox::new(None, self.sender_address.clone()))
            .to(Mailbox::new(None, to))
            .subject(p.subject)
            .date(date)
            .message_id(Some(id.clone()))
            .header(lettre::message::header::MIME_VERSION_1_0)
            .header(ContentType::TEXT_PLAIN)
            .body(p.text)
            .map_err(|_| EmailError::InvalidPayload)?;
        let body = message.formatted();
        if body.len() > self.wire_limit {
            return Err(EmailError::TooLarge);
        }
        Ok(PreparedEmail::new(self.sender.clone(), p.to, id, body))
    }
}
impl fmt::Debug for EmailProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmailProfile")
            .field("wire_limit", &self.wire_limit)
            .finish_non_exhaustive()
    }
}
