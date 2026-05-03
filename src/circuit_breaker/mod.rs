//! Circuit breaker module
//!
//! Prevents cascading failures by tracking per-alert-type failure rates
//! and blocking execution when too many consecutive failures occur.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Circuit states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CircuitState {
    /// Normal operation — executions allowed
    Closed,
    /// Too many failures — executions blocked
    Open,
    /// Cooldown elapsed — allow one probe to test recovery
    HalfOpen,
}

/// Thresholds controlling circuit breaker behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before tripping open
    pub failure_threshold: u32,
    /// Consecutive successes in HalfOpen required to close
    pub success_threshold: u32,
    /// Seconds to wait in Open before moving to HalfOpen
    pub timeout_seconds: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_seconds: 300,
        }
    }
}

/// Per-alert-type circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreaker {
    pub alert_type: String,
    pub failure_count: u32,
    pub success_count: u32,
    pub state: CircuitState,
    pub last_failure: Option<DateTime<Utc>>,
    pub last_state_change: DateTime<Utc>,
    pub config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub fn new(alert_type: String, config: CircuitBreakerConfig) -> Self {
        Self {
            alert_type,
            failure_count: 0,
            success_count: 0,
            state: CircuitState::Closed,
            last_failure: None,
            last_state_change: Utc::now(),
            config,
        }
    }

    /// Returns true if execution should be allowed; may transition Open → HalfOpen.
    pub fn should_allow_execution(&mut self) -> bool {
        match self.state {
            CircuitState::Closed | CircuitState::HalfOpen => true,
            CircuitState::Open => {
                let timeout =
                    chrono::Duration::seconds(self.config.timeout_seconds as i64);
                if self.last_failure.map_or(false, |t| Utc::now() - t > timeout) {
                    self.state = CircuitState::HalfOpen;
                    self.success_count = 0;
                    self.last_state_change = Utc::now();
                    tracing::info!(
                        "Circuit breaker '{}' transitioned Open → HalfOpen",
                        self.alert_type
                    );
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Record a successful execution. Returns true if circuit just closed.
    pub fn record_success(&mut self) -> bool {
        match self.state {
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.config.success_threshold {
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                    self.last_state_change = Utc::now();
                    tracing::info!(
                        "Circuit breaker '{}' closed after recovery",
                        self.alert_type
                    );
                    return true;
                }
            }
            CircuitState::Closed => {
                self.failure_count = self.failure_count.saturating_sub(1);
            }
            CircuitState::Open => {}
        }
        false
    }

    /// Record a failed execution. Returns true if circuit just tripped open.
    pub fn record_failure(&mut self) -> bool {
        self.last_failure = Some(Utc::now());

        match self.state {
            CircuitState::Closed | CircuitState::HalfOpen => {
                self.failure_count += 1;
                if self.failure_count >= self.config.failure_threshold {
                    self.state = CircuitState::Open;
                    self.last_state_change = Utc::now();
                    tracing::warn!(
                        "Circuit breaker '{}' TRIPPED after {} failures — blocking further attempts",
                        self.alert_type,
                        self.failure_count
                    );
                    return true;
                }
            }
            CircuitState::Open => {}
        }
        false
    }

    pub fn is_open(&self) -> bool {
        self.state == CircuitState::Open
    }
}

/// Manages circuit breakers for all alert types
pub struct CircuitBreakerManager {
    breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    default_config: CircuitBreakerConfig,
}

impl CircuitBreakerManager {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            breakers: Arc::new(RwLock::new(HashMap::new())),
            default_config: config,
        }
    }

    pub async fn should_allow(&self, alert_type: &str) -> bool {
        let mut breakers = self.breakers.write().await;
        let cb = breakers.entry(alert_type.to_string()).or_insert_with(|| {
            CircuitBreaker::new(alert_type.to_string(), self.default_config.clone())
        });
        cb.should_allow_execution()
    }

    /// Returns true if circuit closed as a result of this success.
    pub async fn record_success(&self, alert_type: &str) -> bool {
        let mut breakers = self.breakers.write().await;
        if let Some(cb) = breakers.get_mut(alert_type) {
            return cb.record_success();
        }
        false
    }

    /// Returns true if the circuit tripped open as a result of this failure.
    pub async fn record_failure(&self, alert_type: &str) -> bool {
        let mut breakers = self.breakers.write().await;
        let cb = breakers.entry(alert_type.to_string()).or_insert_with(|| {
            CircuitBreaker::new(alert_type.to_string(), self.default_config.clone())
        });
        cb.record_failure()
    }

    pub async fn get_state(&self, alert_type: &str) -> Option<CircuitState> {
        let breakers = self.breakers.read().await;
        breakers.get(alert_type).map(|cb| cb.state.clone())
    }

    pub async fn get_all_states(&self) -> HashMap<String, CircuitState> {
        let breakers = self.breakers.read().await;
        breakers
            .iter()
            .map(|(k, cb)| (k.clone(), cb.state.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cb(failure_threshold: u32) -> CircuitBreaker {
        CircuitBreaker::new(
            "TestAlert".to_string(),
            CircuitBreakerConfig {
                failure_threshold,
                success_threshold: 2,
                timeout_seconds: 1,
            },
        )
    }

    #[test]
    fn trips_after_threshold() {
        let mut cb = make_cb(3);
        assert!(cb.should_allow_execution());
        cb.record_failure();
        cb.record_failure();
        // 3rd failure should trip the circuit (returns true = tripped)
        assert!(cb.record_failure());
        assert_eq!(cb.state, CircuitState::Open);
        assert!(!cb.should_allow_execution());
    }

    #[test]
    fn closed_resets_on_success() {
        let mut cb = make_cb(5);
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        assert_eq!(cb.failure_count, 1); // decremented by one
        assert_eq!(cb.state, CircuitState::Closed);
    }

    #[tokio::test]
    async fn manager_tracks_per_alert_type() {
        let mgr = CircuitBreakerManager::new(CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            timeout_seconds: 300,
        });

        // Trip the breaker for "AlertA"
        mgr.record_failure("AlertA").await;
        let tripped = mgr.record_failure("AlertA").await;
        assert!(tripped);
        assert!(!mgr.should_allow("AlertA").await);

        // "AlertB" is unaffected
        assert!(mgr.should_allow("AlertB").await);
    }
}
