//! MCP (Model Context Protocol) server implementation
//! 
//! This module provides the MCP tools that allow Bob to interact with
//! the infrastructure, including reading logs, executing commands, and
//! querying system state.

pub mod security;
pub mod tools;

pub use security::SecurityValidator;
pub use tools::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// MCP tool response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub metadata: ToolMetadata,
}

/// Metadata for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub tool_name: String,
    pub execution_time_ms: u64,
    pub timestamp: String,
    pub requires_approval: bool,
    pub risk_level: RiskLevel,
}

/// Risk level classification for operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl<T> ToolResponse<T> {
    pub fn success(data: T, metadata: ToolMetadata) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            metadata,
        }
    }

    pub fn error(error: String, metadata: ToolMetadata) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            metadata,
        }
    }
}

// Made with Bob
