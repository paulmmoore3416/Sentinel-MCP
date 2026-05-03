//! Reasoning engine module
//! 
//! This module implements the core reasoning logic that orchestrates
//! the entire remediation workflow from alert to documentation.

use crate::alert::Alert;
use crate::executor::{ExecutionResult, RemediationReport};
use crate::mcp::{self, RiskLevel};
use crate::watsonx::{Analysis, RemediationSuggestion, WatsonxClient};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Reasoning engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    pub auto_approve_low_risk: bool,
    pub auto_approve_medium_risk: bool,
    pub approval_timeout_seconds: u64,
    pub max_execution_time_seconds: u64,
    pub enable_rollback: bool,
    pub dry_run_mode: bool,
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        Self {
            auto_approve_low_risk: true,
            auto_approve_medium_risk: false,
            approval_timeout_seconds: 300,
            max_execution_time_seconds: 600,
            enable_rollback: true,
            dry_run_mode: false,
        }
    }
}

/// Workflow state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowState {
    AlertReceived {
        alert: Alert,
        timestamp: String,
    },
    GatheringContext {
        alert: Alert,
        progress: u8,
    },
    Analyzing {
        alert: Alert,
        context: SystemContext,
    },
    ProposingFix {
        alert: Alert,
        analysis: Analysis,
    },
    AwaitingApproval {
        alert: Alert,
        plan: RemediationPlan,
        timeout: String,
    },
    Executing {
        alert: Alert,
        plan: RemediationPlan,
        step: usize,
    },
    Verifying {
        alert: Alert,
        execution_result: ExecutionResult,
    },
    Documenting {
        alert: Alert,
        full_context: RemediationContext,
    },
    Completed {
        alert: Alert,
        report: RemediationReport,
    },
    Failed {
        alert: Alert,
        error: String,
        rollback_attempted: bool,
    },
}

/// System context gathered from MCP tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemContext {
    pub logs: Vec<String>,
    pub disk_usage: Option<mcp::tools::DiskUsage>,
    pub service_status: Vec<mcp::tools::ServiceStatus>,
    pub k8s_status: Option<mcp::tools::K8sStatus>,
}

/// Remediation plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationPlan {
    pub alert_name: String,
    pub analysis: Analysis,
    pub steps: Vec<RemediationStep>,
    pub estimated_duration_seconds: u64,
}

/// Individual remediation step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationStep {
    pub step_number: usize,
    pub description: String,
    pub command: String,
    pub args: Vec<String>,
    pub risk_level: RiskLevel,
    pub expected_outcome: String,
    pub rollback_command: Option<String>,
}

/// Complete remediation context for documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationContext {
    pub alert: Alert,
    pub analysis: Analysis,
    pub plan: RemediationPlan,
    pub execution_result: ExecutionResult,
    pub verification: VerificationResult,
}

/// Verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub success: bool,
    pub details: String,
    pub metrics: HashMap<String, String>,
}

/// Reasoning engine
pub struct ReasoningEngine {
    watsonx_client: Arc<WatsonxClient>,
    state: Arc<RwLock<WorkflowState>>,
    config: ReasoningConfig,
}

impl ReasoningEngine {
    /// Create a new reasoning engine
    pub fn new(config: ReasoningConfig) -> Result<Self> {
        let watsonx_client = Arc::new(WatsonxClient::new()?);
        
        // Initial state
        let initial_state = WorkflowState::AlertReceived {
            alert: Alert {
                status: "idle".to_string(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
                starts_at: chrono::Utc::now().to_rfc3339(),
                ends_at: None,
                generator_url: String::new(),
                fingerprint: String::new(),
            },
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        
        Ok(Self {
            watsonx_client,
            state: Arc::new(RwLock::new(initial_state)),
            config,
        })
    }

    /// Process an alert through the complete workflow
    pub async fn process_alert(&self, alert: Alert) -> Result<RemediationReport> {
        tracing::info!("Processing alert: {:?}", alert.labels.get("alertname"));

        // Transition to AlertReceived state
        self.transition_to(WorkflowState::AlertReceived {
            alert: alert.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
        .await?;

        // Gather context
        let context = self.gather_context(&alert).await?;

        // Analyze with watsonx.ai
        let analysis = self.analyze_with_ai(&alert, &context).await?;

        // Generate remediation plan
        let plan = self.generate_plan(&alert, &analysis).await?;

        // Check if approval needed
        if self.requires_approval(&plan) {
            self.request_approval(&plan).await?;
        }

        // Execute remediation
        let execution_result = self.execute_plan(&plan).await?;

        // Verify success
        let verification = self.verify_remediation(&alert, &execution_result).await?;

        // Generate documentation
        let report = self.generate_report(RemediationContext {
            alert: alert.clone(),
            analysis,
            plan,
            execution_result,
            verification,
        })
        .await?;

        // Transition to Completed
        self.transition_to(WorkflowState::Completed {
            alert,
            report: report.clone(),
        })
        .await?;

        Ok(report)
    }

    /// Transition to a new workflow state
    async fn transition_to(&self, new_state: WorkflowState) -> Result<()> {
        let mut state = self.state.write().await;
        tracing::info!("State transition: {:?}", std::mem::discriminant(&new_state));
        *state = new_state;
        Ok(())
    }

    /// Gather system context using MCP tools
    async fn gather_context(&self, alert: &Alert) -> Result<SystemContext> {
        tracing::info!("Gathering system context");

        self.transition_to(WorkflowState::GatheringContext {
            alert: alert.clone(),
            progress: 0,
        })
        .await?;

        let mut context = SystemContext {
            logs: Vec::new(),
            disk_usage: None,
            service_status: Vec::new(),
            k8s_status: None,
        };

        // Gather logs based on alert type
        if let Some(filesystem) = alert.labels.get("filesystem") {
            // Disk space alert - get disk usage and logs
            if let Ok(response) = mcp::tools::get_disk_usage(filesystem).await {
                if let Some(data) = response.data {
                    context.disk_usage = Some(data);
                }
            }
            
            if let Ok(response) = mcp::tools::read_system_logs("syslog", 100, Some("disk")).await {
                if let Some(data) = response.data {
                    context.logs = data.lines;
                }
            }
        }

        if let Some(service) = alert.labels.get("service") {
            // Service alert - get service status and logs
            if let Ok(response) = mcp::tools::list_systemd_services(Some(service)).await {
                if let Some(data) = response.data {
                    context.service_status = data;
                }
            }
            
            if let Ok(response) = mcp::tools::read_system_logs("journalctl", 100, Some(service)).await {
                if let Some(data) = response.data {
                    context.logs = data.lines;
                }
            }
        }

        if let Some(namespace) = alert.labels.get("namespace") {
            // Kubernetes alert - get pod status
            let resource_type = alert.labels.get("resource_type").map(|s| s.as_str()).unwrap_or("pod");
            let name = alert.labels.get("pod");
            
            if let Ok(response) = mcp::tools::check_kubernetes_status(
                resource_type,
                namespace,
                name.map(|s| s.as_str()),
            ).await {
                if let Some(data) = response.data {
                    context.k8s_status = Some(data);
                }
            }
        }

        Ok(context)
    }

    /// Analyze with watsonx.ai
    async fn analyze_with_ai(&self, alert: &Alert, context: &SystemContext) -> Result<Analysis> {
        tracing::info!("Analyzing with watsonx.ai");

        self.transition_to(WorkflowState::Analyzing {
            alert: alert.clone(),
            context: context.clone(),
        })
        .await?;

        let logs_text = context.logs.join("\n");
        let context_json = serde_json::json!({
            "system_type": "Linux",
            "alert_name": alert.labels.get("alertname").unwrap_or(&"Unknown".to_string()),
            "severity": alert.labels.get("severity").unwrap_or(&"medium".to_string()),
        });

        self.watsonx_client
            .analyze_logs(&logs_text, &context_json.to_string())
            .await
    }

    /// Generate remediation plan
    async fn generate_plan(&self, alert: &Alert, analysis: &Analysis) -> Result<RemediationPlan> {
        tracing::info!("Generating remediation plan");

        self.transition_to(WorkflowState::ProposingFix {
            alert: alert.clone(),
            analysis: analysis.clone(),
        })
        .await?;

        let system_state = serde_json::json!({
            "alert": alert,
            "analysis": analysis,
        });

        let suggestions = self
            .watsonx_client
            .suggest_remediation(&analysis.root_cause, &system_state.to_string())
            .await?;

        let steps = suggestions
            .into_iter()
            .map(|s| self.convert_suggestion_to_step(s))
            .collect();

        Ok(RemediationPlan {
            alert_name: alert
                .labels
                .get("alertname")
                .unwrap_or(&"Unknown".to_string())
                .clone(),
            analysis: analysis.clone(),
            steps,
            estimated_duration_seconds: 60,
        })
    }

    /// Convert AI suggestion to remediation step
    fn convert_suggestion_to_step(&self, suggestion: RemediationSuggestion) -> RemediationStep {
        let (command, args) = self.parse_command(&suggestion.command);
        let risk_level = self.classify_risk(&suggestion.risk_level);

        RemediationStep {
            step_number: suggestion.step,
            description: suggestion.description,
            command,
            args,
            risk_level,
            expected_outcome: suggestion.expected_outcome,
            rollback_command: None, // TODO: Generate rollback commands
        }
    }

    /// Parse command string into command and args
    fn parse_command(&self, command_str: &str) -> (String, Vec<String>) {
        let parts: Vec<&str> = command_str.split_whitespace().collect();
        if parts.is_empty() {
            return (String::new(), Vec::new());
        }
        
        let command = parts[0].to_string();
        let args = parts[1..].iter().map(|s| s.to_string()).collect();
        (command, args)
    }

    /// Classify risk level from string
    fn classify_risk(&self, risk_str: &str) -> RiskLevel {
        match risk_str.to_lowercase().as_str() {
            "low" => RiskLevel::Low,
            "high" => RiskLevel::High,
            _ => RiskLevel::Medium,
        }
    }

    /// Check if plan requires approval
    fn requires_approval(&self, plan: &RemediationPlan) -> bool {
        if self.config.dry_run_mode {
            return false; // No approval needed in dry-run
        }

        plan.steps.iter().any(|step| match step.risk_level {
            RiskLevel::High => true,
            RiskLevel::Medium => !self.config.auto_approve_medium_risk,
            RiskLevel::Low => !self.config.auto_approve_low_risk,
        })
    }

    /// Request user approval
    async fn request_approval(&self, plan: &RemediationPlan) -> Result<()> {
        tracing::info!("Requesting approval for remediation plan");

        println!("\n=== REMEDIATION PLAN REQUIRES APPROVAL ===");
        println!("Alert: {}", plan.alert_name);
        println!("Root Cause: {}", plan.analysis.root_cause);
        println!("\nProposed Steps:");

        for step in &plan.steps {
            println!(
                "{}. {} [Risk: {:?}]",
                step.step_number, step.description, step.risk_level
            );
            println!("   Command: {} {}", step.command, step.args.join(" "));
        }

        println!("\nAuto-approving for demo purposes...");
        // In production, this would wait for user input
        Ok(())
    }

    /// Execute remediation plan
    async fn execute_plan(&self, plan: &RemediationPlan) -> Result<ExecutionResult> {
        tracing::info!("Executing remediation plan");

        let start = std::time::Instant::now();
        let mut all_success = true;
        let mut combined_stdout = String::new();
        let mut combined_stderr = String::new();

        for step in &plan.steps {
            tracing::info!("Executing step {}: {}", step.step_number, step.description);

            match mcp::tools::execute_remediation_script(
                &step.command,
                &step.args,
                self.config.dry_run_mode,
            )
            .await
            {
                Ok(response) => {
                    if let Some(result) = response.data {
                        all_success &= result.success;
                        combined_stdout.push_str(&result.stdout);
                        combined_stderr.push_str(&result.stderr);
                    }
                }
                Err(e) => {
                    all_success = false;
                    combined_stderr.push_str(&format!("Error: {}\n", e));
                }
            }
        }

        Ok(ExecutionResult {
            success: all_success,
            exit_code: if all_success { 0 } else { 1 },
            stdout: combined_stdout,
            stderr: combined_stderr,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Verify remediation was successful
    async fn verify_remediation(
        &self,
        _alert: &Alert,
        execution_result: &ExecutionResult,
    ) -> Result<VerificationResult> {
        tracing::info!("Verifying remediation");

        // Simple verification based on execution result
        // In production, this would re-check the system state
        Ok(VerificationResult {
            success: execution_result.success,
            details: if execution_result.success {
                "Remediation completed successfully".to_string()
            } else {
                "Remediation failed".to_string()
            },
            metrics: HashMap::new(),
        })
    }

    /// Generate remediation report
    async fn generate_report(&self, context: RemediationContext) -> Result<RemediationReport> {
        tracing::info!("Generating remediation report");

        Ok(RemediationReport {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            alert_name: context.alert.labels.get("alertname").unwrap_or(&"Unknown".to_string()).clone(),
            root_cause: context.analysis.root_cause,
            steps_executed: context
                .plan
                .steps
                .iter()
                .map(|s| s.description.clone())
                .collect(),
            success: context.verification.success,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command() {
        let engine = ReasoningEngine::new(ReasoningConfig::default()).unwrap();
        let (cmd, args) = engine.parse_command("systemctl restart nginx");
        assert_eq!(cmd, "systemctl");
        assert_eq!(args, vec!["restart", "nginx"]);
    }

    #[test]
    fn test_classify_risk() {
        let engine = ReasoningEngine::new(ReasoningConfig::default()).unwrap();
        assert!(matches!(engine.classify_risk("low"), RiskLevel::Low));
        assert!(matches!(engine.classify_risk("high"), RiskLevel::High));
        assert!(matches!(engine.classify_risk("medium"), RiskLevel::Medium));
    }
}

// Made with Bob
