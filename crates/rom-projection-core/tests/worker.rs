//! Actual durable files with a synthetic trusted target, not native provider qualification.
mod support;
mod worker_fixture;
use rom_projection_core::{Cancellation, Error, WorkerFailure};
use worker_fixture::*;
#[test]
fn native_intent_precedes_dispatch_and_exact_observation_publishes_cursor() {
    let mut f = Fixture::new(Mode::Exact);
    let rt = runtime();
    rt.block_on(f.worker.apply_page(
        support::cursor(3),
        vec![document(2, 1)],
        &Cancellation::new(),
    ))
    .unwrap();
    let s = rt.block_on(f.worker.snapshot()).unwrap();
    assert_eq!(s.checkpoint().cursor("document"), Some(&support::cursor(3)));
    assert!(s.pending_page().is_none());
    assert_eq!(f.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    f.worker.shutdown().unwrap();
}
#[test]
fn request_validation_rejects_before_native_intent_and_dispatch() {
    let mut f = Fixture::new(Mode::TooLarge);
    let rt = runtime();
    assert_eq!(
        rt.block_on(f.worker.apply_page(
            support::cursor(3),
            vec![document(2, 1)],
            &Cancellation::new()
        )),
        Err(WorkerFailure::Core(Error::TooLarge))
    );
    let s = rt.block_on(f.worker.snapshot()).unwrap();
    assert!(s.pending_page().is_none());
    assert_eq!(s.checkpoint().sequence(), 0);
    assert_eq!(f.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}
#[test]
fn unknown_remote_result_keeps_pending_and_blocks_later_page() {
    let mut f = Fixture::new(Mode::Unknown);
    let rt = runtime();
    assert_eq!(
        rt.block_on(f.worker.apply_page(
            support::cursor(3),
            vec![document(2, 1)],
            &Cancellation::new()
        )),
        Err(WorkerFailure::Target(
            rom_projection_core::TargetFailure::Unknown
        ))
    );
    let s = rt.block_on(f.worker.snapshot()).unwrap();
    assert!(s.pending_page().is_some());
    assert_eq!(s.checkpoint().cursor("document"), Some(&support::cursor(0)));
    assert_eq!(
        rt.block_on(f.worker.apply_page(
            support::cursor(5),
            vec![document(4, 2)],
            &Cancellation::new()
        )),
        Err(WorkerFailure::RecoveryRequired)
    );
}
#[test]
fn higher_remote_revision_is_not_exact_acknowledgement() {
    let mut f = Fixture::new(Mode::Higher);
    let rt = runtime();
    assert_eq!(
        rt.block_on(f.worker.apply_page(
            support::cursor(3),
            vec![document(2, 1)],
            &Cancellation::new()
        )),
        Err(WorkerFailure::Core(Error::Conflict))
    );
    assert!(
        rt.block_on(f.worker.snapshot())
            .unwrap()
            .pending_page()
            .is_some()
    );
}
#[test]
fn empty_filtered_page_commits_without_backend_request() {
    let mut f = Fixture::new(Mode::TooLarge);
    let rt = runtime();
    rt.block_on(
        f.worker
            .apply_page(support::cursor(4), vec![], &Cancellation::new()),
    )
    .unwrap();
    assert_eq!(f.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(
        rt.block_on(f.worker.snapshot())
            .unwrap()
            .checkpoint()
            .cursor("document"),
        Some(&support::cursor(4))
    );
}
#[test]
fn restart_recovery_reconstructs_old_interval_before_replay_and_ignores_overrun() {
    let mut f = Fixture::new(Mode::Unknown);
    let rt = runtime();
    assert!(
        rt.block_on(f.worker.apply_page(
            support::cursor(3),
            vec![document(2, 1)],
            &Cancellation::new()
        ))
        .is_err()
    );
    f.restart(Mode::Exact);
    let mut source = History::new(vec![event(2, 1), event(5, 2)], 6);
    rt.block_on(f.worker.recover(&mut source, &Cancellation::new()))
        .unwrap()
        .unwrap();
    assert_eq!(source.mapped, 1);
    assert_eq!(
        rt.block_on(f.worker.snapshot())
            .unwrap()
            .checkpoint()
            .cursor("document"),
        Some(&support::cursor(3))
    );
}
#[test]
fn changed_disclosure_after_restart_never_dispatches_or_advances() {
    let mut f = Fixture::new(Mode::Unknown);
    let rt = runtime();
    assert!(
        rt.block_on(f.worker.apply_page(
            support::cursor(3),
            vec![document(2, 1)],
            &Cancellation::new()
        ))
        .is_err()
    );
    f.restart(Mode::Exact);
    let before = f.calls.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        rt.block_on(
            f.worker
                .recover(&mut History::new(vec![], 4), &Cancellation::new())
        ),
        Err(WorkerFailure::Core(Error::RebuildRequired))
    );
    assert_eq!(f.calls.load(std::sync::atomic::Ordering::SeqCst), before);
    assert!(
        rt.block_on(f.worker.snapshot())
            .unwrap()
            .pending_page()
            .is_some()
    );
}

#[test]
fn cancellation_after_host_acceptance_preserves_pending_before_local_completion() {
    let mut f = Fixture::new(Mode::Cancel);
    let rt = runtime();
    assert_eq!(
        rt.block_on(
            f.worker
                .apply_page(support::cursor(3), vec![document(2, 1)], &f.cancel)
        ),
        Err(WorkerFailure::Core(Error::Cancelled))
    );
    let state = rt.block_on(f.worker.snapshot()).unwrap();
    assert!(state.pending_page().is_some());
    assert_eq!(
        state.checkpoint().cursor("document"),
        Some(&support::cursor(0))
    );
    f.restart(Mode::Exact);
    rt.block_on(f.worker.recover(
        &mut History::new(vec![event(2, 1)], 3),
        &Cancellation::new(),
    ))
    .unwrap()
    .unwrap();
}
#[test]
fn dropped_future_after_host_acceptance_keeps_native_intent_for_recovery() {
    use std::future::Future;
    let mut f = Fixture::new(Mode::Wait);
    let rt = runtime();
    let cancel = Cancellation::new();
    let mut entered = f.entered.take().unwrap();
    let mut attempt = Box::pin(f.worker.apply_page(
        support::cursor(3),
        vec![document(2, 1)],
        &cancel,
    ));
    rt.block_on(std::future::poll_fn(|context| {
        assert!(attempt.as_mut().poll(context).is_pending());
        std::pin::Pin::new(&mut entered)
            .poll(context)
            .map(|r| r.unwrap())
    }));
    drop(attempt);
    drop(f.release.take());
    assert!(
        rt.block_on(f.worker.snapshot())
            .unwrap()
            .pending_page()
            .is_some()
    );
    f.restart(Mode::Exact);
    rt.block_on(f.worker.recover(
        &mut History::new(vec![event(2, 1)], 3),
        &Cancellation::new(),
    ))
    .unwrap()
    .unwrap();
    assert_eq!(
        rt.block_on(f.worker.snapshot())
            .unwrap()
            .checkpoint()
            .cursor("document"),
        Some(&support::cursor(3))
    );
}
#[test]
fn precancelled_attempt_never_prepares_or_dispatches() {
    let mut f = Fixture::new(Mode::Exact);
    let rt = runtime();
    let cancel = Cancellation::new();
    cancel.cancel();
    assert_eq!(
        rt.block_on(
            f.worker
                .apply_page(support::cursor(3), vec![document(2, 1)], &cancel)
        ),
        Err(WorkerFailure::Core(Error::Cancelled))
    );
    assert_eq!(
        rt.block_on(f.worker.snapshot())
            .unwrap()
            .checkpoint()
            .sequence(),
        0
    );
    assert_eq!(f.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[test]
fn recovery_never_fetches_a_sixty_fifth_history_page() {
    struct TinyHistory {
        profile: rom_projection_core::ProjectionProfile,
        calls: u64,
    }
    impl rom_projection_core::ProjectionHistory for TinyHistory {
        fn profile(&self) -> &rom_projection_core::ProjectionProfile {
            &self.profile
        }
        async fn fetch(
            &mut self,
            after: &rom::JournalCursor,
        ) -> rom_projection_core::Result<rom::JournalBatch> {
            self.calls += 1;
            Ok(rom::JournalBatch {
                events: vec![],
                cursor: support::cursor(after.position + 1),
            })
        }
        fn document(
            &mut self,
            _: &rom::JournalView,
        ) -> rom_projection_core::Result<rom_projection_core::ApprovedDocument> {
            panic!("empty source has no documents")
        }
    }
    // The selected old document exists at a later position, beyond sixty-four empty bounded batches.
    let mut f = Fixture::new(Mode::Unknown);
    let rt = runtime();
    assert!(
        rt.block_on(f.worker.apply_page(
            support::cursor(100),
            vec![document(90, 1)],
            &Cancellation::new()
        ))
        .is_err()
    );
    let mut source = TinyHistory {
        profile: mapping().profile().clone(),
        calls: 0,
    };
    assert_eq!(
        rt.block_on(f.worker.recover(&mut source, &Cancellation::new())),
        Err(WorkerFailure::Core(Error::TooLarge))
    );
    assert_eq!(source.calls, 64);
    assert!(
        rt.block_on(f.worker.snapshot())
            .unwrap()
            .pending_page()
            .is_some()
    );
}

#[test]
fn unreconstructable_span_is_rejected_before_native_intent() {
    let mut f = Fixture::new(Mode::Unknown);
    let rt = runtime();
    assert_eq!(
        rt.block_on(f.worker.apply_page(
            support::cursor(4097),
            vec![document(2, 1)],
            &Cancellation::new()
        )),
        Err(WorkerFailure::Core(Error::TooLarge))
    );
    let s = rt.block_on(f.worker.snapshot()).unwrap();
    assert!(s.pending_page().is_none());
    assert_eq!(s.checkpoint().sequence(), 0);
    assert_eq!(f.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}
