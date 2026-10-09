use crate::{Credentials, Endpoint, Limits, SmtpError};
use rom::{Delivery, DeliveryOutcome};
use rom_email_core::{EmailNotification, EmailProfile};
use rustls::{ClientConfig, RootCertStore, pki_types::CertificateDer};
use std::sync::{Arc, Mutex};
use tokio::{
    sync::Semaphore,
    time::{Instant, timeout_at},
};
use tokio_rustls::TlsConnector;
use tokio_util::sync::CancellationToken;
/// Authenticated implicit TLS, one pinned endpoint and bounded one-attempt submission.
/// Share this object through Arc for aggregate admission and dispatch rate policy.
/// Explicit 4xx/5xx rejects are Retryable/Permanent; opaque I/O failure is Unknown.
/// Runtime owns retries and verification. Missing native mail cannot prove NotAccepted.
pub struct Smtp {
    pub(crate) endpoint: Endpoint,
    pub(crate) credentials: Credentials,
    pub(crate) connector: TlsConnector,
    pub(crate) limits: Limits,
    profile: EmailProfile,
    admission: Semaphore,
    next: Mutex<Instant>,
}
impl Smtp {
    /// Construct with pinned public WebPKI roots, without network I/O or a default relay.
    pub fn new(
        endpoint: Endpoint,
        credentials: Credentials,
        profile: EmailProfile,
        limits: Limits,
    ) -> Result<Self, SmtpError> {
        let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        Self::build(endpoint, credentials, profile, limits, roots)
    }
    /// Replace public trust roots with one explicit DER root; never disable identity verification.
    pub fn with_private_root(
        endpoint: Endpoint,
        credentials: Credentials,
        profile: EmailProfile,
        limits: Limits,
        der: &[u8],
    ) -> Result<Self, SmtpError> {
        if der.is_empty() || der.len() > 65536 {
            return Err(SmtpError::InvalidCertificate);
        }
        let mut roots = RootCertStore::empty();
        roots
            .add(CertificateDer::from(der.to_vec()))
            .map_err(|_| SmtpError::InvalidCertificate)?;
        Self::build(endpoint, credentials, profile, limits, roots)
    }
    fn build(
        endpoint: Endpoint,
        credentials: Credentials,
        profile: EmailProfile,
        limits: Limits,
        roots: RootCertStore,
    ) -> Result<Self, SmtpError> {
        let tls =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .map_err(|_| SmtpError::ClientInitialization)?
                .with_root_certificates(roots)
                .with_no_client_auth();
        Ok(Self {
            endpoint,
            credentials,
            connector: TlsConnector::from(Arc::new(tls)),
            limits,
            profile,
            admission: Semaphore::new(1),
            next: Mutex::new(Instant::now()),
        })
    }
    /// Submit one public Runtime delivery; no internal retry, pooling, ledger or QUIT wait.
    /// Dropping this future after dispatch preserves external-effect uncertainty.
    pub async fn deliver(&self, delivery: Delivery<EmailNotification>) -> DeliveryOutcome {
        self.deliver_cancellable(delivery, &CancellationToken::new())
            .await
    }
    /// Apply an explicit cancellation token within the same whole-attempt budget.
    /// Pre-dispatch cancellation is Retryable; cancellation after dispatch is Unknown.
    pub async fn deliver_cancellable(
        &self,
        delivery: Delivery<EmailNotification>,
        cancel: &CancellationToken,
    ) -> DeliveryOutcome {
        if cancel.is_cancelled() {
            return DeliveryOutcome::Retryable;
        }
        let Ok(_permit) = self.admission.try_acquire() else {
            return DeliveryOutcome::Retryable;
        };
        let deadline = Instant::now() + self.limits.timeout;
        let Ok(message) = self.profile.prepare_smtp(delivery) else {
            return DeliveryOutcome::Permanent;
        };
        if cancel.is_cancelled() || Instant::now() >= deadline {
            return DeliveryOutcome::Retryable;
        }
        {
            let Ok(mut next) = self.next.lock() else {
                return DeliveryOutcome::Unknown;
            };
            let now = Instant::now();
            if now < *next {
                return DeliveryOutcome::Retryable;
            }
            *next = now + self.limits.interval;
        }
        tokio::select! {biased;
         _=cancel.cancelled()=>DeliveryOutcome::Unknown,
         outcome=timeout_at(deadline,self.attempt(&message))=>outcome.unwrap_or(DeliveryOutcome::Unknown),
        }
    }
}
