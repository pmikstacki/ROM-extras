//! Real PostgreSQL executor checks; not ROM Storage conformance.
#[path = "../../common/pgwire/mod.rs"]
mod pgwire;
use postgres::Client;
use rom_sql_core::Executor;
fn connection() -> Executor<Client> {
    pgwire::connection("ROM_EXTRAS_POSTGRES_DSN", 3000)
}

#[tokio::test(flavor = "current_thread")]
async fn postgres_client_runs_on_its_worker_inside_a_tokio_caller() {
    let executor = connection();
    let value = executor
        .execute(|client| {
            client
                .query_one("SELECT 17::BIGINT", &[])
                .map(|row| row.get::<_, i64>(0))
        })
        .unwrap()
        .unwrap();
    assert_eq!(value, 17);
    let settings = executor
        .execute(|client| {
            client.query_one(
                "SELECT current_setting('fsync'), current_setting('synchronous_commit'), version()",
                &[],
            )
        })
        .unwrap()
        .unwrap();
    assert_eq!(settings.get::<_, String>(0), "on");
    assert_eq!(settings.get::<_, String>(1), "on");
    println!("Backend: {}", settings.get::<_, String>(2));
    executor.shutdown().unwrap();
}

#[test]
fn waiting_deadline_does_not_cancel_the_real_database_commit() {
    pgwire::deadline(connection);
}
#[test]
fn a_driver_job_panic_retires_its_connection_and_rolls_back_uncommitted_sql() {
    pgwire::panic_rollback(connection);
}
