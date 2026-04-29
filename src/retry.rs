use crate::client::CassandraClient;
use crate::error::Result;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::warn;

/// Retry policy for failed operations
pub struct RetryPolicy {
    max_retries: usize,
    retry_delay_ms: u64,
    exponential_backoff: bool,
}

impl RetryPolicy {
    pub fn new(max_retries: usize, retry_delay_ms: u64) -> Self {
        Self {
            max_retries,
            retry_delay_ms,
            exponential_backoff: true,
        }
    }
    
    pub async fn execute<F, Fut, T>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut attempts = 0;
        let mut delay = self.retry_delay_ms;
        
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempts += 1;
                    if attempts >= self.max_retries {
                        return Err(e);
                    }
                    
                    warn!("Operation failed (attempt {}/{}): {:?}", attempts, self.max_retries, e);
                    sleep(Duration::from_millis(delay)).await;
                    
                    if self.exponential_backoff {
                        delay *= 2;
                    }
                }
            }
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(3, 100)
    }
}

/// Circuit breaker for fault tolerance
pub struct CircuitBreaker {
    failure_threshold: usize,
    success_threshold: usize,
    timeout_ms: u64,
    failures: std::sync::atomic::AtomicUsize,
    state: std::sync::RwLock<CircuitState>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: usize, success_threshold: usize, timeout_ms: u64) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout_ms,
            failures: std::sync::atomic::AtomicUsize::new(0),
            state: std::sync::RwLock::new(CircuitState::Closed),
        }
    }
    
    pub fn is_open(&self) -> bool {
        let state = self.state.read().unwrap();
        *state == CircuitState::Open
    }
    
    pub fn record_success(&self) {
        self.failures.store(0, std::sync::atomic::Ordering::Relaxed);
        let mut state = self.state.write().unwrap();
        *state = CircuitState::Closed;
    }
    
    pub fn record_failure(&self) {
        let failures = self.failures.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        if failures >= self.failure_threshold {
            let mut state = self.state.write().unwrap();
            *state = CircuitState::Open;
        }
    }
}
