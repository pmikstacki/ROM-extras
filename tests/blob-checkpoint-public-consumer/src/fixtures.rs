#[path = "../../common/azure_fixture.rs"]
mod azure_fixture;
// The shared S3 fixture also defines restart helpers used by other consumers.
#[allow(dead_code)]
#[path = "../../public-consumer/tests/s3_fixture/profile.rs"]
mod profile;
pub(crate) use azure_fixture::emulator;
pub(crate) use profile::adapter;
