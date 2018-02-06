//! Bulkheads to limit the concurrents accesses to a shared resource
//!
//! # When to use
//!
//! You can use this patterns if you have a resource that can be
//! concurrently accessed but you want to achieve one or more
//! of these goals:
//!
//! * The resource itself is limited. E.g. it can only handle
//! a certain amount of operations at a time.
//! * You want to protect downstream components or services
//!
//! # What you should know
//!
//! Using the components in this module has a great performance impact.
//!
//! Furthermore the resource managed by this bulkhead has no knowledge
//! about the number of threads it is accessed from.
//! So your managed resource should be
//! able to handle the number of threads you access it from.
//!
//! The components in this module check the timeout before
//! a task is executed and via `rec_timeout` on the blocked thread. That
//! means that even though a timeout error occured a running task
//! might still being executed. This makes it mandatory that you also set
//! timeouts(e.g. request timeouts) matching your requirements.

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::sync::mpsc;

use threadpool::{Builder, ThreadPool};

use super::{BulkheadError, BulkheadResult};

pub struct Config {
    /// The number of worker threads to use which is also
    /// the maximum number of jobs executed on the
    /// resource concurrently
    pub num_threads: usize,
    /// The maximum number of jobs that may be queued for execution
    pub max_queued: usize,
    /// The stack size for the workers
    pub thread_stack_size: Option<usize>,
    /// The name of all the workers
    pub thread_name: Option<String>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            num_threads: 4,
            max_queued: 100,
            thread_stack_size: None,
            thread_name: None,
        }
    }
}

impl Config {
    pub fn with_num_threads(mut self, n: usize) -> Self {
        self.num_threads = n;
        self
    }

    pub fn with_max_queued(mut self, n: usize) -> Self {
        self.max_queued = n;
        self
    }

    pub fn with_thread_stack_size(mut self, n: usize) -> Self {
        self.thread_stack_size = Some(n);
        self
    }

    pub fn with_thread_name<T: Into<String>>(mut self, name: T) -> Self {
        self.thread_name = Some(name.into());
        self
    }
}

/// Provides a shared resource
pub trait SharedResourceProvider {
    type Resource;
    /// Get a resource. Returns `None` if no resource could be
    /// aquired
    fn get_resource(&self) -> Option<Self::Resource>;
}

/// Limits the number of concurent accesses on a resource
///
/// The resource is accessed via a function that is gets the resource as
/// a parameter.
pub struct ConcurrencyLimiter<P> {
    pool: ThreadPool,
    resource_provider: Arc<P>,
    max_queued: usize,
}

impl<P> ConcurrencyLimiter<P>
where
    P: SharedResourceProvider + Send + Sync + 'static,
{
    /// Create a new `ConcurrencyLimiter` given a
    /// `ResourceProvider`.
    pub fn new(resource_provider: P, config: Config) -> Result<ConcurrencyLimiter<P>, String> {
        if config.num_threads == 0 {
            return Err("'n_workers' must be greater than zero.".to_string());
        }

        let builder = Builder::new().num_threads(config.num_threads);
        let builder = if let Some(name) = config.thread_name {
            builder.thread_name(name)
        } else {
            builder
        };
        let builder = if let Some(stack_size) = config.thread_stack_size {
            builder.thread_stack_size(stack_size)
        } else {
            builder
        };

        let pool = builder.build();

        Ok(ConcurrencyLimiter {
            pool,
            resource_provider: Arc::new(resource_provider),
            max_queued: config.max_queued,
        })
    }

    /// Execute the given closure. Wait at most for the given `Duration`.
    ///
    /// # Errors
    ///
    /// * `BulkheadError::TimedOut` if the function could not be executet in time
    /// * `BulkheadError::Task` if the execution of the function itself returned an error
    /// * `BulkheadError::TaskLimitReached` if the maximum number of enqueued tasks was already
    /// reached
    /// * `BulkheadError::ResourceAcquisition` if the resource could not be provided
    /// for the function
    pub fn execute<T, E, F>(&self, f: F, timeout: Duration) -> BulkheadResult<T, E>
    where
        T: Send + 'static,
        E: Send + 'static,
        F: FnOnce(P::Resource) -> Result<T, E> + Send + 'static,
    {
        let jobs = self.pool.queued_count();
        if self.pool.queued_count() > self.max_queued {
            return Err(BulkheadError::TaskLimitReached(jobs));
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
                Err(BulkheadError::ResourceAcquisition)
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

    /// Execute the given closure. Wait at most until the deadline is met.
    ///
    /// # Errors
    ///
    /// * `BulkheadError::TimedOut` if the function could not be executet in time
    /// * `BulkheadError::Task` if the execution of the function itself returned an error
    /// * `BulkheadError::TaskLimitReached` if the maximum number of enqueued tasks was already
    /// reached
    /// * `BulkheadError::ResourceAcquisition` if the resource could not be provided
    /// for the function
    pub fn execute_until<T, E, F>(&self, f: F, deadline: Instant) -> BulkheadResult<T, E>
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

    /// Returns the number of jobs that are currently enqueued
    pub fn queued_count(&self) -> usize {
        self.pool.queued_count()
    }

    /// Returns the number of jobs currently being executed
    pub fn active_count(&self) -> usize {
        self.pool.active_count()
    }
}

impl<P> Clone for ConcurrencyLimiter<P> {
    fn clone(&self) -> Self {
        ConcurrencyLimiter {
            pool: self.pool.clone(),
            resource_provider: self.resource_provider.clone(),
            max_queued: self.max_queued,
        }
    }
}

/// Executes a command sent to the managed resource
///
/// Useful if you have a resource that always processes the same input
/// type and always returns the same output type. E.g. an HTTP Client that
/// always takes a request and always returns a response or a database driver
/// that always takes some SQL and always returns rows.
///
/// `CmdConcurrencyLimiter` can also execute closures in the
/// same way as `ConcurrencyLimiter` does.
#[derive(Clone)]
pub struct CmdConcurrencyLimiter<P: SharedResourceProvider, C, T, E> {
    limiter: ConcurrencyLimiter<P>,
    exec_cmd: Arc<Fn(C, P::Resource) -> Result<T, E> + Send + Sync + 'static>,
}

impl<P: SharedResourceProvider, C, T, E> CmdConcurrencyLimiter<P, C, T, E>
where
    C: Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
    P: SharedResourceProvider + Send + Sync + 'static,
{
    /// Create a new `CmdConcurrencyLimiter` given a
    /// `ResourceProvider` and a function that executes
    /// a command on the resource and returns the result.
    pub fn new<F>(
        resource_provider: P,
        exec_cmd: F,
        config: Config,
    ) -> Result<CmdConcurrencyLimiter<P, C, T, E>, String>
    where
        F: Fn(C, P::Resource) -> Result<T, E> + Send + Sync + 'static,
    {
        let limiter = ConcurrencyLimiter::new(resource_provider, config)?;

        Ok(CmdConcurrencyLimiter {
            limiter,
            exec_cmd: Arc::new(exec_cmd),
        })
    }

    /// Execute the given command. Wait at most for the given `Duration`.
    ///
    /// # Errors
    ///
    /// * `BulkheadError::TimedOut` if the function could not be executet in time
    /// * `BulkheadError::Task` if the execution of the function itself returned an error
    /// * `BulkheadError::TaskLimitReached` if the maximum number of enqueued tasks was already
    /// reached
    /// * `BulkheadError::ResourceAcquisition` if the resource could not be provided
    /// for the function
    pub fn execute_cmd(&self, cmd: C, timeout: Duration) -> BulkheadResult<T, E> {
        let exec = self.exec_cmd.clone();
        let f = move |resource: P::Resource| (exec)(cmd, resource);
        self.limiter.execute(f, timeout)
    }

    /// Execute the given closure. Wait at most until the deadline is met.
    ///
    /// # Errors
    ///
    /// * `BulkheadError::TimedOut` if the function could not be executet in time
    /// * `BulkheadError::Task` if the execution of the function itself returned an error
    /// * `BulkheadError::TaskLimitReached` if the maximum number of enqueued tasks was already
    /// reached
    /// * `BulkheadError::ResourceAcquisition` if the resource could not be provided
    /// for the function
    pub fn execute_cmd_until(&self, cmd: C, deadline: Instant) -> BulkheadResult<T, E> {
        let exec = self.exec_cmd.clone();
        let f = move |resource: P::Resource| (exec)(cmd, resource);
        self.limiter.execute_until(f, deadline)
    }

    /// Execute the given closure. Wait at most for the given `Duration`.
    ///
    /// # Errors
    ///
    /// * `BulkheadError::TimedOut` if the function could not be executet in time
    /// * `BulkheadError::Task` if the execution of the function itself returned an error
    /// * `BulkheadError::TaskLimitReached` if the maximum number of enqueued tasks was already
    /// reached
    /// * `BulkheadError::ResourceAcquisition` if the resource could not be provided
    /// for the function
    pub fn execute<TT, EE, FF>(&self, f: FF, timeout: Duration) -> BulkheadResult<TT, EE>
    where
        TT: Send + 'static,
        EE: Send + 'static,
        FF: Fn(P::Resource) -> Result<TT, EE> + Send + 'static,
    {
        self.limiter.execute(f, timeout)
    }

    /// Execute the given closure. Wait at most until the deadline is met.
    ///
    /// # Errors
    ///
    /// * `BulkheadError::TimedOut` if the function could not be executet in time
    /// * `BulkheadError::Task` if the execution of the function itself returned an error
    /// * `BulkheadError::TaskLimitReached` if the maximum number of enqueued tasks was already
    /// reached
    /// * `BulkheadError::ResourceAcquisition` if the resource could not be provided
    /// for the function
    pub fn execute_until<TT, EE, FF>(&self, f: FF, deadline: Instant) -> BulkheadResult<TT, EE>
    where
        TT: Send + 'static,
        EE: Send + 'static,
        FF: FnOnce(P::Resource) -> Result<TT, EE> + Send + 'static,
    {
        self.limiter.execute_until(f, deadline)
    }

    /// Returns the number of jobs that are currently enqueued
    pub fn queued_count(&self) -> usize {
        self.limiter.queued_count()
    }

    /// Returns the number of jobs currently being executed
    pub fn active_count(&self) -> usize {
        self.limiter.active_count()
    }
}

#[cfg(test)]
mod concurrency_limiter_tests {
    use super::*;

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

        let config = Config::default().with_num_threads(2).with_max_queued(2);

        let limiter = ConcurrencyLimiter::new(provider, config).unwrap();

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
    fn should_work_with_commands_from_a_single_thread() {
        let n = 100;

        let provider = AdderProvider {
            resource: Adder { to_add: 42 },
        };

        let config = Config::default().with_num_threads(2).with_max_queued(2);
        let limiter =
            CmdConcurrencyLimiter::new(provider, |cmd, adder| adder.add(cmd), config).unwrap();

        let counter = Arc::new(AtomicUsize::new(0));

        for i in 0..n {
            let x = limiter.execute_cmd(i, Duration::from_millis(10)).unwrap();
            let _ = counter.fetch_add(x, Ordering::SeqCst);
        }

        let count = counter.load(Ordering::SeqCst);

        assert_eq!(count, (n * (n - 1) / 2) + 42 * n);
    }

    #[test]
    fn should_work_from_multiple_threads() {
        let n = 100;
        let n_threads = 10;

        let provider = AdderProvider {
            resource: Adder { to_add: 42 },
        };

        let config = Config::default().with_num_threads(10).with_max_queued(100);

        let limiter = ConcurrencyLimiter::new(provider, config).unwrap();

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
