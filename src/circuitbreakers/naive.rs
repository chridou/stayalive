//! A very simplistc circuit breaker

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use circuitbreakers::{CircuitBreakerError, CircuitBreakerResult, State};

#[derive(Clone)]
pub struct NcbState {
    errors: usize,
    open_until: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub max_errors: usize,
    pub recovery_period: Duration,
}

impl Config {
    pub fn with_max_errors(mut self, n: usize) -> Self {
        self.max_errors = n;
        self
    }

    pub fn with_recovery_period(mut self, d: Duration) -> Self {
        self.recovery_period = d;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_errors: 10,
            recovery_period: Duration::from_secs(1),
        }
    }
}

/// A very simple circuit breaker that opens on consecutive failures.
///
/// This circuit breaker is not very accurate and its state
/// might flicker a bit. So there might be a few more attempts
/// being made even though the circuit breaker
/// should have opened already.
#[derive(Clone)]
pub struct NaiveCircuitBreaker {
    max_errors: usize,
    recovery_period: Duration,
    ncb_state: Arc<Mutex<NcbState>>,
}

impl NaiveCircuitBreaker {
    pub fn new(config: Config) -> Result<NaiveCircuitBreaker, String> {
        Ok(NaiveCircuitBreaker {
            max_errors: config.max_errors,
            recovery_period: config.recovery_period,
            ncb_state: Arc::new(Mutex::new(NcbState {
                errors: 0,
                open_until: None,
            })),
        })
    }

    /// Call the given `Fn` from the circuit breaker
    pub fn call<T, E, F>(&self, f: F) -> CircuitBreakerResult<T, E>
    where
        F: Fn() -> Result<T, E>,
    {
        let state = { self.ncb_state.lock().unwrap().clone() };

        if let Some(until) = state.open_until {
            if until > Instant::now() {
                return Err(CircuitBreakerError::Open);
            } else {
                // Half open. Open phase is over and nobody set it to None yet.
                match f() {
                    Ok(r) => {
                        let state = &mut self.ncb_state.lock().unwrap();
                        state.open_until = None;
                        state.errors = 0;
                        return Ok(r);
                    }
                    Err(err) => {
                        // Failed in half open. Immediately open again.
                        let state = &mut self.ncb_state.lock().unwrap();
                        state.open_until = Some(Instant::now() + self.recovery_period);
                        state.errors = 0;
                        return Err(CircuitBreakerError::Execution(err));
                    }
                }
            }
        } else {
            match f() {
                Ok(r) => {
                    let state = &mut self.ncb_state.lock().unwrap();
                    state.errors = 0;
                    state.open_until = None;
                    return Ok(r);
                }
                Err(err) => {
                    // Failed in closed. Check what to do.
                    if state.errors < self.max_errors {
                        // Still fine
                        let state = &mut self.ncb_state.lock().unwrap();
                        state.errors += 1;
                        state.open_until = None;
                    } else {
                        // Open now...
                        let state = &mut self.ncb_state.lock().unwrap();
                        state.errors = 0;
                        state.open_until = Some(Instant::now() + self.recovery_period);
                    }
                    return Err(CircuitBreakerError::Execution(err));
                }
            }
        }
    }

    /// Call the given `FnOnce` from the circuit breaker
    pub fn call_once<T, E, F>(&self, f: F) -> CircuitBreakerResult<T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        let state = { self.ncb_state.lock().unwrap().clone() };

        if let Some(until) = state.open_until {
            if until > Instant::now() {
                return Err(CircuitBreakerError::Open);
            } else {
                // Half open. Open phase is over and nobody set it to None yet.
                match f() {
                    Ok(r) => {
                        let state = &mut self.ncb_state.lock().unwrap();
                        state.open_until = None;
                        state.errors = 0;
                        return Ok(r);
                    }
                    Err(err) => {
                        // Failed in half open. Immediately open again.
                        let state = &mut self.ncb_state.lock().unwrap();
                        state.open_until = Some(Instant::now() + self.recovery_period);
                        state.errors = 0;
                        return Err(CircuitBreakerError::Execution(err));
                    }
                }
            }
        } else {
            match f() {
                Ok(r) => {
                    let state = &mut self.ncb_state.lock().unwrap();
                    state.errors = 0;
                    state.open_until = None;
                    return Ok(r);
                }
                Err(err) => {
                    // Failed in closed. Check what to do.
                    if state.errors < self.max_errors {
                        // Still fine
                        let state = &mut self.ncb_state.lock().unwrap();
                        state.errors += 1;
                        state.open_until = None;
                    } else {
                        // Open now...
                        let state = &mut self.ncb_state.lock().unwrap();
                        state.errors = 0;
                        state.open_until = Some(Instant::now() + self.recovery_period);
                    }
                    return Err(CircuitBreakerError::Execution(err));
                }
            }
        }
    }

    pub fn state(&self) -> State {
        let state = self.ncb_state.lock().unwrap();
        if let Some(until) = state.open_until {
            if until > Instant::now() {
                State::Open
            } else {
                State::HalfOpen
            }
        } else {
            State::Closed
        }
    }
}
