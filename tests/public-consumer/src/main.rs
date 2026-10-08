//! Independent consumer of public fields, SQL execution, identity, and delivery.
use rom::{Error, Field, Result, Shape, Value};
use rom_sql_core::Executor;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Slug(String);

impl Field for Slug {
    fn shape() -> Shape {
        Shape::String
    }

    fn encode(&self) -> Value {
        Value::String(self.0.clone())
    }

    fn decode(value: Value) -> Result<Self> {
        let text = value
            .as_str()
            .filter(|value| {
                !value.is_empty() && value.bytes().all(|c| c.is_ascii_lowercase() || c == b'-')
            })
            .ok_or_else(|| Error::invalid("slug", "invalid characters"))?;
        Ok(Self(text.to_owned()))
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let value = Slug::decode(Value::String("external-consumer".into()))?;
    assert_eq!(value.encode(), Value::String("external-consumer".into()));
    assert!(Slug::decode(Value::String(String::new())).is_err());
    assert!(Slug::decode(Value::Bool(false)).is_err());
    let worker = Executor::spawn(2, || Ok(0usize))?;
    assert_eq!(
        worker.execute(|counter| {
            *counter += 1;
            *counter
        })?,
        1
    );
    worker.shutdown()?;
    let issuer = rom_oidc_presets::IssuerPreset::keycloak("https://login.example/auth", "staff")?;
    assert_eq!(issuer.issuer(), "https://login.example/auth/realms/staff");
    let message = rom_delivery_core::PreparedDelivery::prepare(
        rom::Delivery {
            id: "work-17".into(),
            attempt: 1,
            payload: false,
        },
        rom_delivery_core::PayloadLimit::default(),
    )?;
    assert_eq!(message.id(), "work-17");
    assert_eq!(message.body(), b"false");
    println!(
        "Public custom field, connection execution, issuer preset, and bounded delivery verified."
    );
    Ok(())
}
