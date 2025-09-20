use crate::error::ApiError;
use backoff::{ExponentialBackoff, backoff::Backoff};
use std::future::Future;
use std::time::Duration;

/// Retry configuration for API operations
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial retry delay
    pub initial_delay: Duration,
    /// Maximum retry delay
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub multiplier: f64,
    /// Whether to retry on client errors (4xx)
    pub retry_client_errors: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
            multiplier: 2.0,
            retry_client_errors: false,
        }
    }
}

impl RetryConfig {
    /// Create a config for aggressive retry (longer delays, more attempts)
    pub fn aggressive() -> Self {
        Self {
            max_retries: 5,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(120),
            multiplier: 2.5,
            retry_client_errors: false,
        }
    }

    /// Create a config for quick retry (shorter delays, fewer attempts)
    pub fn quick() -> Self {
        Self {
            max_retries: 2,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(10),
            multiplier: 1.5,
            retry_client_errors: false,
        }
    }
}

/// Enhanced retry executor with configurable policies
pub struct RetryExecutor {
    config: RetryConfig,
}

impl RetryExecutor {
    /// Create a new retry executor with the given config
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Execute an async operation with retry logic
    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T, ApiError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, ApiError>>,
    {
        let mut backoff = ExponentialBackoff {
            initial_interval: self.config.initial_delay,
            max_interval: self.config.max_delay,
            multiplier: self.config.multiplier,
            max_elapsed_time: None,
            ..Default::default()
        };

        let mut attempt = 0;

        loop {
            attempt += 1;

            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    let should_retry = self.should_retry(&error, attempt);

                    if !should_retry {
                        return Err(error);
                    }

                    if let Some(delay) = backoff.next_backoff() {
                        log::debug!("Retrying operation after {:?} (attempt {})", delay, attempt);
                        tokio::time::sleep(delay).await;
                    } else {
                        log::warn!(
                            "Max retry attempts reached ({}), giving up",
                            self.config.max_retries
                        );
                        return Err(error);
                    }
                }
            }
        }
    }

    /// Determine if an error should trigger a retry
    fn should_retry(&self, error: &ApiError, attempt: u32) -> bool {
        if attempt >= self.config.max_retries {
            return false;
        }

        match error {
            // Always retry on server errors and timeouts
            ApiError::Http {
                status: 500..=599, ..
            } => true,
            ApiError::Timeout { .. } => true,

            // Retry on client errors only if configured
            ApiError::Http {
                status: 400..=499, ..
            } => self.config.retry_client_errors,

            // Don't retry on authentication errors
            ApiError::Unauthorized { .. } => false,

            // Don't retry on other HTTP errors (informational, success, other)
            ApiError::Http { .. } => false,
        }
    }
}

/// Convenience function for quick retry operations
pub async fn with_retry<F, Fut, T>(operation: F) -> Result<T, ApiError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, ApiError>>,
{
    let executor = RetryExecutor::new(RetryConfig::default());
    executor.execute(operation).await
}

/// Convenience function for aggressive retry operations
pub async fn with_aggressive_retry<F, Fut, T>(operation: F) -> Result<T, ApiError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, ApiError>>,
{
    let executor = RetryExecutor::new(RetryConfig::aggressive());
    executor.execute(operation).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_success_immediate() {
        let executor = RetryExecutor::new(RetryConfig::default());

        let result = executor.execute(|| async { Ok::<i32, ApiError>(42) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_gives_up_on_auth_error() {
        let executor = RetryExecutor::new(RetryConfig::default());

        let result: Result<String, ApiError> = executor
            .execute(|| async {
                Err(ApiError::Unauthorized {
                    status: 401,
                    endpoint: "/test".to_string(),
                    server_message: "Unauthorized".to_string(),
                })
            })
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_retry_config_presets() {
        let default = RetryConfig::default();
        assert_eq!(default.max_retries, 3);
        assert_eq!(default.initial_delay, Duration::from_millis(100));

        let aggressive = RetryConfig::aggressive();
        assert_eq!(aggressive.max_retries, 5);
        assert_eq!(aggressive.initial_delay, Duration::from_millis(200));

        let quick = RetryConfig::quick();
        assert_eq!(quick.max_retries, 2);
        assert_eq!(quick.initial_delay, Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_convenience_functions() {
        let result = with_retry(|| async { Ok::<String, ApiError>("success".to_string()) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }
}
