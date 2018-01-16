pub mod bulkheads {

    pub type BulkheadResult<T,E> = Result<T, BulkheadError<E>>;

    pub enum BulkheadError<E> {
        Task(E),
        TimedOut,
        TooManyJobs,
        Closed
    }

    mod executor {
        use std::sync::{Arc, Mutex};
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

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
            num_inflight: Arc<Mutex<u64>>,
            default_timeout: Option<Duration>,
            sender: mpsc::Sender<Task<T, E>>
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


        fn run_loop() {

        }
    }





}