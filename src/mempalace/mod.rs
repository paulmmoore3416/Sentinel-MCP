//! MemPalace integration module
//! 
//! Integrates with MemPalace MCP server to provide long-term semantic memory
//! and "Self-Learning" runbooks for Sentinel-MCP.

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MemPalaceClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

impl MemPalaceClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            base_url: base_url.to_string(),
        }
    }

    /// Query MemPalace for past incidents or tribal knowledge
    pub async fn recall(&self, query: &str) -> Result<String> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "mempalace_recall".to_string(),
            params: serde_json::json!({
                "query": query,
                "limit": 3
            }),
        };

        // Standard MCP HTTP transport or MemPalace custom REST bridge
        let response = self.client
            .post(format!("{}/rpc", self.base_url))
            .json(&req)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("MemPalace HTTP error: {}", response.status()));
        }

        let rpc_res: JsonRpcResponse = response.json().await?;
        
        if let Some(err) = rpc_res.error {
            return Err(anyhow!("MemPalace RPC error: {:?}", err));
        }

        if let Some(result) = rpc_res.result {
            if let Some(memories) = result.get("memories").and_then(|m| m.as_array()) {
                let context: Vec<String> = memories.iter()
                    .filter_map(|m| m.get("content").and_then(|c| c.as_str()).map(|s| s.to_string()))
                    .collect();
                return Ok(context.join("\n\n"));
            }
        }

        Ok(String::new())
    }

    /// Store a completed remediation report in MemPalace
    pub async fn memorize(&self, content: &str, metadata: serde_json::Value) -> Result<()> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 2,
            method: "mempalace_memorize".to_string(),
            params: serde_json::json!({
                "content": content,
                "metadata": metadata
            }),
        };

        let response = self.client
            .post(format!("{}/rpc", self.base_url))
            .json(&req)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("MemPalace HTTP error: {}", response.status()));
        }

        let rpc_res: JsonRpcResponse = response.json().await?;
        
        if let Some(err) = rpc_res.error {
            return Err(anyhow!("MemPalace RPC error: {:?}", err));
        }

        Ok(())
    }
}
