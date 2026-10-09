mod runtime;
use rom::{Delivery, DeliveryOutcome};
use rom_email_core::{EmailNotification, EmailProfile};
use rom_smtp::{Credentials, Endpoint, Limits, Smtp};
use std::time::Duration;
fn value(name: &str) -> String {
    std::env::var(name).unwrap()
}
fn profile() -> EmailProfile {
    EmailProfile::new(
        "sender@example.invalid",
        &["Exact+Tag@example.invalid"],
        "host.example.invalid",
        1048576,
    )
    .unwrap()
}
fn endpoint(identity: &str) -> Endpoint {
    Endpoint::new(
        value("ROM_EXTRAS_SMTP_ADDRESS").parse().unwrap(),
        identity,
        "host.example.invalid",
    )
    .unwrap()
}
fn credentials(password: String) -> Credentials {
    Credentials::new(value("ROM_EXTRAS_SMTP_USERNAME"), password).unwrap()
}
fn limits() -> Limits {
    Limits::new(Duration::from_secs(3), 65536, Duration::ZERO).unwrap()
}
fn delivery() -> Delivery<EmailNotification> {
    let id = value("ROM_EXTRAS_SMTP_RUN_ID");
    Delivery {
        id: format!("native-{id}"),
        attempt: 1,
        payload: EmailNotification::new(
            "Exact+Tag@example.invalid",
            &format!("native-{id}"),
            "Treść natywna\n.\n",
            1800000000,
        )
        .unwrap(),
    }
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let ca = std::fs::read(value("ROM_EXTRAS_SMTP_CA_DER")).unwrap();
    let phase = std::env::args().nth(1).unwrap();
    match phase.as_str() {
        "accept" => {
            let s = Smtp::with_private_root(
                endpoint("mailpit.fixture.test"),
                credentials(value("ROM_EXTRAS_SMTP_PASSWORD")),
                profile(),
                limits(),
                &ca,
            )
            .unwrap();
            assert_eq!(s.deliver(delivery()).await, DeliveryOutcome::Accepted);
        }
        "runtime" => runtime::qualify(&ca).await,
        "negative" => {
            let s = Smtp::with_private_root(
                endpoint("mailpit.fixture.test"),
                credentials("intentionally-wrong-fixture-password".into()),
                profile(),
                limits(),
                &ca,
            )
            .unwrap();
            assert_eq!(s.deliver(delivery()).await, DeliveryOutcome::Permanent);
            let s = Smtp::with_private_root(
                endpoint("wrong.fixture.test"),
                credentials(value("ROM_EXTRAS_SMTP_PASSWORD")),
                profile(),
                limits(),
                &ca,
            )
            .unwrap();
            assert_eq!(s.deliver(delivery()).await, DeliveryOutcome::Unknown);
            let s = Smtp::new(
                endpoint("mailpit.fixture.test"),
                credentials(value("ROM_EXTRAS_SMTP_PASSWORD")),
                profile(),
                limits(),
            )
            .unwrap();
            assert_eq!(s.deliver(delivery()).await, DeliveryOutcome::Unknown);
        }
        _ => panic!("invalid fixture consumer phase"),
    }
    println!("Native SMTP {phase} consumer phase passed; internet delivery remains unqualified");
}
