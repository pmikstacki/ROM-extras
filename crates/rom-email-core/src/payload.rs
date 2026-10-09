use crate::{EmailError, address};
use rom::{Input, Value};
use std::fmt;
/// Immutable single-recipient plain-text intent; its persisted fields contain personal data.
/// Construction and public Input decoding both validate the same bounded profile.
#[derive(Clone)]
pub struct EmailNotification {
    pub(crate) to: String,
    pub(crate) subject: String,
    pub(crate) text: String,
    pub(crate) created_unix_seconds: u64,
}
impl EmailNotification {
    /// Validate one exact ASCII mailbox, UTF-8 subject/text and immutable Unix creation time.
    /// Subject is 1–512 bytes; text is at most 262144 bytes. Time is 0–4102444800 seconds.
    pub fn new(
        to: &str,
        subject: &str,
        text: &str,
        created_unix_seconds: u64,
    ) -> Result<Self, EmailError> {
        if subject.len() > 512 || text.len() > 262144 {
            return Err(EmailError::TooLarge);
        }
        address::mailbox(to)?;
        if subject.is_empty()
            || subject.chars().any(char::is_control)
            || created_unix_seconds > 4102444800
        {
            return Err(EmailError::InvalidPayload);
        }
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\r' {
                if chars.next() != Some('\n') {
                    return Err(EmailError::InvalidPayload);
                }
            } else if c.is_control() && !matches!(c, '\n' | '\t') {
                return Err(EmailError::InvalidPayload);
            }
        }
        Ok(Self {
            to: to.into(),
            subject: subject.into(),
            text: text.into(),
            created_unix_seconds,
        })
    }
}
impl Input for EmailNotification {
    fn encode(&self) -> Value {
        serde_json::json!({"to":self.to,"subject":self.subject,"text":self.text,"created_unix_seconds":self.created_unix_seconds})
    }
    fn decode(v: Value) -> rom::Result<Self> {
        let invalid = || rom::Error::invalid("email", "payload");
        let o = v.as_object().ok_or_else(invalid)?;
        if o.len() != 4 {
            return Err(invalid());
        }
        let to = o.get("to").and_then(Value::as_str).ok_or_else(invalid)?;
        let subject = o
            .get("subject")
            .and_then(Value::as_str)
            .ok_or_else(invalid)?;
        let text = o.get("text").and_then(Value::as_str).ok_or_else(invalid)?;
        let time = o
            .get("created_unix_seconds")
            .and_then(Value::as_u64)
            .ok_or_else(invalid)?;
        Self::new(to, subject, text, time).map_err(|e| {
            if e == EmailError::TooLarge {
                rom::Error::TooLarge
            } else {
                invalid()
            }
        })
    }
}
impl fmt::Debug for EmailNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmailNotification").finish_non_exhaustive()
    }
}
