use std::fmt;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

pub trait Job: fmt::Debug + Send + Sync {
    fn handle(self: Box<Self>) -> Result<(), String>;
    fn name(&self) -> &str { std::any::type_name_of_val(self) }
}

pub trait Driver: fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn push(&self, job: Box<dyn Job>) -> Result<(), String>;
    fn schedule(&self, delay_secs: u64, job: Box<dyn Job>) -> Result<(), String>;
    fn pop(&self) -> Option<Box<dyn Job>> { None }
}

#[derive(Debug)]
pub struct SyncQueue;

impl Driver for SyncQueue {
    fn name(&self) -> &str { "sync" }
    fn push(&self, job: Box<dyn Job>) -> Result<(), String> { job.handle() }
    fn schedule(&self, delay_secs: u64, job: Box<dyn Job>) -> Result<(), String> {
        if delay_secs > 0 {
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(delay_secs));
                let _ = job.handle();
            });
            Ok(())
        } else {
            job.handle()
        }
    }
}

#[derive(Debug)]
pub struct Queue { driver: Box<dyn Driver>, }

impl Queue {
    pub fn new(driver: impl Driver + 'static) -> Self { Queue { driver: Box::new(driver) } }
    pub fn driver(&self) -> &dyn Driver { self.driver.as_ref() }
    pub fn push(&self, job: impl Job + 'static) -> Result<(), String> { self.driver.push(Box::new(job)) }
    pub fn schedule(&self, delay: u64, job: impl Job + 'static) -> Result<(), String> { self.driver.schedule(delay, Box::new(job)) }
}

impl Default for Queue { fn default() -> Self { Queue::new(SyncQueue) } }

// ── Queue Worker ──

pub struct QueueWorker {
    queue: Arc<Queue>,
    running: Arc<AtomicBool>,
}

impl fmt::Debug for QueueWorker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QueueWorker").field("running", &self.running.load(Ordering::Relaxed)).finish()
    }
}

impl QueueWorker {
    pub fn new(queue: Queue) -> Self {
        QueueWorker {
            queue: Arc::new(queue),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn queue(&self) -> &Queue { &self.queue }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Start the worker loop. Blocks the current thread.
    /// Pops and processes jobs from the queue indefinitely.
    pub fn run(&self) {
        self.running.store(true, Ordering::SeqCst);
        println!("  Queue worker started");
        while self.running.load(Ordering::SeqCst) {
            match self.queue.driver().pop() {
                Some(job) => {
                    let name = job.name().to_string();
                    if let Err(e) = job.handle() {
                        eprintln!("  Queue job '{}' failed: {}", name, e);
                    }
                }
                None => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
        println!("  Queue worker stopped");
    }

    /// Run the worker in a background thread. Returns the handle.
    pub fn run_background(&self) -> std::thread::JoinHandle<()> {
        let running = self.running.clone();
        let queue = self.queue.clone();
        std::thread::spawn(move || {
            running.store(true, Ordering::SeqCst);
            while running.load(Ordering::SeqCst) {
                match queue.driver().pop() {
                    Some(job) => {
                        let _ = job.handle();
                    }
                    None => {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        })
    }

    /// Signal the worker to stop.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}
