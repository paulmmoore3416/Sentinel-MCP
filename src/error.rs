//! Comprehensive error handling module
//! 
//! Provides structured error types, retry logic with exponential backoff,
//! and circuit breaker pattern for resilient operations.

use std::fmt;
use std::time::Duration;
use thiserror::Error;

/// Main error type for Sentinel-MCP
#[derive(Error, Debug)]
pub enum SentinelError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("MCP tool error: {0}")]
    McpTool(String),
    
    #[error("Watsonx API error: {0}")]
    WatsonxApi(String),
    
    #[error("Kubernetes error: {0}")]
    Kubernetes(String),
    
    #[error("System command error: {0}")]
    SystemCommand(String),
    
    #[error("Alert processing error: {0}")]
    AlertProcessing(String),
    
    #[error("Remediation execution error: {0}")]
    RemediationExecution(String),
    
    #[error("Security validation error: {0}")]
    SecurityValidation(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Timeout error: operation took longer than {0:?}")]
    Timeout(Duration),
    
    #[error("Circuit breaker open: {0}")]
    CircuitBreakerOpen(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, SentinelError>;

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a retry config for critical operations
    pub fn critical() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(60),
            multiplier: 2.0,
            jitter: true,
        }
    }
    
    /// Create a retry config for non-critical operations
    pub fn non_critical() -> Self {
        Self {
            max_attempts: 2,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(10),
            multiplier: 1.5,
            jitter: false,
        }
    }
}

/// Retry a fallible async operation with exponential backoff
pub async fn retry_with_backoff<F, Fut, T, E>(
    config: RetryConfig,
    mut operation: F,
) -> std::result::Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<T, E>>,
    E: fmt::Display,
{
    let mut attempt = 0;
    let mut delay = config.initial_delay;
    
    loop {
        attempt += 1;
        
        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    tracing::info!("Operation succeeded after {} attempts", attempt);
                }
                return Ok(result);
            }
            Err(e) if attempt >= config.max_attempts => {
                tracing::error!(
                    "Operation failed after {} attempts: {}",
                    config.max_attempts,
                    e
                );
                return Err(e);
            }
            Err(e) => {
                tracing::warn!(
                    "Operation failed (attempt {}/{}): {}. Retrying in {:?}",
                    attempt,
                    config.max_attempts,
                    e,
                    delay
                );
                
                // Apply jitter if enabled
                let actual_delay = if config.jitter {
                    let jitter = rand::random::<f64>() * 0.3; // ±30% jitter
                    Duration::from_millis(
                        (delay.as_millis() as f64 * (1.0 + jitter - 0.15)) as u64
                    )
                } else {
                    delay
                };
                
                tokio::time::sleep(actual_delay).await;
                
                // Calculate next delay with exponential backoff
                delay = Duration::from_millis(
                    ((delay.as_millis() as f64 * config.multiplier) as u64)
                        .min(config.max_delay.as_millis() as u64)
                );
            }
        }
    }
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker for preventing cascading failures
pub struct CircuitBreaker {
    state: tokio::sync::RwLock<CircuitState>,
    failure_count: tokio::sync::RwLock<u32>,
    last_failure_time: tokio::sync::RwLock<Option<std::time::Instant>>,
    config: CircuitBreakerConfig,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
    pub half_open_timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            half_open_timeout: Duration::from_secs(30),
        }
    }
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: tokio::sync::RwLock::new(CircuitState::Closed),
            failure_count: tokio::sync::RwLock::new(0),
            last_failure_time: tokio::sync::RwLock::new(None),
            config,
        }
    }
    
    /// Execute an operation through the circuit breaker
    pub async fn call<F, Fut, T, E>(&self, operation: F) -> std::result::Result<T, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<T, E>>,
        E: From<SentinelError>,
    {
        // Check if circuit is open
        let state = *self.state.read().await;
        
        match state {
            CircuitState::Open => {
                // Check if timeout has elapsed
                let last_failure = *self.last_failure_time.read().await;
                if let Some(last_time) = last_failure {
                    if last_time.elapsed() >= self.config.timeout {
                        // Transition to half-open
                        *self.state.write().await = CircuitState::HalfOpen;
                        tracing::info!("Circuit breaker transitioning to half-open");
                    } else {
                        return Err(SentinelError::CircuitBreakerOpen(
                            "Circuit breaker is open".to_string()
                        ).into());
                    }
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited requests through
                tracing::debug!("Circuit breaker in half-open state, allowing request");
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }
        
        // Execute the operation
        match operation().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(e)
            }
        }
    }
    
    /// Handle successful operation
    async fn on_success(&self) {
        let state = *self.state.read().await;
        
        match state {
            CircuitState::HalfOpen => {
                // Count successes in half-open state
                let mut count = self.failure_count.write().await;
                *count = count.saturating_sub(1);
                
                if *count == 0 {
                    *self.state.write().await = CircuitState::Closed;
                    tracing::info!("Circuit breaker closed after successful recovery");
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                *self.failure_count.write().await = 0;
            }
            CircuitState::Open => {
                // Should not happen, but reset if it does
                *self.state.write().await = CircuitState::Closed;
            }
        }
    }
    
    /// Handle failed operation
    async fn on_failure(&self) {
        let mut count = self.failure_count.write().await;
        *count += 1;
        
        *self.last_failure_time.write().await = Some(std::time::Instant::now());
        
        if *count >= self.config.failure_threshold {
            *self.state.write().await = CircuitState::Open;
            tracing::error!(
                "Circuit breaker opened after {} failures",
                self.config.failure_threshold
            );
        }
    }
    
    /// Get current circuit state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }
    
    /// Reset the circuit breaker
    pub async fn reset(&self) {
        *self.state.write().await = CircuitState::Closed;
        *self.failure_count.write().await = 0;
        *self.last_failure_time.write().await = None;
        tracing::info!("Circuit breaker manually reset");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_retry_success() {
        let mut attempts = 0;
        let result = retry_with_backoff(
            RetryConfig::default(),
            || async {
                attempts += 1;
                if attempts < 2 {
                    Err("temporary failure")
                } else {
                    Ok("success")
                }
            }
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(attempts, 2);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            timeout: Duration::from_millis(100),
            half_open_timeout: Duration::from_millis(50),
        });
        
        // First failure
        let result1: std::result::Result<(), SentinelError> = cb.call(|| async move {
            Err(SentinelError::Unknown("test".to_string()))
        }).await;
        assert!(result1.is_err());
        assert_eq!(cb.state().await, CircuitState::Closed);
        
        // Second failure - should open circuit
        let result2: std::result::Result<(), SentinelError> = cb.call(|| async move {
            Err(SentinelError::Unknown("test".to_string()))
        }).await;
        assert!(result2.is_err());
        assert_eq!(cb.state().await, CircuitState::Open);
    }
}

// Made with Bob