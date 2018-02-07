extern crate stayalive;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;

use stayalive::bulkheads::concurrency_limiters::*;

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

fn main() {
    let n_total = 1_000_000;
    let n_threads = 8;
    let n = n_total / n_threads;

    let provider = AdderProvider {
        resource: Adder { to_add: 42 },
    };

    let config = Config::default().with_num_threads(10).with_max_queued(100);

    let limiter = ConcurrencyLimiter::new(provider, config).unwrap();

    let calculated_result = Arc::new(AtomicUsize::new(0));
    let counter = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();

    let mut handles = Vec::new();
    for _ in 0..n_threads {
        let limiter = limiter.clone();
        let calculated_result = calculated_result.clone();
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            for i in 0..n {
                let f = move |adder: Adder| adder.add(i);
                match limiter.execute(f, Duration::from_millis(1_000)) {
                    Ok(x) => {
                        let _ = calculated_result.fetch_add(x, Ordering::SeqCst);
                        let _ = counter.fetch_add(1, Ordering::SeqCst);
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

    println!("Took {:?}", start.elapsed());

    let calculated = calculated_result.load(Ordering::SeqCst);
    let count = counter.load(Ordering::SeqCst);

    println!("Calculated {}", calculated);
    println!("Count {}", count);

    let sum_per_thread = (n * (n - 1) / 2) + 42 * n;

    assert_eq!(calculated, sum_per_thread * n_threads);
}
