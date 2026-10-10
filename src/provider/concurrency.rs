use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, thiserror::Error)]
pub enum ConcurrencyError {
    #[error("Timed out waiting for concurrency permit after {0:?}")]
    Timeout(Duration),
    #[error("Concurrency limiter mutex poisoned")]
    Poisoned,
}

/// A thread-safe bounded concurrency limiter that manages maximum concurrent operations.
#[derive(Debug, Clone)]
pub struct ConcurrencyLimiter {
    inner: Arc<LimiterInner>,
}

#[derive(Debug)]
struct LimiterInner {
    max_permits: usize,
    active_count: Mutex<usize>,
    condvar: Condvar,
}

impl ConcurrencyLimiter {
    pub fn new(max_permits: usize) -> Self {
        assert!(max_permits > 0, "max_permits must be greater than 0");
        Self {
            inner: Arc::new(LimiterInner {
                max_permits,
                active_count: Mutex::new(0),
                condvar: Condvar::new(),
            }),
        }
    }

    pub fn max_permits(&self) -> usize {
        self.inner.max_permits
    }

    pub fn active_permits(&self) -> usize {
        *self
            .inner
            .active_count
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// Acquire a permit, waiting up to `timeout`.
    pub fn acquire(&self, timeout: Duration) -> Result<Permit, ConcurrencyError> {
        let start = Instant::now();
        let mut count = self
            .inner
            .active_count
            .lock()
            .map_err(|_| ConcurrencyError::Poisoned)?;

        while *count >= self.inner.max_permits {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return Err(ConcurrencyError::Timeout(timeout));
            }
            let remaining = timeout - elapsed;
            let (new_count, wait_res) = self
                .inner
                .condvar
                .wait_timeout(count, remaining)
                .map_err(|_| ConcurrencyError::Poisoned)?;
            count = new_count;
            if wait_res.timed_out() && *count >= self.inner.max_permits {
                return Err(ConcurrencyError::Timeout(timeout));
            }
        }

        *count += 1;
        Ok(Permit {
            inner: Arc::clone(&self.inner),
        })
    }

    /// Execute a closure under a bounded concurrency permit.
    pub fn execute_bounded<F, T>(&self, timeout: Duration, f: F) -> Result<T, ConcurrencyError>
    where
        F: FnOnce() -> T,
    {
        let _permit = self.acquire(timeout)?;
        Ok(f())
    }
}

/// An RAII permit that decrements the active count when dropped.
pub struct Permit {
    inner: Arc<LimiterInner>,
}

impl Drop for Permit {
    fn drop(&mut self) {
        if let Ok(mut count) = self.inner.active_count.lock()
            && *count > 0
        {
            *count -= 1;
            self.inner.condvar.notify_one();
        }
    }
}
