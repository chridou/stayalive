pub mod naive;

pub type CircuitBreakerResult<T, E> = Result<T, CircuitBreakerError<E>>;

pub enum CircuitBreakerError<E> {
    Open,
    Execution(E),
}

#[derive(Debug, Clone, Copy)]
pub enum State {
    Open,
    HalfOpen,
    Closed,
}

pub trait CircuitBreaker {
    fn execute<T, E, F>(&self, f: F) -> CircuitBreakerResult<T, E>
    where
        T: Send + 'static,
        E: Send + 'static,
        F: Fn() -> Result<T, E>;

    fn state(&self) -> State;
}
