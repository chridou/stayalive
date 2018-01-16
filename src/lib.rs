extern crate chan;

pub mod bulkheads {

    pub type BulkheadResult<T,E> = Result<T, BulkheadError<E>>;

    pub enum BulkheadError<E> {
        Task(E),
        TimedOut,
        TooManyJobs(usize),
        Closed
    }

    mod executor {
        use std::sync::{Arc, Mutex};
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::time::{Duration, Instant};
        use std::sync::mpsc;

        use chan;
   
        use super::{BulkheadError, BulkheadResult};
        
        type Task<T,E> = Box<FnOnce() -> Result<T,E> + Send + 'static>;

        pub struct BulkheadInitializationError(String);

        #[derive(Clone, Debug)]
        pub struct BulkheadExecutorConfig {

        }

        struct DispatchTask<T,E> {
            task: Task<T,E>,
            deadline: Option<Instant>,
            back_channel: mpsc::Sender<BulkheadResult<T,E>>
        }

        #[derive(Clone)]
        pub struct BulkheadExecutor<T: Send + 'static + Sized,E: Send+'static + Sized> {
            num_inflight: Arc<Mutex<usize>>,
            default_timeout: Option<Duration>,
            sender: chan::Sender<Task<T, E>>
        }

        impl<T: Send + 'static, E: Send + 'static> BulkheadExecutor<T,E> {
            pub fn new(config: &BulkheadExecutorConfig) -> Result<BulkheadExecutor<T, E>, BulkheadInitializationError> {
                unimplemented!()
            }

            pub fn execute<F>(&self, task: F) -> BulkheadResult<T,E> where F: Fn() -> Result<T,E> {
                self.execute_internal(task, self.default_timeout.map(|timeout| Instant::now() + timeout))
            } 

            pub fn execute_with_timeout<F>(&self, task: F, timeout: Duration) -> BulkheadResult<T,E> where F: Fn() -> Result<T,E> {
                self.execute_internal(task, Some(Instant::now() + timeout))
            } 

            pub fn execute_with_deadline<F>(&self, task: F, deadline: Instant) -> BulkheadResult<T,E> where F: Fn() -> Result<T,E> {
                self.execute_internal(task, Some(deadline))
            } 

            fn execute_internal<F>(&self, task: F, deadline: Option<Instant>) -> BulkheadResult<T,E> where F: Fn() -> Result<T,E> {
                unimplemented!()
            }            
        }

        struct Worker;

        impl Worker {
            pub fn new<T, E>(receiver: chan::Receiver<DispatchTask<T,E>>) -> Worker {
                unimplemented!()
            }
        }

        fn worker_loop<T,E>(
            receiver: chan::Receiver<DispatchTask<T,E>>,
            is_aborted: Arc<AtomicBool>) {

            loop {
                let to_execute = if let Some(next) = receiver.recv() {
                    next
                } else {
                    // The channel does not exist anymore...
                    break;
                };

                if to_execute.deadline.map(|dl| dl < Instant::now()).unwrap_or(false) {
                    if let Ok(_) = to_execute.back_channel.send(Err(BulkheadError::TimedOut)) {
                        continue
                       } else {
                            // The sender hung up. Exit. 
                        continue;
                        }
                    }

                let to_send = match (to_execute.task)() {
                    Ok(t) => Ok(t),
                    Err(err) =>  Err(BulkheadError::Task(err)),
                };


            }

            }
    }

}