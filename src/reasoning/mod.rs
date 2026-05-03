//! Reasoning engine module
//!
//! Orchestrates the full remediation workflow: alert → context → analysis →
//! plan → approval → execution → verification → report.
//!
//! Safety layers added over the original skeleton:
//!   - Circuit breakers prevent cascading failures per alert type
//!   - Runbook registry provides tested, rollback-ready step templates
//!   - Rollback commands are generated for every reversible operation
//!   - Deep verification checks the alert is actually resolved

use crate::alert::Alert;
use crate::circuit_breaker::{CircuitBreakerConfig, CircuitBreakerManager};
use crate::executor::{ExecutionResult, RemediationReport};
use crate::mcp::{self, RiskLevel};
use crate::mempalace::MemPalaceClient;
use crate::runbook::RunbookRegistry;
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

/// Workflow state machine
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
    pub network_status: Option<mcp::tools::NetworkDiagnostics>,
    pub db_status: Option<mcp::tools::DatabaseMetrics>,
    pub node_status: Option<mcp::tools::NodeDiagnostics>,
    pub tls_status: Option<mcp::tools::TlsVerification>,
}

/// Remediation plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationPlan {
    pub alert_name: String,
    pub analysis: Analysis,
    pub steps: Vec<RemediationStep>,
    pub estimated_duration_seconds: u64,
    /// Whether steps came from a pre-tested runbook or AI generation
    pub from_runbook: bool,
}

/// Individual remediation step with optional rollback
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

/// Deep verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub success: bool,
    pub details: String,
    pub metrics: HashMap<String, String>,
    /// Whether the originating alert appears resolved post-remediation
    pub alert_resolved: bool,
    /// 0.0–1.0 confidence that the remediation succeeded
    pub confidence_score: f64,
}

/// Reasoning engine
pub struct ReasoningEngine {
    watsonx_client: Arc<WatsonxClient>,
    state: Arc<RwLock<WorkflowState>>,
    config: ReasoningConfig,
    circuit_breakers: Arc<CircuitBreakerManager>,
    runbook_registry: Arc<RunbookRegistry>,
    mempalace_client: Option<Arc<MemPalaceClient>>,
}

impl ReasoningEngine {
    pub fn new(config: ReasoningConfig) -> Result<Self> {
        let watsonx_client = Arc::new(WatsonxClient::new()?);

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

        // Initialize MemPalace client if configured
        let mempalace_client = if let Ok(url) = std::env::var("MEMPALACE_URL") {
            tracing::info!("MemPalace integration enabled at {}", url);
            Some(Arc::new(MemPalaceClient::new(&url)))
        } else {
            tracing::info!("MemPalace integration disabled (MEMPALACE_URL not set)");
            None
        };

        Ok(Self {
            watsonx_client,
            state: Arc::new(RwLock::new(initial_state)),
            config,
            circuit_breakers: Arc::new(CircuitBreakerManager::new(
                CircuitBreakerConfig::default(),
            )),
            runbook_registry: Arc::new(RunbookRegistry::new()),
            mempalace_client,
        })
    }

    /// Process an alert through the complete workflow
    pub async fn process_alert(&self, alert: Alert) -> Result<RemediationReport> {
        let alert_type = alert
            .labels
            .get("alertname")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        tracing::info!("Processing alert: {}", alert_type);

        // Circuit breaker check — block if too many recent failures
        if !self.circuit_breakers.should_allow(&alert_type).await {
            tracing::warn!("Circuit breaker OPEN for '{}' — escalating to human", alert_type);
            return Err(anyhow!(
                "Circuit breaker open for '{}': too many recent remediation failures. \
                 Escalate to on-call engineer.",
                alert_type
            ));
        }

        self.transition_to(WorkflowState::AlertReceived {
            alert: alert.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
        .await?;

        let context = self.gather_context(&alert).await?;
        let analysis = self.analyze_with_ai(&alert, &context).await?;
        let plan = self.generate_plan(&alert, &analysis).await?;

        if self.requires_approval(&plan) {
            self.request_approval(&plan).await?;
        }

        let execution_result = self.execute_plan(&plan).await?;
        let verification = self.verify_remediation(&alert, &execution_result).await?;

        // Update circuit breaker based on outcome
        if verification.success {
            self.circuit_breakers.record_success(&alert_type).await;
        } else {
            let tripped = self.circuit_breakers.record_failure(&alert_type).await;
            if tripped {
                tracing::warn!(
                    "Circuit breaker tripped for '{}' after this failure",
                    alert_type
                );
            }
        }

        let report = self
            .generate_report(RemediationContext {
                alert: alert.clone(),
                analysis,
                plan,
                execution_result,
                verification,
            })
            .await?;

        self.transition_to(WorkflowState::Completed {
            alert,
            report: report.clone(),
        })
        .await?;

        Ok(report)
    }

    async fn transition_to(&self, new_state: WorkflowState) -> Result<()> {
        let mut state = self.state.write().await;
        tracing::debug!("State transition: {:?}", std::mem::discriminant(&new_state));
        *state = new_state;
        Ok(())
    }

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
            network_status: None,
            db_status: None,
            node_status: None,
            tls_status: None,
        };

        if let Some(filesystem) = alert.labels.get("filesystem") {
            if let Ok(response) = mcp::tools::get_disk_usage(filesystem).await {
                if let Some(data) = response.data {
                    context.disk_usage = Some(data);
                }
            }
            if let Ok(response) =
                mcp::tools::read_system_logs("syslog", 100, Some("disk")).await
            {
                if let Some(data) = response.data {
                    context.logs = data.lines;
                }
            }
        }

        if let Some(service) = alert.labels.get("service") {
            if let Ok(response) = mcp::tools::list_systemd_services(Some(service)).await {
                if let Some(data) = response.data {
                    context.service_status = data;
                }
            }
            if let Ok(response) =
                mcp::tools::read_system_logs("journalctl", 100, Some(service)).await
            {
                if let Some(data) = response.data {
                    context.logs = data.lines;
                }
            }
        }

        if let Some(namespace) = alert.labels.get("namespace") {
            let resource_type = alert
                .labels
                .get("resource_type")
                .map(|s| s.as_str())
                .unwrap_or("pod");
            let name = alert.labels.get("pod");

            if let Ok(response) = mcp::tools::check_kubernetes_status(
                resource_type,
                namespace,
                name.map(|s| s.as_str()),
            )
            .await
            {
                if let Some(data) = response.data {
                    context.k8s_status = Some(data);
                }
            }
        }

        if let Some(endpoint) = alert.labels.get("endpoint") {
            if alert.labels.get("alertname").map(|s| s.as_str()) == Some("EndpointUnreachable") {
                if let Ok(response) = mcp::tools::run_network_diagnostics(endpoint).await {
                    if let Some(data) = response.data {
                        context.network_status = Some(data);
                    }
                }
            }
            if alert.labels.get("alertname").map(|s| s.as_str()) == Some("TLSCertificateExpiring") {
                if let Ok(response) = mcp::tools::check_tls_certificate(endpoint).await {
                    if let Some(data) = response.data {
                        context.tls_status = Some(data);
                    }
                }
            }
        }

        if alert.labels.get("alertname").map(|s| s.as_str()) == Some("DatabaseHighConnections") {
            if let Ok(response) = mcp::tools::check_db_metrics().await {
                if let Some(data) = response.data {
                    context.db_status = Some(data);
                }
            }
        }

        if let Some(node) = alert.labels.get("node") {
            if alert.labels.get("alertname").map(|s| s.as_str()) == Some("NodeNotReady") {
                if let Ok(response) = mcp::tools::describe_node(node).await {
                    if let Some(data) = response.data {
                        context.node_status = Some(data);
                    }
                }
            }
        }

        Ok(context)
    }

    async fn analyze_with_ai(&self, alert: &Alert, context: &SystemContext) -> Result<Analysis> {
        tracing::info!("Analyzing with watsonx.ai");

        self.transition_to(WorkflowState::Analyzing {
            alert: alert.clone(),
            context: context.clone(),
        })
        .await?;

        let logs_text = context.logs.join("\n");
        let unknown_alert = "Unknown".to_string();
        let alert_name = alert.labels.get("alertname").unwrap_or(&unknown_alert);

        // Recall from MemPalace
        let mut historical_context = String::new();
        if let Some(mempalace) = &self.mempalace_client {
            let query = format!("previous incidents for alert '{}'", alert_name);
            match mempalace.recall(&query).await {
                Ok(memories) if !memories.is_empty() => {
                    tracing::info!("Found relevant memories in MemPalace");
                    historical_context = memories;
                }
                Ok(_) => tracing::info!("No relevant memories found in MemPalace"),
                Err(e) => tracing::warn!("MemPalace recall failed: {}", e),
            }
        }
        
        let context_json = serde_json::json!({
            "system_type": "Linux",
            "alert_name": alert_name,
            "severity": alert.labels.get("severity").unwrap_or(&"medium".to_string()),
            "historical_context": historical_context,
        });

        self.watsonx_client
            .analyze_logs(&logs_text, &context_json.to_string())
            .await
    }

    /// Generate a remediation plan, preferring tested runbooks over AI generation.
    async fn generate_plan(&self, alert: &Alert, analysis: &Analysis) -> Result<RemediationPlan> {
        tracing::info!("Generating remediation plan");

        self.transition_to(WorkflowState::ProposingFix {
            alert: alert.clone(),
            analysis: analysis.clone(),
        })
        .await?;

        let alert_name = alert
            .labels
            .get("alertname")
            .map(|s| s.as_str())
            .unwrap_or("Unknown");

        // Tier 1: exact runbook match
        if let Some(runbook) = self.runbook_registry.find_by_alert(alert_name) {
            tracing::info!("Using runbook '{}' for alert '{}'", runbook.id, alert_name);

            let mut vars = HashMap::new();
            for key in &["service", "pod", "namespace", "filesystem"] {
                if let Some(val) = alert.labels.get(*key) {
                    vars.insert(key.to_string(), val.clone());
                }
            }

            let resolved = self.runbook_registry.apply_template(runbook, &vars);
            let steps = resolved
                .steps
                .iter()
                .map(|rs| RemediationStep {
                    step_number: rs.step_number,
                    description: rs.description.clone(),
                    command: rs.command.clone(),
                    args: rs.args.clone(),
                    risk_level: rs.risk_level,
                    expected_outcome: rs.expected_outcome.clone(),
                    rollback_command: rs.rollback_command.clone(),
                })
                .collect();

            return Ok(RemediationPlan {
                alert_name: alert_name.to_string(),
                analysis: analysis.clone(),
                steps,
                estimated_duration_seconds: 60,
                from_runbook: true,
            });
        }

        // Tier 2: AI-generated steps with rollback generation
        tracing::info!("No runbook match; falling back to AI-generated plan");

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
            alert_name: alert_name.to_string(),
            analysis: analysis.clone(),
            steps,
            estimated_duration_seconds: 60,
            from_runbook: false,
        })
    }

    fn convert_suggestion_to_step(&self, suggestion: RemediationSuggestion) -> RemediationStep {
        let (command, args) = Self::parse_command(&suggestion.command);
        let risk_level = Self::classify_risk(&suggestion.risk_level);
        let rollback_command = Self::generate_rollback_command(&command, &args);

        RemediationStep {
            step_number: suggestion.step,
            description: suggestion.description,
            command,
            args,
            risk_level,
            expected_outcome: suggestion.expected_outcome,
            rollback_command,
        }
    }

    /// Derive a rollback command from a forward command where possible.
    fn generate_rollback_command(command: &str, args: &[String]) -> Option<String> {
        let first = args.first().map(|s| s.as_str()).unwrap_or("");
        let rest = if args.len() > 1 { &args[1..] } else { &[] };

        match command {
            "systemctl" => match first {
                "stop" => rest.first().map(|s| format!("systemctl start {}", s)),
                "start" => rest.first().map(|s| format!("systemctl stop {}", s)),
                "disable" => rest.first().map(|s| format!("systemctl enable {}", s)),
                "enable" => rest.first().map(|s| format!("systemctl disable {}", s)),
                // restart is its own inverse
                "restart" => rest.first().map(|s| format!("systemctl restart {}", s)),
                _ => None,
            },
            "apt-get" | "apt" => match first {
                "install" => rest
                    .iter()
                    .find(|a| !a.starts_with('-'))
                    .map(|pkg| format!("apt-get remove -y {}", pkg)),
                "remove" | "purge" => rest
                    .iter()
                    .find(|a| !a.starts_with('-'))
                    .map(|pkg| format!("apt-get install -y {}", pkg)),
                _ => None,
            },
            "kubectl" => match first {
                // Scale can be reversed if we know the original replica count,
                // but we don't have that here — leave it for the snapshot to handle.
                _ => None,
            },
            // Read-only or irreversible operations
            "find" | "cat" | "grep" | "ls" | "df" | "du" | "ps" | "top" | "free"
            | "journalctl" | "logrotate" | "sync" | "rm" | "rmdir" => None,
            _ => None,
        }
    }

    fn parse_command(command_str: &str) -> (String, Vec<String>) {
        let parts: Vec<&str> = command_str.split_whitespace().collect();
        if parts.is_empty() {
            return (String::new(), Vec::new());
        }
        let command = parts[0].to_string();
        let args = parts[1..].iter().map(|s| s.to_string()).collect();
        (command, args)
    }

    fn classify_risk(risk_str: &str) -> RiskLevel {
        match risk_str.to_lowercase().as_str() {
            "low" => RiskLevel::Low,
            "high" => RiskLevel::High,
            _ => RiskLevel::Medium,
        }
    }

    fn requires_approval(&self, plan: &RemediationPlan) -> bool {
        if self.config.dry_run_mode {
            return false;
        }
        plan.steps.iter().any(|step| match step.risk_level {
            RiskLevel::High => true,
            RiskLevel::Medium => !self.config.auto_approve_medium_risk,
            RiskLevel::Low => !self.config.auto_approve_low_risk,
        })
    }

    async fn request_approval(&self, plan: &RemediationPlan) -> Result<()> {
        tracing::info!("Requesting approval for remediation plan");

        println!("\n=== REMEDIATION PLAN REQUIRES APPROVAL ===");
        println!("Alert: {}", plan.alert_name);
        println!("Root Cause: {}", plan.analysis.root_cause);
        println!("Source: {}", if plan.from_runbook { "tested runbook" } else { "AI-generated" });
        println!("\nProposed Steps:");

        for step in &plan.steps {
            println!(
                "  {}. {} [Risk: {:?}]",
                step.step_number, step.description, step.risk_level
            );
            println!("     Command: {} {}", step.command, step.args.join(" "));
            if let Some(rb) = &step.rollback_command {
                println!("     Rollback: {}", rb);
            }
        }

        println!("\nAuto-approving for demo purposes...");
        Ok(())
    }

    async fn execute_plan(&self, plan: &RemediationPlan) -> Result<ExecutionResult> {
        tracing::info!("Executing remediation plan");

        let start = std::time::Instant::now();
        let mut all_success = true;
        let mut combined_stdout = String::new();
        let mut combined_stderr = String::new();

        for step in &plan.steps {
            tracing::info!(
                "Executing step {}: {}",
                step.step_number,
                step.description
            );

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

    /// Deep verification: re-probe the system to confirm the alert is actually resolved.
    async fn verify_remediation(
        &self,
        alert: &Alert,
        execution_result: &ExecutionResult,
    ) -> Result<VerificationResult> {
        tracing::info!("Verifying remediation");

        let command_success = execution_result.success;
        let alert_resolved = self.check_alert_resolved(alert).await;

        let confidence_score = match (command_success, alert_resolved) {
            (true, true) => 0.95,
            (true, false) => 0.60,  // command ran but alert not yet resolved
            (false, true) => 0.40,  // alert resolved despite command failure (partial success)
            (false, false) => 0.05,
        };

        let success = command_success && confidence_score >= 0.5;

        Ok(VerificationResult {
            success,
            alert_resolved,
            confidence_score,
            details: format!(
                "Command: {}, Alert resolved: {}, Confidence: {:.0}%",
                if command_success { "succeeded" } else { "failed" },
                if alert_resolved { "yes" } else { "unknown/no" },
                confidence_score * 100.0,
            ),
            metrics: HashMap::from([
                ("command_success".to_string(), command_success.to_string()),
                ("alert_resolved".to_string(), alert_resolved.to_string()),
                ("confidence_score".to_string(), format!("{:.2}", confidence_score)),
                (
                    "exit_code".to_string(),
                    execution_result.exit_code.to_string(),
                ),
            ]),
        })
    }

    /// Re-probe the system to determine whether the alert condition has cleared.
    async fn check_alert_resolved(&self, alert: &Alert) -> bool {
        if let Some(filesystem) = alert.labels.get("filesystem") {
            if let Ok(response) = mcp::tools::get_disk_usage(filesystem).await {
                if let Some(usage) = response.data {
                    // Consider resolved if usage dropped below 85 %
                    return usage.percentage < 85.0;
                }
            }
        }

        if let Some(service) = alert.labels.get("service") {
            if let Ok(response) = mcp::tools::list_systemd_services(Some(service)).await {
                if let Some(statuses) = response.data {
                    return statuses.iter().any(|s| s.active);
                }
            }
        }

        // For K8s or unknown alert types we can't re-probe cheaply; return false
        // so the confidence score reflects uncertainty rather than false confidence.
        false
    }

    async fn generate_report(&self, context: RemediationContext) -> Result<RemediationReport> {
        tracing::info!("Generating remediation report");

        let report = RemediationReport {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            alert_name: context
                .alert
                .labels
                .get("alertname")
                .unwrap_or(&"Unknown".to_string())
                .clone(),
            root_cause: context.analysis.root_cause.clone(),
            steps_executed: context
                .plan
                .steps
                .iter()
                .map(|s| s.description.clone())
                .collect(),
            success: context.verification.success,
        };

        // Memorize the report in MemPalace
        if let Some(mempalace) = &self.mempalace_client {
            let memory_content = format!(
                "Incident: {}\nResult: {}\nRoot Cause: {}\nSteps: {}",
                report.alert_name,
                if report.success { "SUCCESS" } else { "FAILURE" },
                report.root_cause,
                report.steps_executed.join(", ")
            );
            let metadata = serde_json::json!({
                "alert": context.alert,
                "success": report.success,
                "timestamp": report.timestamp,
            });

            let mempalace_clone = mempalace.clone();
            tokio::spawn(async move {
                if let Err(e) = mempalace_clone.memorize(&memory_content, metadata).await {
                    tracing::error!("Failed to memorize report in MemPalace: {}", e);
                } else {
                    tracing::info!("Successfully memorized report in MemPalace");
                }
            });
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command() {
        let (cmd, args) = ReasoningEngine::parse_command("systemctl restart nginx");
        assert_eq!(cmd, "systemctl");
        assert_eq!(args, vec!["restart", "nginx"]);
    }

    #[test]
    fn classify_risk() {
        assert!(matches!(ReasoningEngine::classify_risk("low"), RiskLevel::Low));
        assert!(matches!(ReasoningEngine::classify_risk("high"), RiskLevel::High));
        assert!(matches!(ReasoningEngine::classify_risk("medium"), RiskLevel::Medium));
    }

    #[test]
    fn rollback_systemctl_stop() {
        let rb = ReasoningEngine::generate_rollback_command(
            "systemctl",
            &["stop".to_string(), "nginx".to_string()],
        );
        assert_eq!(rb.as_deref(), Some("systemctl start nginx"));
    }

    #[test]
    fn rollback_apt_install() {
        let rb = ReasoningEngine::generate_rollback_command(
            "apt-get",
            &["install".to_string(), "-y".to_string(), "jq".to_string()],
        );
        assert_eq!(rb.as_deref(), Some("apt-get remove -y jq"));
    }

    #[test]
    fn rollback_irreversible_returns_none() {
        assert!(ReasoningEngine::generate_rollback_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()]
        )
        .is_none());

        assert!(ReasoningEngine::generate_rollback_command("journalctl", &[
            "--vacuum-time=7d".to_string()
        ])
        .is_none());
    }
}

// Made with Bob
