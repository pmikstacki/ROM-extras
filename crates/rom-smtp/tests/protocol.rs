//! Authored TLS protocol checks; not native SMTP service qualification.
use rom::{Delivery, DeliveryOutcome};
use rom_email_core::{EmailNotification, EmailProfile};
use rom_smtp::{Credentials, Endpoint, Limits, Smtp};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
fn transport(hello: &str, password: &str, timeout: Duration, interval: Duration) -> Smtp {
    let port = std::env::var("ROM_EXTRAS_SMTP_FIXTURE_PORT").unwrap();
    let endpoint = Endpoint::new(
        format!("127.0.0.1:{port}").parse().unwrap(),
        "smtp.fixture.test",
        hello,
    )
    .unwrap();
    let credentials = Credentials::new("fixture".into(), password.into()).unwrap();
    let profile = EmailProfile::new(
        "sender@example.invalid",
        &["to@example.invalid"],
        "host.example.invalid",
        1048576,
    )
    .unwrap();
    Smtp::with_private_root(
        endpoint,
        credentials,
        profile,
        Limits::new(timeout, 65536, interval).unwrap(),
        &std::fs::read(std::env::var("ROM_EXTRAS_SMTP_FIXTURE_CA_DER").unwrap()).unwrap(),
    )
    .unwrap()
}
fn delivery(scenario: &str, attempt: u32) -> Delivery<EmailNotification> {
    Delivery {
        id: format!("smtp-{scenario}"),
        attempt,
        payload: EmailNotification::new(
            "to@example.invalid",
            scenario,
            "protected-message\n.\n",
            1800000000,
        )
        .unwrap(),
    }
}
#[tokio::test]
async fn actual_tls_data_acknowledgment_and_explicit_rejections() {
    let s = transport(
        "host.example.invalid",
        "synthetic-password",
        Duration::from_secs(2),
        Duration::ZERO,
    );
    assert_eq!(
        s.deliver(delivery("accept", 1)).await,
        DeliveryOutcome::Accepted
    );
    assert_eq!(
        s.deliver(delivery("transient", 1)).await,
        DeliveryOutcome::Retryable
    );
    assert_eq!(
        s.deliver(delivery("permanent", 1)).await,
        DeliveryOutcome::Permanent
    );
    assert_eq!(
        s.deliver(delivery("wrongpositive", 1)).await,
        DeliveryOutcome::Unknown
    );
}
#[tokio::test]
async fn authentication_rejection_and_unterminated_response_are_bounded() {
    let bad = transport(
        "host.example.invalid",
        "wrong-password",
        Duration::from_secs(2),
        Duration::ZERO,
    );
    assert_eq!(
        bad.deliver(delivery("badauth", 1)).await,
        DeliveryOutcome::Permanent
    );
    let overflow = transport(
        "overflow.example.invalid",
        "synthetic-password",
        Duration::from_secs(2),
        Duration::ZERO,
    );
    assert_eq!(
        overflow.deliver(delivery("overflow", 1)).await,
        DeliveryOutcome::Unknown
    );
}
#[tokio::test]
async fn loss_timeout_cancellation_and_admission_keep_uncertainty() {
    let s = transport(
        "host.example.invalid",
        "synthetic-password",
        Duration::from_millis(150),
        Duration::ZERO,
    );
    assert_eq!(
        s.deliver(delivery("loss", 1)).await,
        DeliveryOutcome::Unknown
    );
    assert_eq!(
        s.deliver(delivery("slow", 1)).await,
        DeliveryOutcome::Unknown
    );
    let token = CancellationToken::new();
    token.cancel();
    assert_eq!(
        s.deliver_cancellable(delivery("precancel", 1), &token)
            .await,
        DeliveryOutcome::Retryable
    );
    let token = CancellationToken::new();
    let future = s.deliver_cancellable(delivery("cancel", 1), &token);
    let cancel = async {
        tokio::time::sleep(Duration::from_millis(40)).await;
        token.cancel();
    };
    let (outcome, ()) = tokio::join!(future, cancel);
    assert_eq!(outcome, DeliveryOutcome::Unknown);
    let slow = s.deliver(delivery("held", 1));
    let competing = async {
        tokio::time::sleep(Duration::from_millis(40)).await;
        s.deliver(delivery("competing", 1)).await
    };
    let (a, b) = tokio::join!(slow, competing);
    assert_eq!(a, DeliveryOutcome::Unknown);
    assert_eq!(b, DeliveryOutcome::Retryable);
    assert_eq!(
        s.deliver(delivery("release", 1)).await,
        DeliveryOutcome::Accepted
    );
}
#[tokio::test]
async fn host_rate_policy_rejects_before_another_dispatch() {
    let s = transport(
        "host.example.invalid",
        "synthetic-password",
        Duration::from_secs(2),
        Duration::from_secs(60),
    );
    assert_eq!(
        s.deliver(delivery("ratefirst", 1)).await,
        DeliveryOutcome::Accepted
    );
    assert_eq!(
        s.deliver(delivery("ratesecond", 1)).await,
        DeliveryOutcome::Retryable
    );
}

#[tokio::test]
async fn explicit_rejection_survives_silent_quit() {
    let s = transport(
        "host.example.invalid",
        "synthetic-password",
        Duration::from_millis(150),
        Duration::ZERO,
    );
    assert_eq!(
        s.deliver(delivery("rejectquit", 1)).await,
        DeliveryOutcome::Retryable
    );
}

#[tokio::test]
async fn invalid_data_readiness_sends_no_body() {
    let s = transport(
        "wrongdata.example.invalid",
        "synthetic-password",
        Duration::from_secs(2),
        Duration::ZERO,
    );
    assert_eq!(
        s.deliver(delivery("wrongdata", 1)).await,
        DeliveryOutcome::Unknown
    );
}

#[tokio::test]
async fn ehlo_rejection_does_not_wait_for_quit() {
    let s = transport(
        "rejectehlo.example.invalid",
        "synthetic-password",
        Duration::from_millis(150),
        Duration::ZERO,
    );
    assert_eq!(
        s.deliver(delivery("initreject", 1)).await,
        DeliveryOutcome::Retryable
    );
}
#[tokio::test]
async fn unsupported_auth_challenge_is_unknown_without_message_submission() {
    let s = transport(
        "challenge.example.invalid",
        "synthetic-password",
        Duration::from_millis(150),
        Duration::ZERO,
    );
    assert_eq!(
        s.deliver(delivery("initreject", 1)).await,
        DeliveryOutcome::Unknown
    );
}
