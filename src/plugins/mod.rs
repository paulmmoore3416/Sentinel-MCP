//! Plugin system for extensible remediation strategies
//! 
//! This module provides a plugin architecture that allows users to:
//! - Define custom remediation strategies
//! - Register handlers for specific alert types
//! - Extend functionality without modifying core code

use crate::alert::Alert;
use crate::error::{Result, SentinelError};
use crate::reasoning::{RemediationPlan, RemediationStep};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub supported_alert_types: Vec<String>,
}

/// Plugin context provided to plugins
#[derive(Debug, Clone)]
pub struct PluginContext {
    pub alert: Alert,
    pub system_info: HashMap<String, String>,
    pub dry_run: bool,
}

/// Plugin trait that all plugins must implement
#[async_trait]
pub trait RemediationPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> PluginMetadata;
    
    /// Check if this plugin can handle the given alert
    fn can_handle(&self, alert: &Alert) -> bool;
    
    /// Analyze the alert and generate a remediation plan
    async fn analyze(&self, context: &PluginContext) -> Result<RemediationPlan>;
    
    /// Execute a remediation step
    async fn execute_step(
        &self,
        context: &PluginContext,
        step: &RemediationStep,
    ) -> Result<ExecutionResult>;
    
    /// Verify that remediation was successful
    async fn verify(&self, context: &PluginContext) -> Result<VerificationResult>;
    
    /// Optional: Rollback changes if remediation failed
    async fn rollback(&self, context: &PluginContext) -> Result<()> {
        Ok(())
    }
}

/// Execution result from a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub message: String,
    pub details: HashMap<String, String>,
}

/// Verification result from a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub success: bool,
    pub message: String,
    pub metrics: HashMap<String, String>,
}

/// Plugin registry for managing plugins
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, Arc<dyn RemediationPlugin>>>>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a plugin
    pub async fn register(&self, plugin: Arc<dyn RemediationPlugin>) -> Result<()> {
        let metadata = plugin.metadata();
        let mut plugins = self.plugins.write().await;
        
        if plugins.contains_key(&metadata.name) {
            return Err(SentinelError::Config(format!(
                "Plugin '{}' is already registered",
                metadata.name
            )));
        }
        
        tracing::info!(
            "Registering plugin: {} v{} by {}",
            metadata.name,
            metadata.version,
            metadata.author
        );
        
        plugins.insert(metadata.name.clone(), plugin);
        Ok(())
    }
    
    /// Unregister a plugin
    pub async fn unregister(&self, name: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        
        if plugins.remove(name).is_none() {
            return Err(SentinelError::Config(format!(
                "Plugin '{}' not found",
                name
            )));
        }
        
        tracing::info!("Unregistered plugin: {}", name);
        Ok(())
    }
    
    /// Find a plugin that can handle the given alert
    pub async fn find_handler(&self, alert: &Alert) -> Option<Arc<dyn RemediationPlugin>> {
        let plugins = self.plugins.read().await;
        
        for plugin in plugins.values() {
            if plugin.can_handle(alert) {
                return Some(Arc::clone(plugin));
            }
        }
        
        None
    }
    
    /// Get all registered plugins
    pub async fn list_plugins(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins.values().map(|p| p.metadata()).collect()
    }
    
    /// Get a specific plugin by name
    pub async fn get_plugin(&self, name: &str) -> Option<Arc<dyn RemediationPlugin>> {
        let plugins = self.plugins.read().await;
        plugins.get(name).map(Arc::clone)
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in disk cleanup plugin
pub struct DiskCleanupPlugin;

#[async_trait]
impl RemediationPlugin for DiskCleanupPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "disk-cleanup".to_string(),
            version: "1.0.0".to_string(),
            author: "Sentinel-MCP".to_string(),
            description: "Handles disk space alerts by cleaning up old files".to_string(),
            supported_alert_types: vec!["DiskSpaceLow".to_string(), "DiskSpaceCritical".to_string()],
        }
    }
    
    fn can_handle(&self, alert: &Alert) -> bool {
        alert.labels.get("alertname")
            .map(|name| name.contains("DiskSpace"))
            .unwrap_or(false)
    }
    
    async fn analyze(&self, context: &PluginContext) -> Result<RemediationPlan> {
        use crate::mcp::RiskLevel;
        use crate::watsonx::Analysis;
        
        let filesystem = context.alert.labels.get("filesystem")
            .ok_or_else(|| SentinelError::AlertProcessing(
                "Missing filesystem label".to_string()
            ))?;
        
        let steps = vec![
            RemediationStep {
                step_number: 1,
                description: format!("Analyze disk usage on {}", filesystem),
                command: "du".to_string(),
                args: vec!["-sh".to_string(), format!("{}/*", filesystem)],
                risk_level: RiskLevel::Low,
                expected_outcome: "Identify large directories".to_string(),
                rollback_command: None,
            },
            RemediationStep {
                step_number: 2,
                description: "Clean old log files".to_string(),
                command: "find".to_string(),
                args: vec![
                    format!("{}/log", filesystem),
                    "-name".to_string(),
                    "*.log.*".to_string(),
                    "-mtime".to_string(),
                    "+30".to_string(),
                    "-delete".to_string(),
                ],
                risk_level: RiskLevel::Medium,
                expected_outcome: "Free up disk space".to_string(),
                rollback_command: None,
            },
        ];
        
        Ok(RemediationPlan {
            alert_name: "DiskSpaceLow".to_string(),
            analysis: Analysis {
                root_cause: "Disk space exhausted due to old log files".to_string(),
                affected_components: vec![filesystem.clone()],
                impact: "Service degradation possible".to_string(),
                urgency: "high".to_string(),
                lessons_learned: None,
            },
            steps,
            estimated_duration_seconds: 120,
            from_runbook: true,
        })
    }
    
    async fn execute_step(
        &self,
        context: &PluginContext,
        step: &RemediationStep,
    ) -> Result<ExecutionResult> {
        if context.dry_run {
            return Ok(ExecutionResult {
                success: true,
                message: format!("[DRY RUN] Would execute: {} {}", step.command, step.args.join(" ")),
                details: HashMap::new(),
            });
        }
        
        // Execute the command
        use tokio::process::Command;
        let output = Command::new(&step.command)
            .args(&step.args)
            .output()
            .await
            .map_err(|e| SentinelError::SystemCommand(e.to_string()))?;
        
        Ok(ExecutionResult {
            success: output.status.success(),
            message: if output.status.success() {
                "Command executed successfully".to_string()
            } else {
                format!("Command failed with exit code: {:?}", output.status.code())
            },
            details: {
                let mut details = HashMap::new();
                details.insert("stdout".to_string(), String::from_utf8_lossy(&output.stdout).to_string());
                details.insert("stderr".to_string(), String::from_utf8_lossy(&output.stderr).to_string());
                details
            },
        })
    }
    
    async fn verify(&self, context: &PluginContext) -> Result<VerificationResult> {
        let filesystem = context.alert.labels.get("filesystem")
            .ok_or_else(|| SentinelError::AlertProcessing(
                "Missing filesystem label".to_string()
            ))?;
        
        // Check disk usage
        use tokio::process::Command;
        let output = Command::new("df")
            .args(&["-h", filesystem])
            .output()
            .await
            .map_err(|e| SentinelError::SystemCommand(e.to_string()))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let success = output.status.success() && !stdout.contains("100%");
        
        Ok(VerificationResult {
            success,
            message: if success {
                "Disk space recovered successfully".to_string()
            } else {
                "Disk space still critical".to_string()
            },
            metrics: {
                let mut metrics = HashMap::new();
                metrics.insert("filesystem".to_string(), filesystem.clone());
                metrics.insert("df_output".to_string(), stdout.to_string());
                metrics
            },
        })
    }
}

/// Built-in service restart plugin
pub struct ServiceRestartPlugin;

#[async_trait]
impl RemediationPlugin for ServiceRestartPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "service-restart".to_string(),
            version: "1.0.0".to_string(),
            author: "Sentinel-MCP".to_string(),
            description: "Handles service failures by restarting services".to_string(),
            supported_alert_types: vec!["ServiceDown".to_string(), "ServiceCrash".to_string()],
        }
    }
    
    fn can_handle(&self, alert: &Alert) -> bool {
        alert.labels.get("alertname")
            .map(|name| name.contains("Service"))
            .unwrap_or(false)
    }
    
    async fn analyze(&self, context: &PluginContext) -> Result<RemediationPlan> {
        use crate::mcp::RiskLevel;
        use crate::watsonx::Analysis;
        
        let service = context.alert.labels.get("service")
            .ok_or_else(|| SentinelError::AlertProcessing(
                "Missing service label".to_string()
            ))?;
        
        let steps = vec![
            RemediationStep {
                step_number: 1,
                description: format!("Check status of {}", service),
                command: "systemctl".to_string(),
                args: vec!["status".to_string(), service.clone()],
                risk_level: RiskLevel::Low,
                expected_outcome: "Determine service state".to_string(),
                rollback_command: None,
            },
            RemediationStep {
                step_number: 2,
                description: format!("Restart {}", service),
                command: "systemctl".to_string(),
                args: vec!["restart".to_string(), service.clone()],
                risk_level: RiskLevel::Medium,
                expected_outcome: "Service running".to_string(),
                rollback_command: Some(format!("systemctl stop {}", service)),
            },
        ];
        
        Ok(RemediationPlan {
            alert_name: "ServiceDown".to_string(),
            analysis: Analysis {
                root_cause: format!("Service {} is not running", service),
                affected_components: vec![service.clone()],
                impact: "Service unavailable".to_string(),
                urgency: "high".to_string(),
                lessons_learned: None,
            },
            steps,
            estimated_duration_seconds: 60,
            from_runbook: true,
        })
    }
    
    async fn execute_step(
        &self,
        context: &PluginContext,
        step: &RemediationStep,
    ) -> Result<ExecutionResult> {
        if context.dry_run {
            return Ok(ExecutionResult {
                success: true,
                message: format!("[DRY RUN] Would execute: {} {}", step.command, step.args.join(" ")),
                details: HashMap::new(),
            });
        }
        
        use tokio::process::Command;
        let output = Command::new(&step.command)
            .args(&step.args)
            .output()
            .await
            .map_err(|e| SentinelError::SystemCommand(e.to_string()))?;
        
        Ok(ExecutionResult {
            success: output.status.success(),
            message: if output.status.success() {
                "Command executed successfully".to_string()
            } else {
                format!("Command failed: {}", String::from_utf8_lossy(&output.stderr))
            },
            details: {
                let mut details = HashMap::new();
                details.insert("stdout".to_string(), String::from_utf8_lossy(&output.stdout).to_string());
                details
            },
        })
    }
    
    async fn verify(&self, context: &PluginContext) -> Result<VerificationResult> {
        let service = context.alert.labels.get("service")
            .ok_or_else(|| SentinelError::AlertProcessing(
                "Missing service label".to_string()
            ))?;
        
        use tokio::process::Command;
        let output = Command::new("systemctl")
            .args(&["is-active", service])
            .output()
            .await
            .map_err(|e| SentinelError::SystemCommand(e.to_string()))?;
        
        let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let success = status == "active";
        
        Ok(VerificationResult {
            success,
            message: if success {
                format!("Service {} is now active", service)
            } else {
                format!("Service {} is still {}", service, status)
            },
            metrics: {
                let mut metrics = HashMap::new();
                metrics.insert("service".to_string(), service.clone());
                metrics.insert("status".to_string(), status);
                metrics
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_plugin_registry() {
        let registry = PluginRegistry::new();
        let plugin = Arc::new(DiskCleanupPlugin);
        
        registry.register(plugin).await.unwrap();
        
        let plugins = registry.list_plugins().await;
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "disk-cleanup");
    }
    
    #[tokio::test]
    async fn test_disk_cleanup_plugin() {
        let plugin = DiskCleanupPlugin;
        let metadata = plugin.metadata();
        
        assert_eq!(metadata.name, "disk-cleanup");
        assert!(metadata.supported_alert_types.contains(&"DiskSpaceLow".to_string()));
    }
}

// Made with Bob