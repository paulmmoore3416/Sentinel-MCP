//! watsonx.ai integration module
//! 
//! This module provides integration with IBM watsonx.ai for intelligent
//! log analysis and remediation suggestions using IBM Granite models.

pub mod prompts;

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// watsonx.ai client
pub struct WatsonxClient {
    api_key: String,
    project_id: String,
    base_url: String,
    client: Client,
    model_id: String,
}

/// Analysis result from watsonx.ai
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analysis {
    pub root_cause: String,
    pub affected_components: Vec<String>,
    pub impact: String,
    pub urgency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lessons_learned: Option<String>,
}

/// Remediation suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationSuggestion {
    pub step: usize,
    pub description: String,
    pub command: String,
    pub expected_outcome: String,
    pub risk_level: String,
}

/// watsonx.ai API request
#[derive(Debug, Serialize)]
struct WatsonxRequest {
    model_id: String,
    input: String,
    parameters: WatsonxParameters,
    project_id: String,
}

/// watsonx.ai generation parameters
#[derive(Debug, Serialize)]
struct WatsonxParameters {
    max_new_tokens: u32,
    temperature: f32,
    top_p: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

/// watsonx.ai API response
#[derive(Debug, Deserialize)]
struct WatsonxResponse {
    results: Vec<WatsonxResult>,
}

#[derive(Debug, Deserialize)]
struct WatsonxResult {
    generated_text: String,
}

impl WatsonxClient {
    /// Create a new watsonx.ai client
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("WATSONX_API_KEY")
            .map_err(|_| anyhow!("WATSONX_API_KEY not set"))?;
        let project_id = std::env::var("WATSONX_PROJECT_ID")
            .map_err(|_| anyhow!("WATSONX_PROJECT_ID not set"))?;
        let base_url = std::env::var("WATSONX_URL")
            .unwrap_or_else(|_| "https://us-south.ml.cloud.ibm.com".to_string());
        let model_id = std::env::var("WATSONX_MODEL")
            .unwrap_or_else(|_| "ibm/granite-13b-instruct-v2".to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;

        Ok(Self {
            api_key,
            project_id,
            base_url,
            client,
            model_id,
        })
    }

    /// Analyze logs using watsonx.ai
    pub async fn analyze_logs(&self, logs: &str, context: &str) -> Result<Analysis> {
        tracing::info!("Analyzing logs with watsonx.ai");

        // Parse context to extract system info
        let context_json: serde_json::Value = serde_json::from_str(context)
            .unwrap_or_else(|_| serde_json::json!({}));
        
        let system_type = context_json
            .get("system_type")
            .and_then(|v| v.as_str())
            .unwrap_or("Linux");
        let alert_name = context_json
            .get("alert_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        let severity = context_json
            .get("severity")
            .and_then(|v| v.as_str())
            .unwrap_or("medium");

        // Format the prompt
        let prompt = prompts::format_log_analysis_prompt(
            system_type,
            alert_name,
            severity,
            logs,
        );

        // Call watsonx.ai API with retry logic
        let response_text = self.call_api_with_retry(&prompt, 3).await?;

        // Parse the response
        self.parse_analysis_response(&response_text)
    }

    /// Get remediation suggestions
    pub async fn suggest_remediation(
        &self,
        root_cause: &str,
        system_state: &str,
    ) -> Result<Vec<RemediationSuggestion>> {
        tracing::info!("Getting remediation suggestions from watsonx.ai");

        // Format the prompt
        let prompt = prompts::format_remediation_prompt(root_cause, system_state);

        // Call watsonx.ai API with retry logic
        let response_text = self.call_api_with_retry(&prompt, 3).await?;

        // Parse the response
        self.parse_remediation_response(&response_text)
    }

    /// Call watsonx.ai API with retry logic
    async fn call_api_with_retry(&self, prompt: &str, max_retries: u32) -> Result<String> {
        let mut retries = 0;
        let mut delay = Duration::from_secs(1);

        loop {
            match self.call_api(prompt).await {
                Ok(response) => return Ok(response),
                Err(e) if retries < max_retries => {
                    tracing::warn!(
                        "watsonx.ai API call failed (attempt {}/{}): {}",
                        retries + 1,
                        max_retries,
                        e
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                    retries += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Call watsonx.ai API
    async fn call_api(&self, prompt: &str) -> Result<String> {
        let url = format!(
            "{}/ml/v1/text/generation?version=2023-05-29",
            self.base_url
        );

        let request = WatsonxRequest {
            model_id: self.model_id.clone(),
            input: prompt.to_string(),
            parameters: WatsonxParameters {
                max_new_tokens: 1024,
                temperature: 0.7,
                top_p: 0.9,
                stop_sequences: None,
            },
            project_id: self.project_id.clone(),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow!(
                "watsonx.ai API error ({}): {}",
                status,
                error_text
            ));
        }

        let watsonx_response: WatsonxResponse = response.json().await?;

        if watsonx_response.results.is_empty() {
            return Err(anyhow!("No results from watsonx.ai"));
        }

        Ok(watsonx_response.results[0].generated_text.clone())
    }

    /// Parse analysis response from watsonx.ai
    fn parse_analysis_response(&self, response: &str) -> Result<Analysis> {
        // Try to parse as JSON first
        if let Ok(analysis) = serde_json::from_str::<Analysis>(response) {
            return Ok(analysis);
        }

        // If JSON parsing fails, try to extract JSON from the response
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                let json_str = &response[start..=end];
                if let Ok(analysis) = serde_json::from_str::<Analysis>(json_str) {
                    return Ok(analysis);
                }
            }
        }

        // Fallback: Create a basic analysis from the text
        tracing::warn!("Failed to parse structured response, using fallback");
        Ok(Analysis {
            root_cause: response.lines().next().unwrap_or("Unknown").to_string(),
            affected_components: vec!["Unknown".to_string()],
            impact: "Unable to determine from response".to_string(),
            urgency: "medium".to_string(),
            lessons_learned: None,
        })
    }

    /// Parse remediation response from watsonx.ai
    fn parse_remediation_response(&self, response: &str) -> Result<Vec<RemediationSuggestion>> {
        // Try to parse as JSON array first
        if let Ok(suggestions) = serde_json::from_str::<Vec<RemediationSuggestion>>(response) {
            return Ok(suggestions);
        }

        // If JSON parsing fails, try to extract JSON array from the response
        if let Some(start) = response.find('[') {
            if let Some(end) = response.rfind(']') {
                let json_str = &response[start..=end];
                if let Ok(suggestions) = serde_json::from_str::<Vec<RemediationSuggestion>>(json_str) {
                    return Ok(suggestions);
                }
            }
        }

        // Fallback: Return empty suggestions
        tracing::warn!("Failed to parse remediation suggestions, returning empty list");
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_analysis_response() {
        let client = WatsonxClient {
            api_key: "test".to_string(),
            project_id: "test".to_string(),
            base_url: "test".to_string(),
            client: Client::new(),
            model_id: "test".to_string(),
        };

        let json_response = r#"{
            "root_cause": "Disk space exhausted",
            "affected_components": ["filesystem", "logging"],
            "impact": "Service degradation",
            "urgency": "high"
        }"#;

        let result = client.parse_analysis_response(json_response);
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert_eq!(analysis.root_cause, "Disk space exhausted");
        assert_eq!(analysis.urgency, "high");
    }

    #[test]
    fn test_parse_remediation_response() {
        let client = WatsonxClient {
            api_key: "test".to_string(),
            project_id: "test".to_string(),
            base_url: "test".to_string(),
            client: Client::new(),
            model_id: "test".to_string(),
        };

        let json_response = r#"[
            {
                "step": 1,
                "description": "Clean old logs",
                "command": "rm -f /var/log/old-logs/*",
                "expected_outcome": "Free up disk space",
                "risk_level": "medium"
            }
        ]"#;

        let result = client.parse_remediation_response(json_response);
        assert!(result.is_ok());
        let suggestions = result.unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].step, 1);
    }
}

// Made with Bob
