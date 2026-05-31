use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::sync::OnceLock;

pub mod database;
pub use database::DatabaseQueue;

// ── Error Types ──

#[derive(Debug, Clone)]
pub struct JobError(pub String);

impl JobError {
    pub fn new(msg: impl Into<String>) -> Self { JobError(msg.into()) }
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for JobError {
    fn from(s: String) -> Self { JobError(s) }
}

impl From<&str> for JobError {
    fn from(s: &str) -> Self { JobError(s.to_string()) }
}

#[derive(Debug, Clone)]
pub enum QueueError {
    JobFailed(String),
    DriverError(String),
}

impl QueueError {
    pub fn driver(msg: impl Into<String>) -> Self { QueueError::DriverError(msg.into()) }
    pub fn job_failed(msg: impl Into<String>) -> Self { QueueError::JobFailed(msg.into()) }
}

impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueueError::JobFailed(m) => write!(f, "job failed: {}", m),
            QueueError::DriverError(m) => write!(f, "driver error: {}", m),
        }
    }
}

impl From<String> for QueueError {
    fn from(s: String) -> Self { QueueError::DriverError(s) }
}

impl From<&str> for QueueError {
    fn from(s: &str) -> Self { QueueError::DriverError(s.to_string()) }
}

// ── Job Registry ──

static JOB_REGISTRY: OnceLock<Mutex<HashMap<String, Box<dyn JobFactory + Send + Sync>>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<String, Box<dyn JobFactory + Send + Sync>>> {
    JOB_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub trait JobFactory: Send + Sync {
    fn create(&self, payload: &str) -> Box<dyn Job>;
}

pub struct SimpleJobFactory<F: Fn(&str) -> Box<dyn Job> + Send + Sync>(F);

impl<F: Fn(&str) -> Box<dyn Job> + Send + Sync> SimpleJobFactory<F> {
    pub fn new(f: F) -> Self { SimpleJobFactory(f) }
}

impl<F: Fn(&str) -> Box<dyn Job> + Send + Sync> JobFactory for SimpleJobFactory<F> {
    fn create(&self, payload: &str) -> Box<dyn Job> {
        (self.0)(payload)
    }
}

pub fn register_job(name: &str, factory: impl JobFactory + 'static) {
    if let Ok(mut reg) = registry().lock() {
        reg.insert(name.to_string(), Box::new(factory));
    }
}

pub fn make_job(name: &str, payload: &str) -> Option<Box<dyn Job>> {
    registry().lock().ok().and_then(|reg| {
        reg.get(name).map(|f| f.create(payload))
    })
}

pub trait Job: fmt::Debug + Send + Sync {
    fn handle(self: Box<Self>) -> Result<(), JobError>;
    fn name(&self) -> &str { std::any::type_name_of_val(self) }
    fn retries(&self) -> u8 { 0 }
    fn retry_delay(&self) -> u64 { 5 }
}

pub trait Driver: fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn push(&self, job: Box<dyn Job>) -> Result<(), QueueError>;
    fn schedule(&self, delay_secs: u64, job: Box<dyn Job>) -> Result<(), QueueError>;
    fn pop(&self) -> Option<Box<dyn Job>> { None }
}

#[derive(Debug)]
pub struct SyncQueue;

impl Driver for SyncQueue {
    fn name(&self) -> &str { "sync" }
    fn push(&self, job: Box<dyn Job>) -> Result<(), QueueError> {
        job.handle().map_err(|e| QueueError::JobFailed(e.to_string()))
    }
    fn schedule(&self, delay_secs: u64, job: Box<dyn Job>) -> Result<(), QueueError> {
        if delay_secs > 0 {
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(delay_secs));
                let _ = job.handle();
            });
            Ok(())
        } else {
            job.handle().map_err(|e| QueueError::JobFailed(e.to_string()))
        }
    }
}

#[derive(Debug)]
pub struct Queue { driver: Box<dyn Driver>, }

impl Queue {
    pub fn new(driver: impl Driver + 'static) -> Self { Queue { driver: Box::new(driver) } }
    pub fn driver(&self) -> &dyn Driver { self.driver.as_ref() }
    pub fn push(&self, job: impl Job + 'static) -> Result<(), QueueError> { self.driver.push(Box::new(job)) }
    pub fn schedule(&self, delay: u64, job: impl Job + 'static) -> Result<(), QueueError> { self.driver.schedule(delay, Box::new(job)) }
}

impl Default for Queue { fn default() -> Self { Queue::new(SyncQueue) } }

impl Queue {
    pub fn sync() -> Self { Queue::new(SyncQueue) }
    pub fn database(conn: Box<dyn crate::db::Connection>) -> Self { Queue::new(DatabaseQueue::new(conn)) }
}

// ── Queue Worker ──

pub struct QueueWorker {
    queue: Arc<Queue>,
    running: Arc<AtomicBool>,
    max_tries: u8,
    sleep_millis: u64,
    timeout_secs: Option<u64>,
}

impl fmt::Debug for QueueWorker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QueueWorker")
            .field("running", &self.running.load(Ordering::Relaxed))
            .field("max_tries", &self.max_tries)
            .field("sleep_millis", &self.sleep_millis)
            .finish()
    }
}

impl QueueWorker {
    pub fn new(queue: Queue) -> Self {
        QueueWorker {
            queue: Arc::new(queue),
            running: Arc::new(AtomicBool::new(false)),
            max_tries: 1,
            sleep_millis: 1000,
            timeout_secs: None,
        }
    }

    pub fn queue(&self) -> &Queue { &self.queue }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn with_max_tries(mut self, n: u8) -> Self {
        self.max_tries = n;
        self
    }

    pub fn with_sleep(mut self, millis: u64) -> Self {
        self.sleep_millis = millis;
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// Start the worker loop. Blocks the current thread.
    /// Pops and processes jobs from the queue indefinitely.
    pub fn run(&self) {
        self.running.store(true, Ordering::SeqCst);
        println!("  Queue worker started");
        let start = std::time::Instant::now();
        while self.running.load(Ordering::SeqCst) {
            if let Some(timeout) = self.timeout_secs {
                if start.elapsed() >= std::time::Duration::from_secs(timeout) {
                    break;
                }
            }
            match self.queue.driver().pop() {
                Some(job) => {
                    let name = job.name().to_string();
                    if let Err(e) = job.handle() {
                        eprintln!("  Queue job '{}' failed: {}", name, e);
                    }
                }
                None => {
                    std::thread::sleep(std::time::Duration::from_millis(self.sleep_millis));
                }
            }
        }
        println!("  Queue worker stopped");
    }

    /// Process a single job then exit.
    pub fn run_once(&self) {
        self.running.store(true, Ordering::SeqCst);
        match self.queue.driver().pop() {
            Some(job) => {
                let name = job.name().to_string();
                if let Err(e) = job.handle() {
                    eprintln!("  Queue job '{}' failed: {}", name, e);
                }
            }
            None => {
                println!("  No pending jobs");
            }
        }
        self.running.store(false, Ordering::SeqCst);
    }

    /// Run the worker in a background thread. Returns the handle.
    pub fn run_background(&self) -> std::thread::JoinHandle<()> {
        let running = self.running.clone();
        let queue = self.queue.clone();
        let sleep_millis = self.sleep_millis;
        std::thread::spawn(move || {
            running.store(true, Ordering::SeqCst);
            while running.load(Ordering::SeqCst) {
                match queue.driver().pop() {
                    Some(job) => {
                        let _ = job.handle();
                    }
                    None => {
                        std::thread::sleep(std::time::Duration::from_millis(sleep_millis));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestJob(&'static str, Result<(), JobError>);
    impl Job for TestJob {
        fn handle(self: Box<Self>) -> Result<(), JobError> { self.1.clone() }
        fn name(&self) -> &str { self.0 }
    }

    fn static_str(s: &str) -> &'static str {
        Box::leak(s.to_string().into_boxed_str())
    }

    #[test]
    fn test_sync_queue_push_success() {
        let queue = Queue::sync();
        assert!(queue.push(TestJob("ok", Ok(()))).is_ok());
    }

    #[test]
    fn test_sync_queue_push_failure() {
        let queue = Queue::sync();
        let result = queue.push(TestJob("fail", Err(JobError("oops".into()))));
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("oops"));
    }

    #[test]
    fn test_sync_queue_schedule_zero_delay() {
        let queue = Queue::sync();
        assert!(queue.schedule(0, TestJob("ok", Ok(()))).is_ok());
    }

    #[test]
    fn test_job_registry_roundtrip() {
        let name = static_str("test-job");
        register_job(name, SimpleJobFactory::new(|_| Box::new(TestJob(name, Ok(())))));
        let job = make_job(name, "");
        assert!(job.is_some());
        assert_eq!(job.unwrap().name(), name);
    }

    #[test]
    fn test_job_registry_unknown_name() {
        let job = make_job("does-not-exist", "");
        assert!(job.is_none());
    }

    #[test]
    fn test_simple_job_factory() {
        let factory = SimpleJobFactory::new(|_| {
            Box::new(TestJob("from-factory", Ok(())))
        });
        let job = factory.create("");
        assert_eq!(job.name(), "from-factory");
    }

    #[test]
    fn test_worker_run_once_with_job() {
        let queue = Queue::sync();
        assert!(queue.push(TestJob("once", Ok(()))).is_ok());
        let worker = QueueWorker::new(queue);
        worker.run_once();
    }

    #[test]
    fn test_worker_run_once_empty() {
        let queue = Queue::sync();
        let worker = QueueWorker::new(queue);
        worker.run_once();
    }

    #[test]
    fn test_worker_stop() {
        let queue = Queue::sync();
        let worker = QueueWorker::new(queue);
        assert!(!worker.is_running());
        worker.run_background();
        worker.stop();
    }

    #[test]
    fn test_queue_default_is_sync() {
        let queue = Queue::default();
        assert_eq!(queue.driver().name(), "sync");
    }

    #[test]
    fn test_worker_config() {
        let queue = Queue::sync();
        let worker = QueueWorker::new(queue)
            .with_max_tries(3)
            .with_sleep(500)
            .with_timeout(10);
        worker.run_once();
    }

    #[test]
    fn test_job_retries_default() {
        let job = TestJob("default", Ok(()));
        assert_eq!(job.retries(), 0);
        assert_eq!(job.retry_delay(), 5);
    }

    #[derive(Debug)]
    struct RetryJob;
    impl Job for RetryJob {
        fn handle(self: Box<Self>) -> Result<(), JobError> { Err(JobError("retry me".into())) }
        fn name(&self) -> &str { "retry-job" }
        fn retries(&self) -> u8 { 3 }
        fn retry_delay(&self) -> u64 { 10 }
    }

    #[test]
    fn test_job_custom_retries() {
        let job = RetryJob;
        assert_eq!(job.retries(), 3);
        assert_eq!(job.retry_delay(), 10);
    }

    #[test]
    fn test_job_error_display() {
        let err = JobError("something went wrong".into());
        assert_eq!(format!("{}", err), "something went wrong");
    }

    #[test]
    fn test_queue_error_display() {
        let err = QueueError::driver("connection lost");
        assert!(format!("{}", err).contains("connection lost"));
        let failed = QueueError::job_failed("timeout");
        assert!(format!("{}", failed).contains("timeout"));
    }
}
