# Prompt 3: watsonx.ai Integration

## Context
We need to integrate IBM watsonx.ai to provide intelligent log analysis and remediation suggestions using IBM Granite models.

## Prompt for Bob (Code Mode)

```
Bob, implement the watsonx.ai integration module for Sentinel-MCP. Create src/watsonx/mod.rs with the following functionality:

1. **WatsonxClient struct**
   - Fields: api_key, project_id, base_url, http_client
   - Methods: new(), analyze_logs(), suggest_remediation()
   - Use reqwest for HTTP calls
   - Include retry logic with exponential backoff

2. **analyze_logs() method**
   - Input: log_content (string), context (system state)
   - Functionality: Send logs to IBM Granite model for analysis
   - Output: Root cause analysis, severity, affected components
   - Model: Use "ibm/granite-13b-instruct-v2"
   - Parameters: max_new_tokens=1024, temperature=0.7

3. **suggest_remediation() method**
   - Input: root_cause (string), system_state (JSON)
   - Functionality: Get remediation suggestions from Granite
   - Output: Ordered list of remediation steps with commands
   - Include confidence scores for each suggestion

4. **Prompt Engineering**
   Create src/watsonx/prompts.rs with optimized prompts:
   
   ```rust
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
   {
     "root_cause": "...",
     "affected_components": [...],
     "impact": "...",
     "urgency": "low|medium|high|critical"
   }
   "#;
   
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
     {
       "step": 1,
       "description": "...",
       "command": "...",
       "expected_outcome": "...",
       "risk_level": "low"
     }
   ]
   "#;
   ```

5. **Error Handling**
   - Handle API rate limits (429 errors)
   - Retry failed requests (max 3 attempts)
   - Fallback to cached responses if API unavailable
   - Log all API calls for debugging

6. **Configuration**
   Load from environment variables:
   - WATSONX_API_KEY
   - WATSONX_PROJECT_ID
   - WATSONX_URL (default: https://us-south.ml.cloud.ibm.com)
   - WATSONX_MODEL (default: ibm/granite-13b-instruct-v2)

7. **Response Parsing**
   Create robust JSON parsing with fallbacks:
   - Try to parse structured JSON first
   - Fall back to regex extraction if JSON parsing fails
   - Validate all required fields are present

Example usage:
```rust
let client = WatsonxClient::new()?;
let analysis = client.analyze_logs(
    &log_content,
    &system_context
).await?;

let remediation = client.suggest_remediation(
    &analysis.root_cause,
    &system_state
).await?;
```

Include comprehensive error types:
```rust
#[derive(Debug, thiserror::Error)]
pub enum WatsonxError {
    #[error("API request failed: {0}")]
    ApiError(String),
    
    #[error("Authentication failed")]
    AuthError,
    
    #[error("Rate limit exceeded")]
    RateLimitError,
    
    #[error("Invalid response format: {0}")]
    ParseError(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
}
```

Please implement this with production-quality error handling, retry logic, and comprehensive logging.
```

## Expected Output

Bob should create:
1. `src/watsonx/mod.rs` - Main client implementation
2. `src/watsonx/prompts.rs` - Prompt templates
3. `src/watsonx/types.rs` - Request/response types
4. Proper error handling with custom error types
5. Retry logic with exponential backoff
6. Comprehensive logging

## Testing

Create a test file to validate the integration:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_analyze_logs() {
        let client = WatsonxClient::new().unwrap();
        let logs = "ERROR: Disk space critical on /var";
        let result = client.analyze_logs(logs, "{}").await;
        assert!(result.is_ok());
    }
}
```

## Next Steps

After Bob completes this, proceed to Prompt 4 for the reasoning engine and workflow orchestration.