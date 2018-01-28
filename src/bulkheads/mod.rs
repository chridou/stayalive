//! Bulkheads for protecting resources and downstream services

pub mod concurrency_limiters;

pub type BulkheadResult<T, E> = Result<T, BulkheadError<E>>;

/// An error that can be returned from a bulk head itself.
#[derive(Debug, Clone, Copy)]
pub enum BulkheadError<E> {
    /// Executing a task failed. This then contains the error the task returned
    Task(E),
    /// Too much time elapsed when executing the task
    /// or waiting for the task to be scheduled
    TimedOut,
    /// There are curently too many tasks to be processsed
    TooManyTasks(usize),
    /// The bulk head already shut down
    Closed,
    /// The resource required to for execution was not available
    ResourceAquisitionFailed,
}
