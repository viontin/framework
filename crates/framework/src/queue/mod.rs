use std::fmt;

pub trait Job: fmt::Debug + Send + Sync {
    fn handle(self: Box<Self>) -> Result<(), String>;
    fn name(&self) -> &str { std::any::type_name_of_val(self) }
}

pub trait Driver: fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn push(&self, job: Box<dyn Job>) -> Result<(), String>;
    fn later(&self, delay_secs: u64, job: Box<dyn Job>) -> Result<(), String>;
}

#[derive(Debug)]
pub struct SyncQueue;

impl Driver for SyncQueue {
    fn name(&self) -> &str { "sync" }
    fn push(&self, job: Box<dyn Job>) -> Result<(), String> { job.handle() }
    fn later(&self, delay_secs: u64, job: Box<dyn Job>) -> Result<(), String> {
        if delay_secs > 0 {
            // Spawn a thread to wait, then execute
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
    pub fn later(&self, delay: u64, job: impl Job + 'static) -> Result<(), String> { self.driver.later(delay, Box::new(job)) }
}

impl Default for Queue { fn default() -> Self { Queue::new(SyncQueue) } }
