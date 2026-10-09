//! Public checkpoint API exercised independently of workspace feature unification.
#[path = "../../packaged-projections/checkpoint_case.rs"]
mod checkpoint_case;
#[test]
fn public_checkpoint_consumer_closes_reopens_and_reconciles_both_phases() {
    checkpoint_case::run();
}

#[path = "../../packaged-projections/worker_case.rs"]
mod worker_case;
#[test]
fn public_worker_recovers_pending_history_after_restart() {
    worker_case::run();
}
