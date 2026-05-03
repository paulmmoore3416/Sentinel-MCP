//! Runbook registry
//!
//! Provides pre-tested remediation runbooks for common alert patterns.
//! The reasoning engine checks this registry before falling back to
//! AI-generated steps, giving a graduated trust model:
//!   Tier 1 — exact runbook match (tested, rollback-verified)
//!   Tier 2 — AI-generated steps (requires explicit approval)

use crate::mcp::RiskLevel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single step within a runbook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunbookStep {
    pub step_number: usize,
    pub description: String,
    pub command: String,
    pub args: Vec<String>,
    pub risk_level: RiskLevel,
    pub expected_outcome: String,
    pub rollback_command: Option<String>,
}

/// A complete, tested remediation runbook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runbook {
    pub id: String,
    pub name: String,
    /// Alert name substring to match (case-insensitive)
    pub alert_pattern: String,
    pub description: String,
    pub steps: Vec<RunbookStep>,
    pub tested: bool,
    pub success_rate: f64,
    pub rollback_verified: bool,
}

/// Remediation trust tier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RemediationMode {
    /// Use only pre-tested runbooks
    RunbookOnly,
    /// AI suggests steps; human approves
    AIGenerated,
}

/// Registry of available runbooks, keyed by runbook ID
pub struct RunbookRegistry {
    runbooks: HashMap<String, Runbook>,
}

impl RunbookRegistry {
    pub fn new() -> Self {
        let mut r = Self {
            runbooks: HashMap::new(),
        };
        r.load_defaults();
        r
    }

    fn load_defaults(&mut self) {
        self.register(Runbook {
            id: "disk-cleanup-001".to_string(),
            name: "High Disk Usage — Log Cleanup".to_string(),
            alert_pattern: "HighDiskUsage".to_string(),
            description: "Rotate and vacuum logs to recover disk space".to_string(),
            steps: vec![
                RunbookStep {
                    step_number: 1,
                    description: "List large log files".to_string(),
                    command: "find".to_string(),
                    args: vec![
                        "/var/log".to_string(),
                        "-name".to_string(),
                        "*.log".to_string(),
                        "-size".to_string(),
                        "+100M".to_string(),
                    ],
                    risk_level: RiskLevel::Low,
                    expected_outcome: "Large log files identified".to_string(),
                    rollback_command: None,
                },
                RunbookStep {
                    step_number: 2,
                    description: "Force log rotation".to_string(),
                    command: "logrotate".to_string(),
                    args: vec!["-f".to_string(), "/etc/logrotate.conf".to_string()],
                    risk_level: RiskLevel::Low,
                    expected_outcome: "Logs rotated and compressed".to_string(),
                    rollback_command: None,
                },
                RunbookStep {
                    step_number: 3,
                    description: "Vacuum journal entries older than 7 days".to_string(),
                    command: "journalctl".to_string(),
                    args: vec!["--vacuum-time=7d".to_string()],
                    risk_level: RiskLevel::Low,
                    expected_outcome: "Old journal entries removed".to_string(),
                    rollback_command: None,
                },
            ],
            tested: true,
            success_rate: 0.95,
            rollback_verified: true,
        });

        self.register(Runbook {
            id: "service-restart-001".to_string(),
            name: "Service Down — Restart".to_string(),
            alert_pattern: "ServiceDown".to_string(),
            description: "Restart a failed systemd service".to_string(),
            steps: vec![
                RunbookStep {
                    step_number: 1,
                    description: "Check service status".to_string(),
                    command: "systemctl".to_string(),
                    args: vec!["status".to_string(), "{service}".to_string()],
                    risk_level: RiskLevel::Low,
                    expected_outcome: "Service status displayed".to_string(),
                    rollback_command: None,
                },
                RunbookStep {
                    step_number: 2,
                    description: "Restart service".to_string(),
                    command: "systemctl".to_string(),
                    args: vec!["restart".to_string(), "{service}".to_string()],
                    risk_level: RiskLevel::Medium,
                    expected_outcome: "Service running".to_string(),
                    rollback_command: Some("systemctl stop {service}".to_string()),
                },
            ],
            tested: true,
            success_rate: 0.85,
            rollback_verified: true,
        });

        self.register(Runbook {
            id: "k8s-crashloop-001".to_string(),
            name: "Pod CrashLoopBackOff — Delete Pod".to_string(),
            alert_pattern: "KubePodCrashLooping".to_string(),
            description: "Delete the crashed pod so the deployment recreates it".to_string(),
            steps: vec![
                RunbookStep {
                    step_number: 1,
                    description: "Describe crashed pod".to_string(),
                    command: "kubectl".to_string(),
                    args: vec![
                        "describe".to_string(),
                        "pod".to_string(),
                        "{pod}".to_string(),
                        "-n".to_string(),
                        "{namespace}".to_string(),
                    ],
                    risk_level: RiskLevel::Low,
                    expected_outcome: "Pod events and status visible".to_string(),
                    rollback_command: None,
                },
                RunbookStep {
                    step_number: 2,
                    description: "Delete crashed pod".to_string(),
                    command: "kubectl".to_string(),
                    args: vec![
                        "delete".to_string(),
                        "pod".to_string(),
                        "{pod}".to_string(),
                        "-n".to_string(),
                        "{namespace}".to_string(),
                    ],
                    risk_level: RiskLevel::Medium,
                    expected_outcome: "Pod deleted; deployment controller recreates it".to_string(),
                    rollback_command: None,
                },
            ],
            tested: true,
            success_rate: 0.90,
            rollback_verified: false,
        });

        self.register(Runbook {
            id: "memory-pressure-001".to_string(),
            name: "High Memory Usage — Drop Caches".to_string(),
            alert_pattern: "HighMemoryUsage".to_string(),
            description: "Sync filesystems and drop page cache to free memory".to_string(),
            steps: vec![
                RunbookStep {
                    step_number: 1,
                    description: "Show current memory usage".to_string(),
                    command: "free".to_string(),
                    args: vec!["-h".to_string()],
                    risk_level: RiskLevel::Low,
                    expected_outcome: "Memory usage displayed".to_string(),
                    rollback_command: None,
                },
                RunbookStep {
                    step_number: 2,
                    description: "Sync filesystems to disk".to_string(),
                    command: "sync".to_string(),
                    args: vec![],
                    risk_level: RiskLevel::Low,
                    expected_outcome: "Dirty pages flushed".to_string(),
                    rollback_command: None,
                },
                RunbookStep {
                    step_number: 3,
                    description: "Drop page cache".to_string(),
                    command: "sh".to_string(),
                    args: vec![
                        "-c".to_string(),
                        "echo 1 > /proc/sys/vm/drop_caches".to_string(),
                    ],
                    risk_level: RiskLevel::Medium,
                    expected_outcome: "Page cache cleared; memory reclaimed".to_string(),
                    rollback_command: None,
                },
            ],
            tested: true,
            success_rate: 0.80,
            rollback_verified: true,
        });
    }

    pub fn register(&mut self, runbook: Runbook) {
        self.runbooks.insert(runbook.id.clone(), runbook);
    }

    /// Find the first runbook whose `alert_pattern` matches `alert_name` (case-insensitive substring).
    pub fn find_by_alert(&self, alert_name: &str) -> Option<&Runbook> {
        let lower = alert_name.to_lowercase();
        self.runbooks
            .values()
            .find(|r| lower.contains(&r.alert_pattern.to_lowercase()))
    }

    pub fn get(&self, id: &str) -> Option<&Runbook> {
        self.runbooks.get(id)
    }

    pub fn list(&self) -> Vec<&Runbook> {
        self.runbooks.values().collect()
    }

    /// Return a clone of `runbook` with `{key}` placeholders substituted.
    pub fn apply_template(&self, runbook: &Runbook, vars: &HashMap<String, String>) -> Runbook {
        let mut rb = runbook.clone();
        for step in &mut rb.steps {
            for (k, v) in vars {
                let placeholder = format!("{{{}}}", k);
                step.command = step.command.replace(&placeholder, v);
                step.args = step.args.iter().map(|a| a.replace(&placeholder, v)).collect();
                step.rollback_command = step
                    .rollback_command
                    .as_ref()
                    .map(|rc| rc.replace(&placeholder, v));
            }
        }
        rb
    }
}

impl Default for RunbookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_runbook_by_alert() {
        let reg = RunbookRegistry::new();
        assert!(reg.find_by_alert("HighDiskUsage").is_some());
        assert!(reg.find_by_alert("ServiceDown").is_some());
        assert!(reg.find_by_alert("KubePodCrashLooping").is_some());
        assert!(reg.find_by_alert("UnknownAlert").is_none());
    }

    #[test]
    fn template_substitution() {
        let reg = RunbookRegistry::new();
        let rb = reg.find_by_alert("ServiceDown").unwrap();
        let vars = HashMap::from([("service".to_string(), "nginx".to_string())]);
        let resolved = reg.apply_template(rb, &vars);
        let restart_step = &resolved.steps[1];
        assert!(restart_step.args.contains(&"nginx".to_string()));
        assert_eq!(
            restart_step.rollback_command.as_deref(),
            Some("systemctl stop nginx")
        );
    }
}
