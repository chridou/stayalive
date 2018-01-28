pub type BulkheadResult<T, E> = Result<T, BulkheadError<E>>;

#[derive(Debug, Clone, Copy)]
pub enum BulkheadError<E> {
    /// Executing a task failed
    Task(E),
    /// Too much time elapsed when executing the task
    /// or waiting for the task to be scheduled
    TimedOut,
    /// There are curently too many jobs to be processsed
    TooManyJobs(usize),
    /// The bulk head already shut down
    Closed,
    /// The resource required to for execution was not available
    ResourceAquisitionFailed,
}

pub mod shared_resources;
