use std::fmt;

use failure::*;

pub mod naive;

pub type CircuitBreakerResult<T, E> = Result<T, CircuitBreakerError<E>>;

#[derive(Debug, Clone, Fail)]
pub enum CircuitBreakerError<E: fmt::Display> {
    #[fail(display = "Circuit breaker open")]
    Open,
    #[fail(display = "Execution failed: {}", _0)]
    Execution(E),
}

#[derive(Debug, Clone, Copy)]
pub enum State {
    Open,
    HalfOpen,
    Closed,
}
