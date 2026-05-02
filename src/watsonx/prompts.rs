//! Prompt templates for watsonx.ai interactions
//! 
//! These prompts are optimized for IBM Granite models to provide
//! accurate log analysis and remediation suggestions.

/// Prompt template for log analysis
pub const LOG_ANALYSIS_PROMPT: &str = r#"
You are an expert Site Reliability Engineer analyzing infrastructure logs.

Context:
- System: {system_type}
- Alert: {alert_name}
- Severity: {severity}

Logs:
{log_content}

Analyze these logs and provide:
1. Root cause (be specific and technical)
2. Affected components
3. Impact assessment
4. Urgency level

Format your response as JSON:
{{
  "root_cause": "...",
  "affected_components": [...],
  "impact": "...",
  "urgency": "low|medium|high|critical"
}}
"#;

/// Prompt template for remediation suggestions
pub const REMEDIATION_PROMPT: &str = r#"
You are an expert SRE providing remediation steps.

Root Cause: {root_cause}
System State: {system_state}

Provide step-by-step remediation instructions.
For each step, include:
1. Description
2. Command to execute
3. Expected outcome
4. Risk level (low/medium/high)

Format as JSON array:
[
  {{
    "step": 1,
    "description": "...",
    "command": "...",
    "expected_outcome": "...",
    "risk_level": "low"
  }}
]
"#;

/// Helper function to format log analysis prompt
pub fn format_log_analysis_prompt(
    system_type: &str,
    alert_name: &str,
    severity: &str,
    log_content: &str,
) -> String {
    LOG_ANALYSIS_PROMPT
        .replace("{system_type}", system_type)
        .replace("{alert_name}", alert_name)
        .replace("{severity}", severity)
        .replace("{log_content}", log_content)
}

/// Helper function to format remediation prompt
pub fn format_remediation_prompt(root_cause: &str, system_state: &str) -> String {
    REMEDIATION_PROMPT
        .replace("{root_cause}", root_cause)
        .replace("{system_state}", system_state)
}

// Made with Bob
