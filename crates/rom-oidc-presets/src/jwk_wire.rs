use serde::{Deserialize, Deserializer, de::IgnoredAny};
use std::collections::BTreeMap;
#[derive(Deserialize)]
pub(crate) struct Set {
    pub(crate) keys: Vec<Key>,
}
#[derive(Deserialize)]
pub(crate) struct Key {
    pub(crate) kid: String,
    pub(crate) kty: String,
    #[serde(default, deserialize_with = "present")]
    pub(crate) alg: Option<String>,
    #[serde(rename = "use", default, deserialize_with = "present")]
    pub(crate) usage: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub(crate) key_ops: Option<Vec<String>>,
    #[serde(default, deserialize_with = "present")]
    pub(crate) n: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub(crate) e: Option<String>,
    #[serde(flatten)]
    pub(crate) extensions: BTreeMap<String, IgnoredAny>,
}
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}
