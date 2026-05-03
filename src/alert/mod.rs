//! Alert receiver module
//! 
//! This module implements the Prometheus AlertManager webhook receiver
//! and alert processing logic.

use crate::reasoning::{ReasoningEngine, ReasoningConfig};
use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Alert receiver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub listen_address: String,
    pub listen_port: u16,
    pub max_queue_size: usize,
    pub enable_deduplication: bool,
    pub deduplication_window_seconds: u64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            listen_address: "0.0.0.0".to_string(),
            listen_port: 8080,
            max_queue_size: 100,
            enable_deduplication: true,
            deduplication_window_seconds: 300,
        }
    }
}

/// AlertManager webhook payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertManagerPayload {
    pub version: String,
    #[serde(rename = "groupKey")]
    pub group_key: String,
    pub status: String,
    pub receiver: String,
    #[serde(rename = "groupLabels")]
    pub group_labels: HashMap<String, String>,
    #[serde(rename = "commonLabels")]
    pub common_labels: HashMap<String, String>,
    #[serde(rename = "commonAnnotations")]
    pub common_annotations: HashMap<String, String>,
    #[serde(rename = "externalURL")]
    pub external_url: String,
    pub alerts: Vec<Alert>,
}

/// Individual alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub status: String,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    #[serde(rename = "startsAt")]
    pub starts_at: String,
    #[serde(rename = "endsAt")]
    pub ends_at: Option<String>,
    #[serde(rename = "generatorURL")]
    pub generator_url: String,
    pub fingerprint: String,
}

/// Alert response
#[derive(Debug, Serialize)]
pub struct AlertResponse {
    pub received: usize,
    pub status: String,
    pub message: String,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// Status response
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub queue_length: usize,
    pub active_remediations: usize,
    pub total_processed: u64,
    pub total_successful: u64,
    pub total_failed: u64,
}

/// Alert receiver state
pub struct AlertReceiver {
    reasoning_engine: Arc<ReasoningEngine>,
    alert_queue: Arc<Mutex<VecDeque<Alert>>>,
    config: AlertConfig,
    stats: Arc<Mutex<AlertStats>>,
    seen_fingerprints: Arc<Mutex<HashMap<String, chrono::DateTime<chrono::Utc>>>>,
}

/// Alert processing statistics
#[derive(Debug, Default)]
struct AlertStats {
    total_processed: u64,
    total_successful: u64,
    total_failed: u64,
    active_remediations: usize,
}

impl AlertReceiver {
    /// Create a new alert receiver
    pub fn new(config: AlertConfig, reasoning_config: ReasoningConfig) -> Result<Self> {
        let reasoning_engine = Arc::new(ReasoningEngine::new(reasoning_config)?);
        
        Ok(Self {
            reasoning_engine,
            alert_queue: Arc::new(Mutex::new(VecDeque::new())),
            config,
            stats: Arc::new(Mutex::new(AlertStats::default())),
            seen_fingerprints: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Start the HTTP server
    pub async fn start(self: Arc<Self>) -> Result<()> {
        let app = Router::new()
            .route("/api/v1/alerts", post(receive_alerts))
            .route("/api/v1/health", get(health_check))
            .route("/api/v1/status", get(get_status))
            .with_state(self.clone());

        let addr = format!("{}:{}", self.config.listen_address, self.config.listen_port);
        tracing::info!("Starting alert receiver on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Process a single alert
    async fn process_alert(&self, alert: Alert) -> Result<()> {
        // Check for duplicates
        if self.config.enable_deduplication && self.is_duplicate(&alert).await {
            tracing::info!("Skipping duplicate alert: {}", alert.fingerprint);
            return Ok(());
        }

        // Add to queue
        let mut queue = self.alert_queue.lock().await;
        if queue.len() >= self.config.max_queue_size {
            tracing::warn!("Alert queue full, dropping oldest alert");
            queue.pop_front();
        }
        queue.push_back(alert.clone());
        drop(queue);

        // Update stats
        {
            let mut stats = self.stats.lock().await;
            stats.active_remediations += 1;
        }

        // Process with reasoning engine
        let result = self.reasoning_engine.process_alert(alert.clone()).await;

        // Update stats
        {
            let mut stats = self.stats.lock().await;
            stats.active_remediations -= 1;
            stats.total_processed += 1;
            
            match result {
                Ok(_) => stats.total_successful += 1,
                Err(_) => stats.total_failed += 1,
            }
        }

        // Remove from queue
        let mut queue = self.alert_queue.lock().await;
        queue.retain(|a| a.fingerprint != alert.fingerprint);

        result.map(|_| ())
    }

    /// Check if alert is a duplicate
    async fn is_duplicate(&self, alert: &Alert) -> bool {
        let mut seen = self.seen_fingerprints.lock().await;
        let now = chrono::Utc::now();

        // Clean up old entries
        seen.retain(|_, timestamp| {
            now.signed_duration_since(*timestamp).num_seconds()
                < self.config.deduplication_window_seconds as i64
        });

        // Check if we've seen this fingerprint recently
        if seen.contains_key(&alert.fingerprint) {
            return true;
        }

        // Record this fingerprint
        seen.insert(alert.fingerprint.clone(), now);
        false
    }

    /// Get current statistics
    async fn get_stats(&self) -> AlertStats {
        let stats = self.stats.lock().await;
        AlertStats {
            total_processed: stats.total_processed,
            total_successful: stats.total_successful,
            total_failed: stats.total_failed,
            active_remediations: stats.active_remediations,
        }
    }
}

/// HTTP handler for receiving alerts
pub async fn receive_alerts(
    State(receiver): State<Arc<AlertReceiver>>,
    Json(payload): Json<AlertManagerPayload>,
) -> Result<Json<AlertResponse>, StatusCode> {
    tracing::info!(
        "Received {} alerts from AlertManager",
        payload.alerts.len()
    );

    let mut processed = 0;
    let mut errors = Vec::new();
    let alerts_count = payload.alerts.len();

    for alert in payload.alerts {
        if alert.status == "firing" {
            match receiver.process_alert(alert.clone()).await {
                Ok(_) => processed += 1,
                Err(e) => {
                    tracing::error!("Failed to process alert: {}", e);
                    errors.push(format!("Alert {}: {}", alert.fingerprint, e));
                }
            }
        }
    }

    let response = AlertResponse {
        received: alerts_count,
        status: if errors.is_empty() {
            "success".to_string()
        } else {
            "partial".to_string()
        },
        message: if errors.is_empty() {
            format!("Processed {} alerts", processed)
        } else {
            format!("Processed {} alerts, {} errors", processed, errors.len())
        },
    };

    Ok(Json(response))
}

/// HTTP handler for health check
async fn health_check() -> Json<HealthResponse> {
    // Simple uptime calculation (would need to track start time in production)
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // TODO: Track actual uptime
    })
}

/// HTTP handler for status
async fn get_status(State(receiver): State<Arc<AlertReceiver>>) -> Json<StatusResponse> {
    let queue_length = receiver.alert_queue.lock().await.len();
    let stats = receiver.get_stats().await;

    Json(StatusResponse {
        queue_length,
        active_remediations: stats.active_remediations,
        total_processed: stats.total_processed,
        total_successful: stats.total_successful,
        total_failed: stats.total_failed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_config_default() {
        let config = AlertConfig::default();
        assert_eq!(config.listen_port, 8080);
        assert_eq!(config.max_queue_size, 100);
        assert!(config.enable_deduplication);
    }

    #[tokio::test]
    async fn test_duplicate_detection() {
        let config = AlertConfig::default();
        let reasoning_config = ReasoningConfig::default();
        let receiver = AlertReceiver::new(config, reasoning_config).unwrap();

        let alert = Alert {
            status: "firing".to_string(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
            starts_at: chrono::Utc::now().to_rfc3339(),
            ends_at: None,
            generator_url: String::new(),
            fingerprint: "test123".to_string(),
        };

        // First time should not be duplicate
        assert!(!receiver.is_duplicate(&alert).await);

        // Second time should be duplicate
        assert!(receiver.is_duplicate(&alert).await);
    }
}

// Made with Bob
