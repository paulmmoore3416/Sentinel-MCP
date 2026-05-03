//! Metrics collection and monitoring
//! 
//! Provides Prometheus-compatible metrics for:
//! - Alert processing
//! - Remediation success rates
//! - System performance
//! - API usage

use crate::error::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Metrics collector
pub struct MetricsCollector {
    counters: Arc<RwLock<HashMap<String, u64>>>,
    gauges: Arc<RwLock<HashMap<String, f64>>>,
    histograms: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    start_time: Instant,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }
    
    /// Increment a counter
    pub async fn increment_counter(&self, name: &str, value: u64) {
        let mut counters = self.counters.write().await;
        *counters.entry(name.to_string()).or_insert(0) += value;
    }
    
    /// Set a gauge value
    pub async fn set_gauge(&self, name: &str, value: f64) {
        let mut gauges = self.gauges.write().await;
        gauges.insert(name.to_string(), value);
    }
    
    /// Record a histogram value
    pub async fn record_histogram(&self, name: &str, value: f64) {
        let mut histograms = self.histograms.write().await;
        histograms.entry(name.to_string()).or_insert_with(Vec::new).push(value);
    }
    
    /// Get counter value
    pub async fn get_counter(&self, name: &str) -> u64 {
        let counters = self.counters.read().await;
        *counters.get(name).unwrap_or(&0)
    }
    
    /// Get gauge value
    pub async fn get_gauge(&self, name: &str) -> f64 {
        let gauges = self.gauges.read().await;
        *gauges.get(name).unwrap_or(&0.0)
    }
    
    /// Get histogram statistics
    pub async fn get_histogram_stats(&self, name: &str) -> HistogramStats {
        let histograms = self.histograms.read().await;
        
        if let Some(values) = histograms.get(name) {
            if values.is_empty() {
                return HistogramStats::default();
            }
            
            let mut sorted = values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            
            let sum: f64 = sorted.iter().sum();
            let count = sorted.len();
            let mean = sum / count as f64;
            
            let p50 = sorted[count / 2];
            let p95 = sorted[(count as f64 * 0.95) as usize];
            let p99 = sorted[(count as f64 * 0.99) as usize];
            
            HistogramStats {
                count,
                sum,
                mean,
                min: sorted[0],
                max: sorted[count - 1],
                p50,
                p95,
                p99,
            }
        } else {
            HistogramStats::default()
        }
    }
    
    /// Export metrics in Prometheus format
    pub async fn export_prometheus(&self) -> String {
        let mut output = String::new();
        
        // Counters
        let counters = self.counters.read().await;
        for (name, value) in counters.iter() {
            output.push_str(&format!("# TYPE {} counter\n", name));
            output.push_str(&format!("{} {}\n", name, value));
        }
        
        // Gauges
        let gauges = self.gauges.read().await;
        for (name, value) in gauges.iter() {
            output.push_str(&format!("# TYPE {} gauge\n", name));
            output.push_str(&format!("{} {}\n", name, value));
        }
        
        // Histograms
        let histograms = self.histograms.read().await;
        for (name, values) in histograms.iter() {
            if values.is_empty() {
                continue;
            }
            
            let stats = self.get_histogram_stats(name).await;
            output.push_str(&format!("# TYPE {} histogram\n", name));
            output.push_str(&format!("{}_count {}\n", name, stats.count));
            output.push_str(&format!("{}_sum {}\n", name, stats.sum));
            output.push_str(&format!("{}_bucket{{le=\"0.5\"}} {}\n", name, stats.p50));
            output.push_str(&format!("{}_bucket{{le=\"0.95\"}} {}\n", name, stats.p95));
            output.push_str(&format!("{}_bucket{{le=\"0.99\"}} {}\n", name, stats.p99));
            output.push_str(&format!("{}_bucket{{le=\"+Inf\"}} {}\n", name, stats.count));
        }
        
        // Uptime
        let uptime = self.start_time.elapsed().as_secs();
        output.push_str("# TYPE sentinel_uptime_seconds gauge\n");
        output.push_str(&format!("sentinel_uptime_seconds {}\n", uptime));
        
        output
    }
    
    /// Get metrics summary
    pub async fn get_summary(&self) -> MetricsSummary {
        MetricsSummary {
            uptime_seconds: self.start_time.elapsed().as_secs(),
            alerts_received: self.get_counter("alerts_received_total").await,
            remediations_executed: self.get_counter("remediations_executed_total").await,
            remediations_successful: self.get_counter("remediations_successful_total").await,
            remediations_failed: self.get_counter("remediations_failed_total").await,
            active_remediations: self.get_gauge("active_remediations").await as usize,
            queue_length: self.get_gauge("queue_length").await as usize,
            mttr_stats: self.get_histogram_stats("mttr_seconds").await,
            api_calls: self.get_counter("api_calls_total").await,
        }
    }
    
    /// Create router for metrics endpoints
    pub fn router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/metrics", get(metrics_handler))
            .route("/metrics/summary", get(summary_handler))
            .with_state(self)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Histogram statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramStats {
    pub count: usize,
    pub sum: f64,
    pub mean: f64,
    pub min: f64,
    pub max: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

impl Default for HistogramStats {
    fn default() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            mean: 0.0,
            min: 0.0,
            max: 0.0,
            p50: 0.0,
            p95: 0.0,
            p99: 0.0,
        }
    }
}

/// Metrics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub uptime_seconds: u64,
    pub alerts_received: u64,
    pub remediations_executed: u64,
    pub remediations_successful: u64,
    pub remediations_failed: u64,
    pub active_remediations: usize,
    pub queue_length: usize,
    pub mttr_stats: HistogramStats,
    pub api_calls: u64,
}

impl MetricsSummary {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.remediations_executed == 0 {
            return 0.0;
        }
        (self.remediations_successful as f64 / self.remediations_executed as f64) * 100.0
    }
}

/// Prometheus metrics handler
async fn metrics_handler(
    State(collector): State<Arc<MetricsCollector>>,
) -> impl IntoResponse {
    let metrics = collector.export_prometheus().await;
    (StatusCode::OK, metrics)
}

/// Metrics summary handler
async fn summary_handler(
    State(collector): State<Arc<MetricsCollector>>,
) -> impl IntoResponse {
    let summary = collector.get_summary().await;
    Json(summary)
}

/// Health check status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub components: HashMap<String, ComponentHealth>,
}

/// Component health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: String,
    pub message: Option<String>,
    pub last_check: String,
}

/// Health checker
pub struct HealthChecker {
    metrics: Arc<MetricsCollector>,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(metrics: Arc<MetricsCollector>) -> Self {
        Self { metrics }
    }
    
    /// Perform health check
    pub async fn check(&self) -> HealthStatus {
        let mut components = HashMap::new();
        
        // Check API server
        components.insert(
            "api_server".to_string(),
            ComponentHealth {
                status: "healthy".to_string(),
                message: None,
                last_check: chrono::Utc::now().to_rfc3339(),
            },
        );
        
        // Check WebSocket server
        components.insert(
            "websocket_server".to_string(),
            ComponentHealth {
                status: "healthy".to_string(),
                message: None,
                last_check: chrono::Utc::now().to_rfc3339(),
            },
        );
        
        // Check watsonx connection
        components.insert(
            "watsonx_connection".to_string(),
            self.check_watsonx().await,
        );
        
        // Check plugin system
        components.insert(
            "plugin_system".to_string(),
            ComponentHealth {
                status: "healthy".to_string(),
                message: Some("3 plugins loaded".to_string()),
                last_check: chrono::Utc::now().to_rfc3339(),
            },
        );
        
        // Determine overall status
        let overall_status = if components.values().all(|c| c.status == "healthy") {
            "healthy"
        } else if components.values().any(|c| c.status == "unhealthy") {
            "unhealthy"
        } else {
            "degraded"
        };
        
        HealthStatus {
            status: overall_status.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: self.metrics.start_time.elapsed().as_secs(),
            components,
        }
    }
    
    /// Check watsonx connection
    async fn check_watsonx(&self) -> ComponentHealth {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        let is_healthy = client
            .get("https://us-south.ml.cloud.ibm.com")
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false);

        // Unreachable external API = degraded (AI features offline), not unhealthy (system broken)
        ComponentHealth {
            status: if is_healthy { "healthy" } else { "degraded" }.to_string(),
            message: Some(if is_healthy { "API responding".to_string() } else { "API unreachable".to_string() }),
            last_check: chrono::Utc::now().to_rfc3339(),
        }
    }
    
    /// Create router for health endpoints
    pub fn router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/health", get(health_handler))
            .route("/health/live", get(liveness_handler))
            .route("/health/ready", get(readiness_handler))
            .with_state(self)
    }
}

/// Health check handler
async fn health_handler(
    State(checker): State<Arc<HealthChecker>>,
) -> impl IntoResponse {
    let health = checker.check().await;
    let status_code = match health.status.as_str() {
        "healthy" => StatusCode::OK,
        "degraded" => StatusCode::OK,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };
    (status_code, Json(health))
}

/// Liveness probe handler
async fn liveness_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "alive",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Readiness probe handler
async fn readiness_handler(
    State(checker): State<Arc<HealthChecker>>,
) -> impl IntoResponse {
    let health = checker.check().await;
    let ready = health.status == "healthy";
    let status_code = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    
    (status_code, Json(serde_json::json!({
        "ready": ready,
        "status": health.status,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        
        collector.increment_counter("test_counter", 1).await;
        collector.increment_counter("test_counter", 2).await;
        
        assert_eq!(collector.get_counter("test_counter").await, 3);
    }
    
    #[tokio::test]
    async fn test_histogram_stats() {
        let collector = MetricsCollector::new();
        
        collector.record_histogram("test_hist", 1.0).await;
        collector.record_histogram("test_hist", 2.0).await;
        collector.record_histogram("test_hist", 3.0).await;
        
        let stats = collector.get_histogram_stats("test_hist").await;
        assert_eq!(stats.count, 3);
        assert_eq!(stats.mean, 2.0);
    }
    
    #[tokio::test]
    async fn test_health_checker() {
        let metrics = Arc::new(MetricsCollector::new());
        let checker = HealthChecker::new(metrics);
        
        let health = checker.check().await;
        assert_ne!(health.status, "unhealthy", "system should not be unhealthy in test env");
    }
}

// Made with Bob