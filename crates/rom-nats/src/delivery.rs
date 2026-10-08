use crate::NatsDeliveryError;
use async_nats::jetstream::{Context, message::PublishMessage, stream::StorageType};
use rom::{Delivery, DeliveryOutcome, Input};
use rom_delivery_core::{PayloadLimit, PreparedDelivery};
use std::time::Duration;
use tokio::sync::Semaphore;

/// One existing file-backed JetStream stream and exact subject.
///
/// Host owns connection TLS/authentication, reconnect/buffer policy, stream
/// provisioning, replica topology, fsync, retention, and duplicate window.
/// Broker deduplication is time-limited. This does not guarantee exactly-once
/// downstream effects. Runtime owns durable intents, retry, and authorization.
pub struct JetStreamDelivery {
    context: Context,
    stream: String,
    subject: String,
    deadline: Duration,
    payload_limit: PayloadLimit,
    admission: Semaphore,
}
impl JetStreamDelivery {
    /// Bind one explicitly listed subject on a file stream with publish acks.
    ///
    /// Names accept ASCII alphanumeric, hyphen, and underscore; subject tokens
    /// are separated by dots. Wildcard/system subjects are outside this profile.
    /// Admission is 1..=1024; deadline is positive and at most 300 seconds.
    /// Binding checks current configuration; later broker changes remain the
    /// host's responsibility. No stream is created or modified by this adapter.
    pub async fn bind(
        mut context: Context,
        stream: &str,
        subject: &str,
        concurrency: usize,
        deadline: Duration,
        payload_limit: PayloadLimit,
    ) -> Result<Self, NatsDeliveryError> {
        let token = |text: &str| {
            !text.is_empty()
                && text
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        };
        if stream.len() > 128
            || !token(stream)
            || subject.len() > 256
            || !subject.split('.').all(token)
            || !(1..=1024).contains(&concurrency)
            || deadline.is_zero()
            || deadline > Duration::from_secs(300)
        {
            return Err(NatsDeliveryError::InvalidConfiguration);
        }
        context.set_timeout(deadline);
        let bound = tokio::time::timeout(deadline, context.get_stream(stream))
            .await
            .map_err(|_| NatsDeliveryError::Unavailable)?
            .map_err(|_| NatsDeliveryError::Unavailable)?;
        let config = &bound.cached_info().config;
        if config.storage != StorageType::File
            || config.no_ack
            || !config.subjects.iter().any(|listed| listed == subject)
        {
            return Err(NatsDeliveryError::UnsupportedStream);
        }
        Ok(Self {
            context,
            stream: stream.into(),
            subject: subject.into(),
            deadline,
            payload_limit,
            admission: Semaphore::new(concurrency),
        })
    }

    /// Publish exact prepared JSON and await the broker's stream acknowledgement.
    ///
    /// Overload returns Retryable before admission. Preparation rejection is
    /// Permanent. Publication/acknowledgement failures and deadline are Unknown:
    /// a client-buffered command may still publish later. Dropping this future
    /// does not establish rollback. No adapter message retries are performed.
    pub async fn deliver<P: Input>(&self, delivery: Delivery<P>) -> DeliveryOutcome {
        let Ok(_permit) = self.admission.try_acquire() else {
            return DeliveryOutcome::Retryable;
        };
        let Ok(message) = PreparedDelivery::prepare(delivery, self.payload_limit) else {
            return DeliveryOutcome::Permanent;
        };
        let publish = PublishMessage::build()
            .message_id(message.id())
            .expected_stream(&self.stream)
            .payload(message.into_body().into());
        let operation = async {
            let ack = self
                .context
                .send_publish(self.subject.clone(), publish)
                .await?;
            ack.await
        };
        match tokio::time::timeout(self.deadline, operation).await {
            Ok(Ok(ack)) if ack.stream == self.stream && ack.sequence > 0 => {
                DeliveryOutcome::Accepted
            }
            _ => DeliveryOutcome::Unknown,
        }
    }
}
