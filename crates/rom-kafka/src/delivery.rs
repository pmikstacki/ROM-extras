use crate::{KafkaDeliveryError, config::profile};
use rdkafka::{
    ClientConfig,
    error::KafkaError,
    message::{Header, OwnedHeaders},
    producer::{FutureProducer, FutureRecord},
    types::RDKafkaErrorCode,
};
use rom::{Delivery, DeliveryOutcome, Input};
use rom_delivery_core::{PayloadLimit, PreparedDelivery};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::sync::Semaphore;
/// One exact topic and privately configured acknowledged producer.
///
/// Host owns brokers, TLS/SASL, topic provisioning and durability policy.
/// Runtime owns intent identity, authority and retry. Record keys do not deduplicate effects.
pub struct KafkaDelivery {
    producer: FutureProducer,
    topic: String,
    deadline: Duration,
    limit: PayloadLimit,
    admission: Semaphore,
    retired: AtomicBool,
}
impl KafkaDelivery {
    /// Construct a nontransactional producer with all acknowledgements, zero retries and bounded buffers.
    ///
    /// Exact topic names accept ASCII alphanumeric, dot, hyphen and underscore, at most249 bytes,
    /// excluding dot and double-dot. Deadline is 1ms..=300s. No topic is created or modified.
    /// Required delivery properties override host properties; transactional.id is refused.
    /// Construction does not establish broker readiness. Security and topology remain host-owned.
    pub fn new(
        config: ClientConfig,
        topic: &str,
        deadline: Duration,
        limit: PayloadLimit,
    ) -> Result<Self, KafkaDeliveryError> {
        if topic.is_empty()
            || topic.len() > 249
            || matches!(topic, "." | "..")
            || !topic
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        {
            return Err(KafkaDeliveryError::InvalidConfiguration);
        }
        let producer = profile(config, deadline)?
            .create()
            .map_err(|_| KafkaDeliveryError::Unavailable)?;
        Ok(Self {
            producer,
            topic: topic.into(),
            deadline,
            limit,
            admission: Semaphore::new(1),
            retired: AtomicBool::new(false),
        })
    }
    /// Prepare exact JSON, enqueue once, and await a successful delivery report with a real offset.
    ///
    /// Preparation rejection is Permanent. Overload/native QueueFull is Retryable before enqueue.
    /// Uncertainty is Unknown and retires this adapter; the host explicitly constructs a replacement.
    /// Dropping this future does not cancel queued native publication or prove rollback.
    pub async fn deliver<P: Input>(&self, delivery: Delivery<P>) -> DeliveryOutcome {
        let Ok(_permit) = self.admission.try_acquire() else {
            return DeliveryOutcome::Retryable;
        };
        if self.retired.load(Ordering::Acquire) {
            return DeliveryOutcome::Retryable;
        }
        let Ok(message) = PreparedDelivery::prepare(delivery, self.limit) else {
            return DeliveryOutcome::Permanent;
        };
        let record = FutureRecord::to(&self.topic)
            .key(message.id())
            .payload(message.body())
            .headers(OwnedHeaders::new().insert(Header {
                key: "content-type",
                value: Some("application/json"),
            }));
        let future = match self.producer.send_result(record) {
            Ok(future) => future,
            Err((KafkaError::MessageProduction(RDKafkaErrorCode::QueueFull), _)) => {
                return DeliveryOutcome::Retryable;
            }
            Err(_) => {
                self.retired.store(true, Ordering::Release);
                return DeliveryOutcome::Unknown;
            }
        };
        let mut attempt = Attempt {
            retired: &self.retired,
            accepted: false,
        };
        match tokio::time::timeout(self.deadline, future).await {
            Ok(Ok(Ok(report))) if report.partition >= 0 && report.offset >= 0 => {
                attempt.accepted = true;
                DeliveryOutcome::Accepted
            }
            _ => DeliveryOutcome::Unknown,
        }
    }
}
struct Attempt<'a> {
    retired: &'a AtomicBool,
    accepted: bool,
}
impl Drop for Attempt<'_> {
    fn drop(&mut self) {
        if !self.accepted {
            self.retired.store(true, Ordering::Release);
        }
    }
}
#[cfg(test)]
#[path = "delivery_tests.rs"]
mod tests;
