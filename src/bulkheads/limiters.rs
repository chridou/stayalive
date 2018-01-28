//! Bulkheads to limit the access to a shared resource
//!
//! # When to use
//!
//! You can use this patterns if you have a resource that can be
//! concurrently accessed but you want to achieve one
//! of these goals:
//!
//! * You want to protect downstream components or services
//! * The resource itself is limited. E.g. it can only handle
//! a certain amount of operations at a time.
//!
//! # Limitations
//!
//! Using a bulkhead has a great performance impact.

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::sync::mpsc;

use threadpool::ThreadPool;

use super::{BulkheadError, BulkheadResult};

pub struct Config {
    /// The number of worker threads to use which is also
    /// the maximum number of jobs executed on the
    /// resource concurrently
    pub n_workers: usize,
    /// The maximum number of jobs that may be queued for execution
    pub max_queued: usize,
}

/// Provides a shared resource
pub trait SharedResourceProvider {
    type Resource;
    /// Get a resource. Returns `None` if no resource could be
    /// aquired
    fn get_resource(&self) -> Option<Self::Resource>;
}

/// Limits the number of concurent accesses on a resource
pub struct SharedResourceLimiter<P> {
    pool: ThreadPool,
    resource_provider: Arc<P>,
    max_queued: usize,
}

impl<P> SharedResourceLimiter<P>
where
    P: SharedResourceProvider + Send + Sync + 'static,
{
    pub fn new(resource_provider: P, config: Config) -> Result<SharedResourceLimiter<P>, String> {
        if config.n_workers == 0 {
            return Err("'n_workers' must be greater than zero.".to_string());
        }

        let pool = ThreadPool::new(config.n_workers);

        Ok(SharedResourceLimiter {
            pool,
            resource_provider: Arc::new(resource_provider),
            max_queued: config.max_queued,
        })
    }

    pub fn execute_with_deadline<T, E, F>(&self, f: F, deadline: Instant) -> BulkheadResult<T, E>
    where
        T: Send + 'static,
        E: Send + 'static,
        F: FnOnce(P::Resource) -> Result<T, E> + Send + 'static,
    {
        let now = Instant::now();
        if deadline <= now {
            Err(BulkheadError::TimedOut)
        } else {
            self.execute(f, deadline.duration_since(now))
        }
    }

    pub fn execute<T, E, F>(&self, f: F, timeout: Duration) -> BulkheadResult<T, E>
    where
        T: Send + 'static,
        E: Send + 'static,
        F: FnOnce(P::Resource) -> Result<T, E> + Send + 'static,
    {
        let jobs = self.pool.queued_count();
        if self.pool.queued_count() > self.max_queued {
            return Err(BulkheadError::TooManyTasks(jobs));
        }

        let deadline = Instant::now() + timeout;

        let (tx, rx) = mpsc::channel();

        let resource_provider = self.resource_provider.clone();
        let job = move || {
            if deadline <= Instant::now() {
                let _ = tx.send(Err(BulkheadError::TimedOut));
                return;
            }

            let back_msg = if let Some(res) = resource_provider.get_resource() {
                f(res).map_err(BulkheadError::Task)
            } else {
                Err(BulkheadError::ResourceAquisitionFailed)
            };
            let _ = tx.send(back_msg);
        };

        self.pool.execute(job);

        match rx.recv_timeout(timeout) {
            Ok(Ok(r)) => Ok(r),
            Ok(err) => err,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(BulkheadError::TimedOut),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(BulkheadError::TimedOut),
        }
    }

    /// Returns the number og jobs that are currently enqueued
    pub fn queued_count(&self) -> usize {
        self.pool.queued_count()
    }

    /// Returns the number of jobs currently being executed
    pub fn active_count(&self) -> usize {
        self.pool.active_count()
    }
}

impl<P> Clone for SharedResourceLimiter<P> {
    fn clone(&self) -> Self {
        SharedResourceLimiter {
            pool: self.pool.clone(),
            resource_provider: self.resource_provider.clone(),
            max_queued: self.max_queued,
        }
    }
}

#[cfg(test)]
mod shared_resource_limiter_tests {
    use super::*;
    use super::super::*;

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use std::thread;

    #[derive(Clone)]
    struct Adder {
        to_add: usize,
    }

    impl Adder {
        pub fn add(&self, x: usize) -> Result<usize, ()> {
            Ok(self.to_add + x)
        }
    }

    struct AdderProvider {
        resource: Adder,
    }

    impl SharedResourceProvider for AdderProvider {
        type Resource = Adder;

        fn get_resource(&self) -> Option<Adder> {
            Some(self.resource.clone())
        }
    }

    #[test]
    fn should_work_from_a_single_thread() {
        let n = 100;

        let provider = AdderProvider {
            resource: Adder { to_add: 42 },
        };

        let config = Config {
            n_workers: 2,
            max_queued: 2,
        };

        let limiter = SharedResourceLimiter::new(provider, config).unwrap();

        let counter = Arc::new(AtomicUsize::new(0));

        for i in 0..n {
            let f = move |adder: Adder| adder.add(i);
            let x = limiter.execute(f, Duration::from_millis(10)).unwrap();
            let _ = counter.fetch_add(x, Ordering::SeqCst);
        }

        let count = counter.load(Ordering::SeqCst);

        assert_eq!(count, (n * (n - 1) / 2) + 42 * n);
    }

    #[test]
    fn should_work_from_multiple_threads() {
        let n = 1000;
        let n_threads = 10;

        let provider = AdderProvider {
            resource: Adder { to_add: 42 },
        };

        let config = Config {
            n_workers: 10,
            max_queued: 100,
        };

        let limiter = SharedResourceLimiter::new(provider, config).unwrap();

        let counter = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..n_threads {
            let limiter = limiter.clone();
            let counter = counter.clone();
            let handle = thread::spawn(move || {
                for i in 0..n {
                    let f = move |adder: Adder| adder.add(i);
                    match limiter.execute(f, Duration::from_millis(10)) {
                        Ok(x) => {
                            let _ = counter.fetch_add(x, Ordering::SeqCst);
                        }
                        Err(err) => return Err(err),
                    }
                }
                Ok(())
            });
            handles.push(handle);
        }

        for h in handles {
            if let Ok(Err(err)) = h.join() {
                panic!(format!("Something went wrong: {:?}", err));
            }
        }

        let count = counter.load(Ordering::SeqCst);

        let sum_per_thread = (n * (n - 1) / 2) + 42 * n;

        assert_eq!(count, sum_per_thread * n_threads);
    }

}
