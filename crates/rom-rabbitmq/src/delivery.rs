use crate::RabbitMqError;
use lapin::{
    BasicProperties, Channel, Confirmation, Connection,
    options::{BasicPublishOptions, ConfirmSelectOptions, QueueDeclareOptions},
    types::FieldTable,
};
use rom::{Delivery, DeliveryOutcome, Input};
use rom_delivery_core::{PayloadLimit, PreparedDelivery};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::sync::Semaphore;

/// A private confirm channel publishing to one host-provisioned durable queue.
///
/// The host owns TLS/authentication, queue durability/type, ACLs, topology,
/// retention, and connection recovery. Passive binding only checks existence.
/// AMQP message_id preserves identity but does not deduplicate broker messages.
/// The receiver must deduplicate durable effects. Runtime owns message retries.
/// One publication is admitted at a time; uncertain/cancelled publication retires
/// the adapter. A fresh binding is required, without implying rollback.
pub struct RabbitMqDelivery {
    channel: Channel,
    queue: String,
    deadline: Duration,
    payload_limit: PayloadLimit,
    admission: Semaphore,
    retired: AtomicBool,
}
impl RabbitMqDelivery {
    /// Create a dedicated channel, probe an existing queue, and enable confirms.
    ///
    /// Queue names are 1..=255 ASCII alphanumeric/dot/hyphen/underscore bytes,
    /// excluding reserved amq. prefixes. Deadline is positive and <=300 seconds.
    /// The queue must be host-provisioned durable; AMQP passive lookup does not
    /// validate durability, type, policies, or replication. No queue is created.
    pub async fn bind(
        connection: &Connection,
        queue: &str,
        deadline: Duration,
        payload_limit: PayloadLimit,
    ) -> Result<Self, RabbitMqError> {
        if queue.is_empty()
            || queue.len() > 255
            || queue.starts_with("amq.")
            || !queue
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_'))
            || deadline.is_zero()
            || deadline > Duration::from_secs(300)
        {
            return Err(RabbitMqError::InvalidConfiguration);
        }
        let channel = tokio::time::timeout(deadline, async {
            let channel = connection.create_channel().await?;
            channel
                .queue_declare(
                    queue.into(),
                    QueueDeclareOptions {
                        passive: true,
                        ..Default::default()
                    },
                    FieldTable::default(),
                )
                .await?;
            channel
                .confirm_select(ConfirmSelectOptions::default())
                .await?;
            Ok::<_, lapin::Error>(channel)
        })
        .await
        .map_err(|_| RabbitMqError::Unavailable)?
        .map_err(|_| RabbitMqError::Unavailable)?;
        Ok(Self {
            channel,
            queue: queue.into(),
            deadline,
            payload_limit,
            admission: Semaphore::new(1),
            retired: AtomicBool::new(false),
        })
    }
    /// Publish persistent mandatory JSON with unchanged AMQP message_id.
    ///
    /// Accepted requires Ack without return. Confirmed NO_ROUTE is Retryable.
    /// Unknown or cancellation retires this adapter; subsequent calls are
    /// Retryable before external admission. Payload/ID rejection is Permanent.
    /// ID must fit AMQP ShortString (<=255 bytes); longer IDs are never truncated.
    /// Timeouts do not cancel already-buffered writes. Host must explicitly rebind.
    pub async fn deliver<P: Input>(&self, delivery: Delivery<P>) -> DeliveryOutcome {
        let Ok(_permit) = self.admission.try_acquire() else {
            return DeliveryOutcome::Retryable;
        };
        if self.retired.load(Ordering::SeqCst) {
            return DeliveryOutcome::Retryable;
        }
        if delivery.id.len() > 255 {
            return DeliveryOutcome::Permanent;
        }
        let Ok(message) = PreparedDelivery::prepare(delivery, self.payload_limit) else {
            return DeliveryOutcome::Permanent;
        };
        let properties = BasicProperties::default()
            .with_content_type("application/json".into())
            .with_delivery_mode(2)
            .with_message_id(message.id().into());
        let mut attempt = Attempt {
            retired: &self.retired,
            resolved: false,
        };
        let operation = async {
            self.channel
                .basic_publish(
                    "".into(),
                    self.queue.clone().into(),
                    BasicPublishOptions {
                        mandatory: true,
                        ..Default::default()
                    },
                    message.body(),
                    properties,
                )
                .await?
                .await
        };
        let outcome = match tokio::time::timeout(self.deadline, operation).await {
            Ok(Ok(Confirmation::Ack(None))) => DeliveryOutcome::Accepted,
            Ok(Ok(Confirmation::Ack(Some(returned)))) if returned.reply_code == 312 => {
                DeliveryOutcome::Retryable
            }
            _ => DeliveryOutcome::Unknown,
        };
        attempt.resolved = outcome != DeliveryOutcome::Unknown;
        outcome
    }
}
// Cancellation and timeout leave an unresolved command/confirm on this channel.
// Retire before releasing admission, including cancellation of the outer future.
struct Attempt<'a> {
    retired: &'a AtomicBool,
    resolved: bool,
}
impl Drop for Attempt<'_> {
    fn drop(&mut self) {
        if !self.resolved {
            self.retired.store(true, Ordering::SeqCst);
        }
    }
}
