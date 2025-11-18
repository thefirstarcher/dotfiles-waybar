use crate::Result;
use std::time::Duration;
use tokio::time::sleep;

/// Retry strategy with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryStrategy {
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    multiplier: f64,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
        }
    }
}

impl RetryStrategy {
    /// Create a new retry strategy with specified attempts
    pub fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            ..Default::default()
        }
    }

    /// Set initial delay
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Set maximum delay
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Set backoff multiplier
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// Calculate delay for attempt number
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_ms = self.initial_delay.as_millis() as f64
            * self.multiplier.powi(attempt as i32);

        let delay = Duration::from_millis(delay_ms as u64);

        delay.min(self.max_delay)
    }

    /// Execute function with retry logic (async)
    pub async fn execute_async<F, Fut, T>(&self, mut f: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        for attempt in 0..self.max_attempts {
            match f().await {
                Ok(value) => {
                    if attempt > 0 {
                        tracing::info!("Operation succeeded after {} attempts", attempt + 1);
                    }
                    return Ok(value);
                }
                Err(e) => {
                    tracing::warn!("Attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);

                    if attempt + 1 < self.max_attempts {
                        let delay = self.calculate_delay(attempt);
                        tracing::debug!("Retrying in {:?}", delay);
                        sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    /// Execute function with retry logic (sync)
    pub fn execute<F, T>(&self, mut f: F) -> Result<T>
    where
        F: FnMut() -> Result<T>,
    {
        let mut last_error = None;

        for attempt in 0..self.max_attempts {
            match f() {
                Ok(value) => {
                    if attempt > 0 {
                        tracing::info!("Operation succeeded after {} attempts", attempt + 1);
                    }
                    return Ok(value);
                }
                Err(e) => {
                    tracing::warn!("Attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);

                    if attempt + 1 < self.max_attempts {
                        let delay = self.calculate_delay(attempt);
                        tracing::debug!("Retrying in {:?}", delay);
                        std::thread::sleep(delay);
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }
}

/// Convenience macro for retry with default strategy
#[macro_export]
macro_rules! retry {
    ($expr:expr) => {{
        use $crate::retry::RetryStrategy;
        RetryStrategy::default().execute(|| $expr)
    }};

    ($attempts:expr, $expr:expr) => {{
        use $crate::retry::RetryStrategy;
        RetryStrategy::new($attempts).execute(|| $expr)
    }};
}

/// Convenience macro for async retry with default strategy
#[macro_export]
macro_rules! retry_async {
    ($expr:expr) => {{
        use $crate::retry::RetryStrategy;
        RetryStrategy::default().execute_async(|| async { $expr }).await
    }};

    ($attempts:expr, $expr:expr) => {{
        use $crate::retry::RetryStrategy;
        RetryStrategy::new($attempts).execute_async(|| async { $expr }).await
    }};
}
