//! Connection affinity, bounded admission, and honest outcome tests.
use rom_sql_core::{Executor, ExecutorError};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

const WAIT: Duration = Duration::from_secs(5);

#[test]
fn terminal_responses_are_consumed_once_even_after_failure() {
    let executor = Executor::spawn(2, || Ok(())).unwrap();
    let success = executor.submit(|_| 17).unwrap();
    assert_eq!(success.wait_timeout(WAIT), Ok(17));
    assert_eq!(success.wait(), Err(ExecutorError::OutcomeConsumed));
    let failure = executor
        .submit(|_| panic!("synthetic operation panic"))
        .unwrap();
    assert_eq!(failure.wait_timeout(WAIT), Err(ExecutorError::Unknown));
    assert_eq!(
        failure.wait_timeout(WAIT),
        Err(ExecutorError::OutcomeConsumed)
    );
    executor.shutdown().unwrap();
}

struct ConnectionDrop {
    creator: thread::ThreadId,
    completed: mpsc::Sender<thread::ThreadId>,
    panic: bool,
}

impl Drop for ConnectionDrop {
    fn drop(&mut self) {
        assert_eq!(self.creator, thread::current().id());
        self.completed.send(thread::current().id()).unwrap();
        assert!(!self.panic, "synthetic connection destructor panic");
    }
}

#[test]
fn shutdown_joins_connection_destruction_and_reports_destructor_panics() {
    for panic in [false, true] {
        let (completed, completion) = mpsc::channel();
        let executor = Executor::spawn(1, move || {
            Ok(ConnectionDrop {
                creator: thread::current().id(),
                completed,
                panic,
            })
        })
        .unwrap();
        let creator = executor.execute(|connection| connection.creator).unwrap();
        assert_ne!(creator, thread::current().id());
        assert_eq!(
            executor.shutdown(),
            if panic {
                Err(ExecutorError::Unknown)
            } else {
                Ok(())
            }
        );
        assert_eq!(completion.try_recv(), Ok(creator));
        assert!(matches!(
            executor.submit(|_| ()),
            Err(ExecutorError::Closed)
        ));
    }
}

#[test]
fn connection_is_created_and_used_on_its_worker_even_when_not_send() {
    let caller = thread::current().id();
    let executor = Executor::spawn(4, || {
        Ok((thread::current().id(), Rc::new(RefCell::new(0usize))))
    })
    .unwrap();
    for expected in 1..=3 {
        let (creator, user, value) = executor
            .execute(|state| {
                *state.1.borrow_mut() += 1;
                (state.0, thread::current().id(), *state.1.borrow())
            })
            .unwrap();
        assert_ne!(creator, caller);
        assert_eq!(creator, user);
        assert_eq!(value, expected);
    }
    executor.shutdown().unwrap();
}

#[test]
fn invalid_capacity_rejects_before_connection_initialization() {
    for capacity in [0, 65_537, usize::MAX] {
        let initialized = Arc::new(AtomicBool::new(false));
        let flag = initialized.clone();
        let result = Executor::spawn(capacity, move || {
            flag.store(true, Ordering::SeqCst);
            Ok(())
        });
        assert!(matches!(result, Err(ExecutorError::InvalidCapacity)));
        assert!(!initialized.load(Ordering::SeqCst));
    }
}

#[test]
fn full_queue_rejects_before_running_the_operation() {
    let executor = Executor::spawn(1, || Ok(0usize)).unwrap();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let first = executor
        .submit(move |state| {
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(WAIT).unwrap();
            *state += 1;
            *state
        })
        .unwrap();
    started_rx.recv_timeout(WAIT).unwrap();
    let second = executor
        .submit(|state| {
            *state += 1;
            *state
        })
        .unwrap();
    let ran = Arc::new(AtomicBool::new(false));
    let flag = ran.clone();
    let refused = executor.submit(move |_| flag.store(true, Ordering::SeqCst));
    assert!(matches!(refused, Err(ExecutorError::Overloaded)));
    release_tx.send(()).unwrap();
    assert_eq!(first.wait_timeout(WAIT), Ok(1));
    assert_eq!(second.wait_timeout(WAIT), Ok(2));
    assert!(!ran.load(Ordering::SeqCst));
    executor.shutdown().unwrap();
}

#[test]
fn waiting_deadline_preserves_ticket_and_does_not_cancel_admitted_work() {
    let executor = Executor::spawn(2, || Ok(0usize)).unwrap();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let ticket = executor
        .submit(move |state| {
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(WAIT).unwrap();
            *state = 41;
            *state
        })
        .unwrap();
    started_rx.recv_timeout(WAIT).unwrap();
    assert_eq!(
        ticket.wait_timeout(Duration::ZERO),
        Err(ExecutorError::Unknown)
    );
    release_tx.send(()).unwrap();
    assert_eq!(ticket.wait_timeout(WAIT), Ok(41));
    assert_eq!(executor.execute(|state| *state), Ok(41));
    executor.shutdown().unwrap();
}

#[test]
fn discarding_a_ticket_does_not_discard_an_admitted_operation() {
    let executor = Executor::spawn(2, || Ok(0usize)).unwrap();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let ticket = executor
        .submit(move |state| {
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(WAIT).unwrap();
            *state += 1;
        })
        .unwrap();
    started_rx.recv_timeout(WAIT).unwrap();
    drop(ticket);
    let result = executor.submit(|state| *state).unwrap();
    release_tx.send(()).unwrap();
    assert_eq!(result.wait_timeout(WAIT), Ok(1));
    executor.shutdown().unwrap();
}

#[test]
fn close_stops_admission_but_drains_accepted_operations() {
    let executor = Executor::spawn(2, || Ok(0usize)).unwrap();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let first = executor
        .submit(move |state| {
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(WAIT).unwrap();
            *state += 1;
        })
        .unwrap();
    started_rx.recv_timeout(WAIT).unwrap();
    let second = executor
        .submit(|state| {
            *state += 1;
            *state
        })
        .unwrap();
    executor.close().unwrap();
    assert!(matches!(
        executor.submit(|_| ()),
        Err(ExecutorError::Closed)
    ));
    release_tx.send(()).unwrap();
    assert_eq!(first.wait_timeout(WAIT), Ok(()));
    assert_eq!(second.wait_timeout(WAIT), Ok(2));
    executor.shutdown().unwrap();
    executor.shutdown().unwrap();
}

#[test]
fn panicking_operation_has_unknown_outcome_and_retires_connection() {
    let executor = Executor::spawn(2, || Ok(0usize)).unwrap();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let failed = executor
        .submit(move |state| {
            *state = 9;
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(WAIT).unwrap();
            panic!("synthetic driver panic after mutation");
        })
        .unwrap();
    started_rx.recv_timeout(WAIT).unwrap();
    let ran = Arc::new(AtomicBool::new(false));
    let flag = ran.clone();
    let queued = executor
        .submit(move |_| flag.store(true, Ordering::SeqCst))
        .unwrap();
    release_tx.send(()).unwrap();
    assert_eq!(failed.wait_timeout(WAIT), Err(ExecutorError::Unknown));
    assert_eq!(queued.wait_timeout(WAIT), Err(ExecutorError::Closed));
    executor.shutdown().unwrap();
    assert!(matches!(
        executor.submit(|_| ()),
        Err(ExecutorError::Closed)
    ));
    assert!(!ran.load(Ordering::SeqCst));
}

#[test]
fn initialization_errors_and_panics_are_reported_before_admission() {
    let failed = Executor::<()>::spawn(1, || Err(ExecutorError::Initialization));
    assert!(matches!(failed, Err(ExecutorError::Initialization)));
    let panicked = Executor::<()>::spawn(1, || panic!("synthetic connect panic"));
    assert!(matches!(panicked, Err(ExecutorError::Initialization)));
}

#[test]
fn cloned_handles_serialize_connection_operations() {
    let executor = Executor::spawn(32, || Ok(0usize)).unwrap();
    let threads: Vec<_> = (0..16)
        .map(|_| {
            let handle = executor.clone();
            thread::spawn(move || {
                handle
                    .execute(|state| {
                        *state += 1;
                        *state
                    })
                    .unwrap()
            })
        })
        .collect();
    let mut seen: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    seen.sort_unstable();
    assert_eq!(seen, (1..=16).collect::<Vec<_>>());
    assert_eq!(executor.execute(|state| *state), Ok(16));
    executor.shutdown().unwrap();
}

#[test]
fn recursive_use_rejects_instead_of_deadlocking_the_connection() {
    let executor = Executor::spawn(2, || Ok(())).unwrap();
    let nested = executor.clone();
    let result = executor.execute(move |_| nested.execute(|_| 3));
    assert_eq!(result, Ok(Err(ExecutorError::Reentrant)));
    let nested = executor.clone();
    assert_eq!(
        executor.execute(move |_| nested.shutdown()),
        Ok(Err(ExecutorError::Reentrant))
    );
    executor.shutdown().unwrap();
}

#[test]
fn dropping_last_handle_allows_admitted_work_to_finish() {
    let executor = Executor::spawn(2, || Ok(0usize)).unwrap();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let first = executor
        .submit(move |state| {
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(WAIT).unwrap();
            *state += 1;
        })
        .unwrap();
    started_rx.recv_timeout(WAIT).unwrap();
    let second = executor.submit(|state| *state).unwrap();
    drop(executor);
    release_tx.send(()).unwrap();
    assert_eq!(first.wait_timeout(WAIT), Ok(()));
    assert_eq!(second.wait_timeout(WAIT), Ok(1));
}
