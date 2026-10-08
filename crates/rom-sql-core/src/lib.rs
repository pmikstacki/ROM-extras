//! Shared execution machinery for ROM SQL adapters.
//!
//! [`Executor`] confines one connection to a dedicated worker. It provides
//! bounded admission and preserves uncertainty after a caller's waiting deadline.
//! This crate does not yet implement ROM's Storage contract.
//!
//! ```
//! use rom_sql_core::Executor;
//!
//! let connection = Executor::spawn(8, || Ok(0usize))?;
//! let result = connection.execute(|value| {
//!     *value += 1;
//!     *value
//! })?;
//! assert_eq!(result, 1);
//! connection.shutdown()?;
//! # Ok::<(), rom_sql_core::ExecutorError>(())
//! ```

mod error;
mod executor;
mod ticket;

pub use error::ExecutorError;
pub use executor::Executor;
pub use ticket::Ticket;
