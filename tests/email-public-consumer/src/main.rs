use rom::{Channel, Delivery, Input};
use rom_email_core::{EmailNotification, EmailProfile};
fn main() {
    let payload = EmailNotification::new(
        "Exact+Tag@example.invalid",
        "Zażółć gęślą",
        "Treść\n\n.\nDalszy wiersz\t0",
        1800000000,
    )
    .unwrap();
    let channel = Channel::<EmailNotification>::new("email", 1);
    let _intent = channel.intent(payload.clone());
    let restored = EmailNotification::decode(payload.encode()).unwrap();
    let profile = EmailProfile::new(
        "sender@example.invalid",
        &["Exact+Tag@example.invalid"],
        "host.example.invalid",
        1048576,
    )
    .unwrap();
    let a = profile
        .prepare(Delivery {
            id: "work-17".into(),
            attempt: 1,
            payload: payload.clone(),
        })
        .unwrap();
    let b = profile
        .prepare(Delivery {
            id: "work-17".into(),
            attempt: 2,
            payload: restored,
        })
        .unwrap();
    assert_eq!(a.body(), b.body());
    println!(
        "{}",
        serde_json::json!({"wire":String::from_utf8(a.body().to_vec()).unwrap(),"id":a.message_id(),"from":a.from(),"to":a.to()})
    );
}
