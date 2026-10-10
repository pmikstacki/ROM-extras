#[path = "../../common/azure_fixture.rs"]
mod azure_fixture;
#[path = "../../public-consumer/tests/s3_fixture/framing.rs"]
mod framing;
#[path = "../../public-consumer/tests/s3_fixture/profile.rs"]
mod profile;
#[path = "../../public-consumer/tests/s3_fixture/wire.rs"]
mod wire;
pub(crate) use azure_fixture::emulator;
pub(crate) use profile::{adapter, proxy_adapter, restart};
pub(crate) use wire::Proxy;
