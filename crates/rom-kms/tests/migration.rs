//! Public migration capability, lifetime and deadline boundaries.
use rom_kms::{Binding, EncryptionVersion, Envelope, KeyRef, Kms, MigrationLimits, Migrator};
use rom_secrets::{Error, SecretBytes, SecretRef};
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
struct Basic(AtomicUsize);
impl Kms for Basic {
    async fn encrypt(&self, _: &KeyRef, _: &SecretBytes, _: &Binding) -> Result<Envelope, Error> {
        Err(Error::Unavailable)
    }
    async fn decrypt(&self, _: &Envelope, _: &Binding) -> Result<SecretBytes, Error> {
        self.0.fetch_add(1, Ordering::SeqCst);
        SecretBytes::new(vec![0, 255, 42])
    }
}
struct Provider {
    calls: AtomicUsize,
    stage: AtomicUsize,
    delay: Duration,
    bad: usize,
}
impl Provider {
    fn new(delay: Duration, bad: usize) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            stage: AtomicUsize::new(0),
            delay,
            bad,
        }
    }
}
fn source() -> Envelope {
    Envelope::new(
        SecretRef::new("profile").unwrap(),
        KeyRef::new("source").unwrap(),
        1,
        "original".into(),
    )
    .unwrap()
}
fn binding() -> Binding {
    Binding::new(vec![0, 1], vec![255, 42]).unwrap()
}
fn destination() -> KeyRef {
    KeyRef::new("destination").unwrap()
}
impl Kms for Provider {
    fn validate_encryption(&self, key: &KeyRef, _: EncryptionVersion) -> Result<(), Error> {
        if key.as_str() == "destination" {
            Ok(())
        } else {
            Err(Error::Invalid)
        }
    }
    async fn encrypt(&self, _: &KeyRef, _: &SecretBytes, _: &Binding) -> Result<Envelope, Error> {
        Err(Error::Unsupported)
    }
    async fn decrypt(&self, _: &Envelope, binding: &Binding) -> Result<SecretBytes, Error> {
        assert_eq!(binding.context(), [0, 1]);
        assert_eq!(binding.aad(), [255, 42]);
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.stage.store(1, Ordering::SeqCst);
        if self.bad == 6 {
            std::thread::sleep(Duration::from_millis(30));
            return Err(Error::Rejected);
        }
        tokio::time::sleep(self.delay).await;
        if self.bad == 4 {
            return Err(Error::Rejected);
        }
        SecretBytes::new(vec![0, 255, 42])
    }
    async fn encrypt_at(
        &self,
        key: &KeyRef,
        plaintext: &SecretBytes,
        binding: &Binding,
        _: EncryptionVersion,
    ) -> Result<Envelope, Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.stage.store(2, Ordering::SeqCst);
        assert!(plaintext.expose() == [0, 255, 42]);
        assert_eq!(binding.context(), [0, 1]);
        assert_eq!(binding.aad(), [255, 42]);
        if self.bad == 5 {
            std::thread::sleep(Duration::from_millis(30));
        } else {
            tokio::time::sleep(self.delay).await;
        }
        Envelope::new(
            SecretRef::new(if self.bad == 1 { "other" } else { "profile" }).unwrap(),
            if self.bad == 2 {
                KeyRef::new("other").unwrap()
            } else {
                key.clone()
            },
            if self.bad == 3 { 3 } else { 2 },
            "replacement".into(),
        )
    }
}
#[test]
fn version_and_migration_limits_are_validated() {
    assert!(matches!(EncryptionVersion::pinned(0), Err(Error::Invalid)));
    assert!(EncryptionVersion::pinned(u64::MAX).is_ok());
    for limits in [
        MigrationLimits {
            deadline: Duration::ZERO,
            ..MigrationLimits::default()
        },
        MigrationLimits {
            deadline: Duration::from_secs(61),
            ..MigrationLimits::default()
        },
        MigrationLimits {
            max_in_flight: 0,
            ..MigrationLimits::default()
        },
        MigrationLimits {
            max_in_flight: 65,
            ..MigrationLimits::default()
        },
    ] {
        assert!(matches!(
            Migrator::new(Basic(AtomicUsize::new(0)), limits),
            Err(Error::Invalid)
        ));
    }
}
#[tokio::test]
async fn unsupported_latest_and_unapproved_refuse_before_decrypt() {
    let m = Migrator::new(Basic(AtomicUsize::new(0)), MigrationLimits::default()).unwrap();
    assert!(matches!(
        m.migrate(
            &source(),
            &binding(),
            &destination(),
            EncryptionVersion::pinned(2).unwrap()
        )
        .await,
        Err(Error::Unsupported)
    ));
    assert!(matches!(
        m.migrate(
            &source(),
            &binding(),
            &destination(),
            EncryptionVersion::Latest
        )
        .await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        m.provider()
            .encrypt_at(
                &destination(),
                &SecretBytes::new(vec![0]).unwrap(),
                &binding(),
                EncryptionVersion::Latest
            )
            .await,
        Err(Error::Unavailable)
    ));
    assert_eq!(m.provider().0.load(Ordering::SeqCst), 0);
    let m = Migrator::new(Provider::new(Duration::ZERO, 0), MigrationLimits::default()).unwrap();
    assert!(matches!(
        m.migrate(
            &source(),
            &binding(),
            &KeyRef::new("unknown").unwrap(),
            EncryptionVersion::pinned(2).unwrap()
        )
        .await,
        Err(Error::Invalid)
    ));
    assert_eq!(m.provider().calls.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn exact_binding_and_bytes_preserved_original_unchanged() {
    let m = Migrator::new(Provider::new(Duration::ZERO, 0), MigrationLimits::default()).unwrap();
    let original = source();
    let out = m
        .migrate(
            &original,
            &binding(),
            &destination(),
            EncryptionVersion::pinned(2).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(out.key(), &destination());
    assert_eq!(out.version(), 2);
    assert_eq!(original.ciphertext(), "original");
    assert_eq!(m.provider().calls.load(Ordering::SeqCst), 2);
}
#[tokio::test]
async fn mismatched_response_and_failed_authentication_do_not_publish() {
    for bad in 1..=4 {
        let m = Migrator::new(
            Provider::new(Duration::ZERO, bad),
            MigrationLimits::default(),
        )
        .unwrap();
        assert!(matches!(
            m.migrate(
                &source(),
                &binding(),
                &destination(),
                EncryptionVersion::pinned(2).unwrap()
            )
            .await,
            Err(Error::Protocol | Error::Rejected)
        ));
        assert_eq!(
            m.provider().calls.load(Ordering::SeqCst),
            if bad == 4 { 1 } else { 2 }
        );
    }
}
#[tokio::test]
async fn combined_deadline_and_late_non_yielding_completion_refuse_publication() {
    for (delay, bad) in [
        (Duration::from_millis(30), 0),
        (Duration::ZERO, 5),
        (Duration::ZERO, 6),
    ] {
        let m = Migrator::new(
            Provider::new(delay, bad),
            MigrationLimits {
                deadline: Duration::from_millis(20),
                max_in_flight: 1,
            },
        )
        .unwrap();
        assert!(matches!(
            m.migrate(
                &source(),
                &binding(),
                &destination(),
                EncryptionVersion::pinned(2).unwrap()
            )
            .await,
            Err(Error::Timeout)
        ));
    }
    let m = Migrator::new(
        Provider::new(Duration::from_millis(30), 0),
        MigrationLimits {
            deadline: Duration::from_millis(45),
            max_in_flight: 1,
        },
    )
    .unwrap();
    assert!(matches!(
        m.migrate(
            &source(),
            &binding(),
            &destination(),
            EncryptionVersion::pinned(2).unwrap()
        )
        .await,
        Err(Error::Timeout)
    ));
    assert_eq!(m.provider().stage.load(Ordering::SeqCst), 2);
}
#[tokio::test]
async fn cancellation_and_admission_cover_both_calls_and_recover() {
    use std::{future::Future, task::Poll};
    let m = Migrator::new(
        Provider::new(Duration::from_millis(30), 0),
        MigrationLimits {
            max_in_flight: 1,
            ..MigrationLimits::default()
        },
    )
    .unwrap();
    let s = source();
    let b = binding();
    let k = destination();
    let v = EncryptionVersion::pinned(2).unwrap();
    {
        let pending = m.migrate(&s, &b, &k, v);
        drop(pending);
    }
    assert_eq!(m.provider().calls.load(Ordering::SeqCst), 0);
    for wanted in [1, 2] {
        let mut first = Box::pin(m.migrate(&s, &b, &k, v));
        std::future::poll_fn(|cx| {
            assert!(matches!(first.as_mut().poll(cx), Poll::Pending));
            Poll::Ready(())
        })
        .await;
        if wanted == 2 {
            tokio::time::sleep(Duration::from_millis(35)).await;
            std::future::poll_fn(|cx| {
                assert!(matches!(first.as_mut().poll(cx), Poll::Pending));
                Poll::Ready(())
            })
            .await;
        }
        assert_eq!(m.provider().stage.load(Ordering::SeqCst), wanted);
        assert!(matches!(m.migrate(&s, &b, &k, v).await, Err(Error::Busy)));
        drop(first);
        assert!(m.migrate(&s, &b, &k, v).await.is_ok());
    }
}
