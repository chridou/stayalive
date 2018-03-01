//! Bulkheads for protecting resources and downstream services
use std::fmt;

use failure::*;

pub mod concurrency_limiters;

/// Type alias for easier usage of the `Result` a bulkhead returns
pub type BulkheadResult<T, E> = Result<T, BulkheadError<E>>;

/// An error that can be returned from a bulkhead.
#[derive(Debug, Clone, Fail)]
pub enum BulkheadError<E: fmt::Display> {
    /// Executing a task failed. This then contains the error the task returned
    #[fail(display = "Task failed: {}", _0)]
    Task(E),
    /// Too much time elapsed when executing the task
    /// or waiting for the task to be scheduled
    #[fail(display = "Timed out")]
    TimedOut,
    /// There are curently too many tasks to be processsed
    #[fail(display = "Task limit reached({} running)", _0)]
    TaskLimitReached(usize),
    /// The resource required to for execution was not available
    #[fail(display = "Resource acquisition failed: {}", _0)]
    ResourceAcquisition(String),
}
