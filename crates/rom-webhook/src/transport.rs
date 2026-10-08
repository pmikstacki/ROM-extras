use crate::{Destination, TransportLimits, WebhookError};
use reqwest::{Certificate, Client};
use rom::{Delivery, DeliveryOutcome, Input};
use rom_delivery_core::{PayloadLimit, PreparedDelivery, WebhookSigner};
use tokio::sync::Semaphore;

/// One pinned HTTPS endpoint, bounded admission, and host-owned signing keys.
///
/// No internal retries, redirects, proxy, cookie store, or response-body reads.
/// Receiver contract: 2xx acknowledges durable acceptance; 3xx/ordinary 4xx
/// reject permanently. 408, 425, 429, 5xx and network failures are Unknown.
/// Only local overload returns Retryable, before any external attempt.
/// Runtime owns retry and authorization; receiver deduplication remains required.
pub struct Webhook {
    client: Client,
    destination: Destination,
    signer: WebhookSigner,
    payload_limit: PayloadLimit,
    admission: Semaphore,
}
impl Webhook {
    /// Construct a transport with platform certificate verification.
    /// Requires a Tokio runtime with network and time drivers for delivery.
    pub fn new(
        destination: Destination,
        signer: WebhookSigner,
        payload_limit: PayloadLimit,
        limits: TransportLimits,
    ) -> Result<Self, WebhookError> {
        Self::build(destination, signer, payload_limit, limits, None)
    }

    /// Construct a local fixture transport trusting only its explicit PEM root.
    /// This does not disable TLS hostname verification or accept arbitrary roots.
    #[cfg(feature = "loopback-fixture")]
    pub fn with_fixture_root(
        destination: Destination,
        signer: WebhookSigner,
        payload_limit: PayloadLimit,
        limits: TransportLimits,
        pem: &[u8],
    ) -> Result<Self, WebhookError> {
        if destination
            .addresses
            .iter()
            .any(|addr| !addr.ip().is_loopback())
        {
            return Err(WebhookError::ForbiddenAddress);
        }
        let root = Certificate::from_pem(pem).map_err(|_| WebhookError::ClientInitialization)?;
        Self::build(destination, signer, payload_limit, limits, Some(root))
    }

    fn build(
        destination: Destination,
        signer: WebhookSigner,
        payload_limit: PayloadLimit,
        limits: TransportLimits,
        root: Option<Certificate>,
    ) -> Result<Self, WebhookError> {
        let mut builder = Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .referer(false)
            .http1_only()
            .http1_max_headers(64)
            .pool_max_idle_per_host(0)
            .connect_timeout(limits.connect)
            .timeout(limits.request)
            .resolve_to_addrs(
                destination.url.host_str().expect("validated host"),
                &destination.addresses,
            );
        if let Some(root) = root {
            builder = builder.tls_certs_only([root]);
        }
        let client = builder
            .build()
            .map_err(|_| WebhookError::ClientInitialization)?;
        Ok(Self {
            client,
            destination,
            signer,
            payload_limit,
            admission: Semaphore::new(limits.concurrency),
        })
    }

    /// Send one signed JSON attempt with trusted host Unix seconds.
    ///
    /// Admission is immediate; no adapter queue or retry ledger is created.
    /// Dropping the future after network admission cannot prove receiver rollback.
    /// Custom payload encoding work is outside the serialized-body byte bound.
    pub async fn deliver<P: Input>(
        &self,
        delivery: Delivery<P>,
        unix_seconds: u64,
    ) -> DeliveryOutcome {
        let Ok(_permit) = self.admission.try_acquire() else {
            return DeliveryOutcome::Retryable;
        };
        let Ok(message) = PreparedDelivery::prepare(delivery, self.payload_limit) else {
            return DeliveryOutcome::Permanent;
        };
        let headers = self.signer.sign(&message, unix_seconds);
        let result = self
            .client
            .post(self.destination.url.clone())
            .header("content-type", "application/json")
            .header("webhook-id", headers.id())
            .header("webhook-timestamp", headers.timestamp())
            .header("webhook-signature", headers.signature())
            .body(message.into_body())
            .send()
            .await;
        match result {
            Ok(response) => match response.status().as_u16() {
                200..=299 => DeliveryOutcome::Accepted,
                408 | 425 | 429 | 500..=599 => DeliveryOutcome::Unknown,
                300..=499 => DeliveryOutcome::Permanent,
                _ => DeliveryOutcome::Unknown,
            },
            Err(_) => DeliveryOutcome::Unknown,
        }
    }
}
