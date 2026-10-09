//! Public-only native write, observation and persistent-tombstone conformance case.
#[path = "approved_document.rs"]
mod approved_document;
#[path = "native_fixture.rs"]
mod native_fixture;
use rom_projection_core::{ProjectionTarget, TargetFailure};
pub fn run() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (mut target, mapping) = native_fixture::fixture();
    rt.block_on(native_fixture::create_generation(&mut target));
    let first = approved_document::document(&mapping, 1, Some("independent public consumer"));
    let request = target.prepare(&[&first]).unwrap();
    assert!(matches!(
        rt.block_on(target.inspect_prepared(&request)),
        Err(TargetFailure::Rejected)
    ));
    rt.block_on(target.apply(request)).unwrap();
    let inspection = target.prepare(&[&first]).unwrap();
    assert_eq!(
        rt.block_on(target.inspect_prepared(&inspection))
            .unwrap()
            .len(),
        1
    );
    let same = approved_document::document(&mapping, 1, Some("conflicting same revision"));
    let request = target.prepare(&[&same]).unwrap();
    assert!(matches!(
        rt.block_on(target.apply(request)),
        Err(TargetFailure::Rejected)
    ));
    let tomb = approved_document::document(&mapping, 2, None);
    let request = target.prepare(&[&tomb]).unwrap();
    rt.block_on(target.apply(request)).unwrap();
    let request = target.prepare(&[&first]).unwrap();
    assert!(matches!(
        rt.block_on(target.apply(request)),
        Err(TargetFailure::Rejected)
    ));
    let inspection = target.prepare(&[&tomb]).unwrap();
    assert_eq!(
        rt.block_on(target.inspect_prepared(&inspection))
            .unwrap()
            .len(),
        1
    );
}
