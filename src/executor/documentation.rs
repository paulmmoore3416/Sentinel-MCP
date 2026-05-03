//! Documentation generator for remediation reports
//! 
//! This module generates comprehensive documentation of remediation
//! activities in multiple formats (Markdown, JSON, HTML).

use crate::reasoning::RemediationContext;
use anyhow::Result;
use std::fs;
use std::path::Path;

/// Documentation generator
pub struct DocumentationGenerator {
    output_dir: String,
}

impl DocumentationGenerator {
    /// Create a new documentation generator
    pub fn new(output_dir: String) -> Self {
        Self { output_dir }
    }

    /// Generate complete remediation report
    pub fn generate_report(&self, context: &RemediationContext) -> Result<String> {
        // Ensure output directory exists
        fs::create_dir_all(&self.output_dir)?;

        // Generate markdown report
        let markdown = self.generate_markdown(context)?;
        let markdown_path = format!("{}/REMEDIATION_LOG.md", self.output_dir);
        fs::write(&markdown_path, &markdown)?;

        // Generate JSON report
        let json = self.generate_json(context)?;
        let json_path = format!("{}/remediation_report.json", self.output_dir);
        fs::write(&json_path, &json)?;

        // Generate HTML report
        let html = self.generate_html(context)?;
        let html_path = format!("{}/remediation_report.html", self.output_dir);
        fs::write(&html_path, &html)?;

        Ok(markdown_path)
    }

    /// Generate markdown report
    fn generate_markdown(&self, context: &RemediationContext) -> Result<String> {
        let mut md = String::new();

        // Header
        md.push_str("# Sentinel-MCP Remediation Report\n\n");
        md.push_str(&format!("**Generated:** {}\n\n", chrono::Utc::now().to_rfc3339()));
        md.push_str("---\n\n");

        // Incident Details
        md.push_str("## 📋 Incident Details\n\n");
        md.push_str(&format!(
            "- **Alert Name:** {}\n",
            context.alert.labels.get("alertname").unwrap_or(&"Unknown".to_string())
        ));
        md.push_str(&format!(
            "- **Severity:** {}\n",
            context.alert.labels.get("severity").unwrap_or(&"medium".to_string())
        ));
        md.push_str(&format!("- **Started At:** {}\n", context.alert.starts_at));
        md.push_str(&format!("- **Fingerprint:** {}\n", context.alert.fingerprint));
        md.push_str("\n");

        // Alert Labels
        if !context.alert.labels.is_empty() {
            md.push_str("### Labels\n\n");
            for (key, value) in &context.alert.labels {
                md.push_str(&format!("- `{}`: {}\n", key, value));
            }
            md.push_str("\n");
        }

        // Alert Annotations
        if !context.alert.annotations.is_empty() {
            md.push_str("### Annotations\n\n");
            for (key, value) in &context.alert.annotations {
                md.push_str(&format!("- **{}:** {}\n", key, value));
            }
            md.push_str("\n");
        }

        // Root Cause Analysis
        md.push_str("## 🔍 Root Cause Analysis\n\n");
        md.push_str(&format!("**Root Cause:** {}\n\n", context.analysis.root_cause));
        md.push_str(&format!("**Impact:** {}\n\n", context.analysis.impact));
        md.push_str(&format!("**Urgency:** {}\n\n", context.analysis.urgency));

        if !context.analysis.affected_components.is_empty() {
            md.push_str("### Affected Components\n\n");
            for component in &context.analysis.affected_components {
                md.push_str(&format!("- {}\n", component));
            }
            md.push_str("\n");
        }

        // Remediation Steps
        md.push_str("## 🔧 Remediation Steps\n\n");
        for step in &context.plan.steps {
            md.push_str(&format!("### Step {}: {}\n\n", step.step_number, step.description));
            md.push_str(&format!("**Command:** `{} {}`\n\n", step.command, step.args.join(" ")));
            md.push_str(&format!("**Risk Level:** {:?}\n\n", step.risk_level));
            md.push_str(&format!("**Expected Outcome:** {}\n\n", step.expected_outcome));
        }

        // Execution Results
        md.push_str("## ✅ Execution Results\n\n");
        md.push_str(&format!(
            "**Status:** {}\n\n",
            if context.execution_result.success {
                "✅ Success"
            } else {
                "❌ Failed"
            }
        ));
        md.push_str(&format!(
            "**Duration:** {}ms\n\n",
            context.execution_result.duration_ms
        ));
        md.push_str(&format!("**Exit Code:** {}\n\n", context.execution_result.exit_code));

        if !context.execution_result.stdout.is_empty() {
            md.push_str("### Standard Output\n\n");
            md.push_str("```\n");
            md.push_str(&context.execution_result.stdout);
            md.push_str("\n```\n\n");
        }

        if !context.execution_result.stderr.is_empty() {
            md.push_str("### Standard Error\n\n");
            md.push_str("```\n");
            md.push_str(&context.execution_result.stderr);
            md.push_str("\n```\n\n");
        }

        // Verification
        md.push_str("## 🔬 Verification\n\n");
        md.push_str(&format!(
            "**Verification Status:** {}\n\n",
            if context.verification.success {
                "✅ Passed"
            } else {
                "❌ Failed"
            }
        ));
        md.push_str(&format!("**Details:** {}\n\n", context.verification.details));

        if !context.verification.metrics.is_empty() {
            md.push_str("### Metrics\n\n");
            for (key, value) in &context.verification.metrics {
                md.push_str(&format!("- **{}:** {}\n", key, value));
            }
            md.push_str("\n");
        }

        // Footer
        md.push_str("---\n\n");
        md.push_str("*Generated by Sentinel-MCP - Autonomous Infrastructure Repair Agent*\n");

        Ok(md)
    }

    /// Generate JSON report
    fn generate_json(&self, context: &RemediationContext) -> Result<String> {
        let json = serde_json::to_string_pretty(context)?;
        Ok(json)
    }

    /// Generate HTML report
    fn generate_html(&self, context: &RemediationContext) -> Result<String> {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n");
        html.push_str("<html lang=\"en\">\n");
        html.push_str("<head>\n");
        html.push_str("    <meta charset=\"UTF-8\">\n");
        html.push_str("    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
        html.push_str("    <title>Sentinel-MCP Remediation Report</title>\n");
        html.push_str("    <style>\n");
        html.push_str("        body { font-family: Arial, sans-serif; max-width: 1200px; margin: 0 auto; padding: 20px; background: #f5f5f5; }\n");
        html.push_str("        .container { background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }\n");
        html.push_str("        h1 { color: #2c3e50; border-bottom: 3px solid #3498db; padding-bottom: 10px; }\n");
        html.push_str("        h2 { color: #34495e; margin-top: 30px; }\n");
        html.push_str("        .success { color: #27ae60; font-weight: bold; }\n");
        html.push_str("        .failed { color: #e74c3c; font-weight: bold; }\n");
        html.push_str("        .info-box { background: #ecf0f1; padding: 15px; border-radius: 5px; margin: 10px 0; }\n");
        html.push_str("        .step { background: #f8f9fa; padding: 15px; margin: 10px 0; border-left: 4px solid #3498db; }\n");
        html.push_str("        code { background: #2c3e50; color: #ecf0f1; padding: 2px 6px; border-radius: 3px; }\n");
        html.push_str("        pre { background: #2c3e50; color: #ecf0f1; padding: 15px; border-radius: 5px; overflow-x: auto; }\n");
        html.push_str("    </style>\n");
        html.push_str("</head>\n");
        html.push_str("<body>\n");
        html.push_str("    <div class=\"container\">\n");

        // Header
        html.push_str("        <h1>🛡️ Sentinel-MCP Remediation Report</h1>\n");
        html.push_str(&format!(
            "        <p><strong>Generated:</strong> {}</p>\n",
            chrono::Utc::now().to_rfc3339()
        ));

        // Incident Details
        html.push_str("        <h2>📋 Incident Details</h2>\n");
        html.push_str("        <div class=\"info-box\">\n");
        html.push_str(&format!(
            "            <p><strong>Alert Name:</strong> {}</p>\n",
            context.alert.labels.get("alertname").unwrap_or(&"Unknown".to_string())
        ));
        html.push_str(&format!(
            "            <p><strong>Severity:</strong> {}</p>\n",
            context.alert.labels.get("severity").unwrap_or(&"medium".to_string())
        ));
        html.push_str(&format!("            <p><strong>Started At:</strong> {}</p>\n", context.alert.starts_at));
        html.push_str("        </div>\n");

        // Root Cause
        html.push_str("        <h2>🔍 Root Cause Analysis</h2>\n");
        html.push_str("        <div class=\"info-box\">\n");
        html.push_str(&format!("            <p><strong>Root Cause:</strong> {}</p>\n", context.analysis.root_cause));
        html.push_str(&format!("            <p><strong>Impact:</strong> {}</p>\n", context.analysis.impact));
        html.push_str(&format!("            <p><strong>Urgency:</strong> {}</p>\n", context.analysis.urgency));
        html.push_str("        </div>\n");

        // Remediation Steps
        html.push_str("        <h2>🔧 Remediation Steps</h2>\n");
        for step in &context.plan.steps {
            html.push_str("        <div class=\"step\">\n");
            html.push_str(&format!("            <h3>Step {}: {}</h3>\n", step.step_number, step.description));
            html.push_str(&format!("            <p><code>{} {}</code></p>\n", step.command, step.args.join(" ")));
            html.push_str(&format!("            <p><strong>Risk Level:</strong> {:?}</p>\n", step.risk_level));
            html.push_str("        </div>\n");
        }

        // Execution Results
        html.push_str("        <h2>✅ Execution Results</h2>\n");
        html.push_str("        <div class=\"info-box\">\n");
        html.push_str(&format!(
            "            <p class=\"{}\">Status: {}</p>\n",
            if context.execution_result.success { "success" } else { "failed" },
            if context.execution_result.success { "Success" } else { "Failed" }
        ));
        html.push_str(&format!("            <p><strong>Duration:</strong> {}ms</p>\n", context.execution_result.duration_ms));
        html.push_str("        </div>\n");

        // Footer
        html.push_str("        <hr>\n");
        html.push_str("        <p><em>Generated by Sentinel-MCP - Autonomous Infrastructure Repair Agent</em></p>\n");
        html.push_str("    </div>\n");
        html.push_str("</body>\n");
        html.push_str("</html>\n");

        Ok(html)
    }

    /// Generate summary report for multiple remediations
    pub fn generate_summary(&self, contexts: &[RemediationContext]) -> Result<String> {
        let mut md = String::new();

        md.push_str("# Sentinel-MCP Summary Report\n\n");
        md.push_str(&format!("**Generated:** {}\n\n", chrono::Utc::now().to_rfc3339()));
        md.push_str(&format!("**Total Remediations:** {}\n\n", contexts.len()));

        let successful = contexts.iter().filter(|c| c.verification.success).count();
        let failed = contexts.len() - successful;

        md.push_str(&format!("- ✅ Successful: {}\n", successful));
        md.push_str(&format!("- ❌ Failed: {}\n\n", failed));

        md.push_str("## Recent Remediations\n\n");
        for context in contexts.iter().take(10) {
            md.push_str(&format!(
                "- **{}** - {} - {}\n",
                context.alert.labels.get("alertname").unwrap_or(&"Unknown".to_string()),
                context.alert.starts_at,
                if context.verification.success { "✅" } else { "❌" }
            ));
        }

        Ok(md)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert::Alert;
    use crate::executor::ExecutionResult;
    use crate::reasoning::{RemediationContext, RemediationPlan, RemediationStep, VerificationResult};
    use crate::watsonx::Analysis;
    use crate::mcp::RiskLevel;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_context() -> RemediationContext {
        RemediationContext {
            alert: Alert {
                status: "firing".to_string(),
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("alertname".to_string(), "DiskSpaceLow".to_string());
                    labels.insert("severity".to_string(), "warning".to_string());
                    labels
                },
                annotations: HashMap::new(),
                starts_at: "2024-01-01T00:00:00Z".to_string(),
                ends_at: None,
                generator_url: String::new(),
                fingerprint: "test123".to_string(),
            },
            analysis: Analysis {
                root_cause: "Log files filling disk".to_string(),
                impact: "Disk space at 95%".to_string(),
                urgency: "high".to_string(),
                affected_components: vec!["filesystem".to_string()],
                lessons_learned: None,
            },
            plan: RemediationPlan {
                alert_name: "DiskSpaceLow".to_string(),
                analysis: Analysis {
                    root_cause: "Log files filling disk".to_string(),
                    impact: "Disk space at 95%".to_string(),
                    urgency: "high".to_string(),
                    affected_components: vec!["filesystem".to_string()],
                    lessons_learned: None,
                },
                steps: vec![RemediationStep {
                    step_number: 1,
                    description: "Rotate logs".to_string(),
                    command: "logrotate".to_string(),
                    args: vec!["-f".to_string(), "/etc/logrotate.conf".to_string()],
                    risk_level: RiskLevel::Low,
                    expected_outcome: "Logs rotated successfully".to_string(),
                    rollback_command: None,
                }],
                estimated_duration_seconds: 30,
                from_runbook: true,
            },
            execution_result: ExecutionResult {
                success: true,
                exit_code: 0,
                stdout: "Logs rotated successfully".to_string(),
                stderr: String::new(),
                duration_ms: 1500,
            },
            verification: VerificationResult {
                success: true,
                details: "Disk space reduced to 75%".to_string(),
                metrics: HashMap::new(),
                alert_resolved: true,
                confidence_score: 0.95,
            },
        }
    }

    #[test]
    fn test_generate_markdown() {
        let temp_dir = TempDir::new().unwrap();
        let generator = DocumentationGenerator::new(temp_dir.path().to_str().unwrap().to_string());
        let context = create_test_context();

        let markdown = generator.generate_markdown(&context).unwrap();
        assert!(markdown.contains("# Sentinel-MCP Remediation Report"));
        assert!(markdown.contains("DiskSpaceLow"));
        assert!(markdown.contains("Log files filling disk"));
    }

    #[test]
    fn test_generate_json() {
        let temp_dir = TempDir::new().unwrap();
        let generator = DocumentationGenerator::new(temp_dir.path().to_str().unwrap().to_string());
        let context = create_test_context();

        let json = generator.generate_json(&context).unwrap();
        assert!(json.contains("DiskSpaceLow"));
        assert!(json.contains("root_cause"));
    }

    #[test]
    fn test_generate_html() {
        let temp_dir = TempDir::new().unwrap();
        let generator = DocumentationGenerator::new(temp_dir.path().to_str().unwrap().to_string());
        let context = create_test_context();

        let html = generator.generate_html(&context).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Sentinel-MCP Remediation Report"));
        assert!(html.contains("DiskSpaceLow"));
    }
}

// Made with Bob
