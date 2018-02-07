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
