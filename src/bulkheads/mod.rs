//! Bulkheads for protecting resources and downstream services

use std::error::Error;
use std::fmt;

pub mod concurrency_limiters;

/// Type alias for easier usage of the `Result` a bulkhead returns
pub type BulkheadResult<T, E> = Result<T, BulkheadError<E>>;

/// An error that can be returned from a bulkhead.
#[derive(Debug)]
pub enum BulkheadError<E> {
    /// Executing a task failed. This then contains the error the task returned
    Task(E),
    /// Too much time elapsed when executing the task
    /// or waiting for the task to be scheduled
    TimedOut,
    /// There are curently too many tasks to be processsed
    TaskLimitReached(usize),
    /// The resource required to for execution was not available
    ResourceAcquisition,
}

impl<E> fmt::Display for BulkheadError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use self::BulkheadError::*;
        match *self {
            Task(ref e) => write!(f, "task failed: {}", e),
            TimedOut => write!(f, "the task timed out"),
            TaskLimitReached(ref n) => write!(f, "there are too many tasks enqueued: {}", n),
            ResourceAcquisition => write!(f, "resource acquisition failed"),
        }
    }
}

impl<E> Error for BulkheadError<E>
where
    E: Error,
{
    fn description(&self) -> &str {
        use self::BulkheadError::*;
        match *self {
            Task(_) => "task failed",
            TimedOut => "the task timed out",
            TaskLimitReached(_) => "there are too many tasks enqueued",
            ResourceAcquisition => "resource acquisition failed",
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            BulkheadError::Task(ref e) => Some(e),
            _ => None,
        }
    }
}
