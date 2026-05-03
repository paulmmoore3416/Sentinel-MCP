//! MCP tools implementation
//! 
//! This module implements the actual MCP tools that interact with the system.
//! Each tool is designed to be safe, auditable, and provide rich context.

use super::{RiskLevel, ToolMetadata, ToolResponse};
use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

/// System logs response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemLogs {
    pub source: String,
    pub lines: Vec<String>,
    pub total_lines: usize,
    pub filtered: bool,
}

/// Disk usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub path: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub percentage: f64,
}

/// Service status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub status: String,
    pub enabled: bool,
    pub active: bool,
}

/// Command execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// Kubernetes resource status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sStatus {
    pub resource_type: String,
    pub namespace: String,
    pub name: Option<String>,
    pub status: String,
    pub details: serde_json::Value,
}

/// Read system logs from various sources
pub async fn read_system_logs(
    source: &str,
    lines: usize,
    filter: Option<&str>,
) -> Result<ToolResponse<SystemLogs>> {
    let start = std::time::Instant::now();
    
    tracing::info!("Reading {} lines from {}", lines, source);
    
    // Validate source
    let log_lines = match source {
        "syslog" => read_syslog(lines).await?,
        "journalctl" => read_journalctl(lines).await?,
        path if path.starts_with('/') => read_file_logs(path, lines).await?,
        _ => return Err(anyhow!("Invalid log source: {}", source)),
    };
    
    // Apply filter if provided
    let (filtered_lines, is_filtered) = if let Some(pattern) = filter {
        let regex = Regex::new(pattern)?;
        let filtered: Vec<String> = log_lines
            .into_iter()
            .filter(|line| regex.is_match(line))
            .collect();
        (filtered, true)
    } else {
        (log_lines, false)
    };
    
    let duration = start.elapsed().as_millis() as u64;
    
    let data = SystemLogs {
        source: source.to_string(),
        lines: filtered_lines.clone(),
        total_lines: filtered_lines.len(),
        filtered: is_filtered,
    };
    
    let metadata = ToolMetadata {
        tool_name: "read_system_logs".to_string(),
        execution_time_ms: duration,
        timestamp: chrono::Utc::now().to_rfc3339(),
        requires_approval: false,
        risk_level: RiskLevel::Low,
    };
    
    Ok(ToolResponse::success(data, metadata))
}

/// Read syslog
async fn read_syslog(lines: usize) -> Result<Vec<String>> {
    let output = Command::new("tail")
        .args(&["-n", &lines.to_string(), "/var/log/syslog"])
        .output()
        .await?;
    
    if !output.status.success() {
        return Err(anyhow!("Failed to read syslog"));
    }
    
    let content = String::from_utf8_lossy(&output.stdout);
    Ok(content.lines().map(|s| s.to_string()).collect())
}

/// Read journalctl logs
async fn read_journalctl(lines: usize) -> Result<Vec<String>> {
    let output = Command::new("journalctl")
        .args(&["-n", &lines.to_string(), "--no-pager"])
        .output()
        .await?;
    
    if !output.status.success() {
        return Err(anyhow!("Failed to read journalctl"));
    }
    
    let content = String::from_utf8_lossy(&output.stdout);
    Ok(content.lines().map(|s| s.to_string()).collect())
}

/// Read logs from a file
async fn read_file_logs(path: &str, lines: usize) -> Result<Vec<String>> {
    // Security: Validate path
    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return Err(anyhow!("Log file does not exist: {}", path));
    }
    
    // Prevent directory traversal
    if path.contains("..") {
        return Err(anyhow!("Directory traversal not allowed"));
    }
    
    let output = Command::new("tail")
        .args(&["-n", &lines.to_string(), path])
        .output()
        .await?;
    
    if !output.status.success() {
        return Err(anyhow!("Failed to read log file"));
    }
    
    let content = String::from_utf8_lossy(&output.stdout);
    Ok(content.lines().map(|s| s.to_string()).collect())
}

/// Execute a remediation command
pub async fn execute_remediation_script(
    command: &str,
    args: &[String],
    dry_run: bool,
) -> Result<ToolResponse<ExecutionResult>> {
    let start = std::time::Instant::now();
    
    tracing::info!("Executing command: {} {:?} (dry_run: {})", command, args, dry_run);
    
    // Security validation
    let validator = super::SecurityValidator::new();
    let full_command = format!("{} {}", command, args.join(" "));
    let risk_level = validator.validate_command(&full_command)?;
    
    let result = if dry_run {
        // Dry run - don't actually execute
        ExecutionResult {
            success: true,
            exit_code: 0,
            stdout: format!("[DRY RUN] Would execute: {}", full_command),
            stderr: String::new(),
            duration_ms: 0,
        }
    } else {
        // Execute the command
        let output = Command::new(command)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;
        
        ExecutionResult {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
        }
    };
    
    let duration = start.elapsed().as_millis() as u64;
    
    let metadata = ToolMetadata {
        tool_name: "execute_remediation_script".to_string(),
        execution_time_ms: duration,
        timestamp: chrono::Utc::now().to_rfc3339(),
        requires_approval: validator.requires_approval(risk_level),
        risk_level,
    };
    
    Ok(ToolResponse::success(result, metadata))
}

/// Get disk usage for a path
pub async fn get_disk_usage(path: &str) -> Result<ToolResponse<DiskUsage>> {
    let start = std::time::Instant::now();
    
    tracing::info!("Getting disk usage for: {}", path);
    
    // Validate path exists
    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return Err(anyhow!("Path does not exist: {}", path));
    }
    
    // Use df command to get disk usage
    let output = Command::new("df")
        .args(&["-B1", path]) // -B1 for bytes
        .output()
        .await?;
    
    if !output.status.success() {
        return Err(anyhow!("Failed to get disk usage"));
    }
    
    let content = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = content.lines().collect();
    
    if lines.len() < 2 {
        return Err(anyhow!("Unexpected df output"));
    }
    
    // Parse df output (skip header line)
    let parts: Vec<&str> = lines[1].split_whitespace().collect();
    if parts.len() < 6 {
        return Err(anyhow!("Failed to parse df output"));
    }
    
    let total_bytes: u64 = parts[1].parse()?;
    let used_bytes: u64 = parts[2].parse()?;
    let available_bytes: u64 = parts[3].parse()?;
    let percentage: f64 = parts[4].trim_end_matches('%').parse()?;
    
    let duration = start.elapsed().as_millis() as u64;
    
    let data = DiskUsage {
        path: path.to_string(),
        total_bytes,
        used_bytes,
        available_bytes,
        percentage,
    };
    
    let metadata = ToolMetadata {
        tool_name: "get_disk_usage".to_string(),
        execution_time_ms: duration,
        timestamp: chrono::Utc::now().to_rfc3339(),
        requires_approval: false,
        risk_level: RiskLevel::Low,
    };
    
    Ok(ToolResponse::success(data, metadata))
}

/// List systemd services
pub async fn list_systemd_services(
    filter: Option<&str>,
) -> Result<ToolResponse<Vec<ServiceStatus>>> {
    let start = std::time::Instant::now();
    
    tracing::info!("Listing systemd services (filter: {:?})", filter);
    
    // Get list of services
    let output = Command::new("systemctl")
        .args(&["list-units", "--type=service", "--all", "--no-pager", "--plain"])
        .output()
        .await?;
    
    if !output.status.success() {
        return Err(anyhow!("Failed to list services"));
    }
    
    let content = String::from_utf8_lossy(&output.stdout);
    let mut services = Vec::new();
    
    for line in content.lines().skip(1) {
        // Skip header
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        
        let service_name = parts[0].to_string();
        
        // Apply filter if provided
        if let Some(pattern) = filter {
            if !service_name.contains(pattern) {
                continue;
            }
        }
        
        // Get detailed status for this service
        let status_output = Command::new("systemctl")
            .args(&["is-active", &service_name])
            .output()
            .await?;
        
        let enabled_output = Command::new("systemctl")
            .args(&["is-enabled", &service_name])
            .output()
            .await?;
        
        let status = String::from_utf8_lossy(&status_output.stdout).trim().to_string();
        let enabled = String::from_utf8_lossy(&enabled_output.stdout).trim() == "enabled";
        let active = status == "active";
        
        services.push(ServiceStatus {
            name: service_name,
            status,
            enabled,
            active,
        });
    }
    
    let duration = start.elapsed().as_millis() as u64;
    
    let metadata = ToolMetadata {
        tool_name: "list_systemd_services".to_string(),
        execution_time_ms: duration,
        timestamp: chrono::Utc::now().to_rfc3339(),
        requires_approval: false,
        risk_level: RiskLevel::Low,
    };
    
    Ok(ToolResponse::success(services, metadata))
}

/// Check Kubernetes resource status
pub async fn check_kubernetes_status(
    resource_type: &str,
    namespace: &str,
    name: Option<&str>,
) -> Result<ToolResponse<K8sStatus>> {
    let start = std::time::Instant::now();
    
    tracing::info!(
        "Checking Kubernetes {} in namespace {} (name: {:?})",
        resource_type,
        namespace,
        name
    );
    
    // Build kubectl command
    let mut args = vec!["get", resource_type, "-n", namespace, "-o", "json"];
    if let Some(n) = name {
        args.push(n);
    }
    
    let output = Command::new("kubectl")
        .args(&args)
        .output()
        .await?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to query Kubernetes: {}", error));
    }
    
    let content = String::from_utf8_lossy(&output.stdout);
    let details: serde_json::Value = serde_json::from_str(&content)?;
    
    // Extract status
    let status = if let Some(items) = details.get("items") {
        format!("Found {} resources", items.as_array().map(|a| a.len()).unwrap_or(0))
    } else if let Some(status) = details.get("status") {
        status.get("phase")
            .and_then(|p| p.as_str())
            .unwrap_or("Unknown")
            .to_string()
    } else {
        "Unknown".to_string()
    };
    
    let duration = start.elapsed().as_millis() as u64;
    
    let data = K8sStatus {
        resource_type: resource_type.to_string(),
        namespace: namespace.to_string(),
        name: name.map(|s| s.to_string()),
        status,
        details,
    };
    
    let metadata = ToolMetadata {
        tool_name: "check_kubernetes_status".to_string(),
        execution_time_ms: duration,
        timestamp: chrono::Utc::now().to_rfc3339(),
        requires_approval: false,
        risk_level: RiskLevel::Low,
    };
    
    Ok(ToolResponse::success(data, metadata))
}

/// Network diagnostics information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDiagnostics {
    pub endpoint: String,
    pub reachable: bool,
    pub output: String,
}

pub async fn run_network_diagnostics(endpoint: &str) -> Result<ToolResponse<NetworkDiagnostics>> {
    let start = std::time::Instant::now();
    tracing::info!("Running network diagnostics for: {}", endpoint);
    
    // Basic ping test
    let output = Command::new("ping")
        .args(&["-c", "3", "-W", "5", endpoint])
        .output()
        .await?;
        
    let reachable = output.status.success();
    let content = String::from_utf8_lossy(&output.stdout).to_string();
    
    let duration = start.elapsed().as_millis() as u64;
    let data = NetworkDiagnostics {
        endpoint: endpoint.to_string(),
        reachable,
        output: content,
    };
    
    let metadata = ToolMetadata {
        tool_name: "run_network_diagnostics".to_string(),
        execution_time_ms: duration,
        timestamp: chrono::Utc::now().to_rfc3339(),
        requires_approval: false,
        risk_level: RiskLevel::Low,
    };
    Ok(ToolResponse::success(data, metadata))
}

/// Database metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseMetrics {
    pub connection_pool_usage: f64,
    pub status: String,
}

pub async fn check_db_metrics() -> Result<ToolResponse<DatabaseMetrics>> {
    let start = std::time::Instant::now();
    tracing::info!("Checking database metrics");
    
    // Mock implementation for database connection pool check
    let data = DatabaseMetrics {
        connection_pool_usage: 95.0,
        status: "High connection pool usage detected".to_string(),
    };
    
    let duration = start.elapsed().as_millis() as u64;
    let metadata = ToolMetadata {
        tool_name: "check_db_metrics".to_string(),
        execution_time_ms: duration,
        timestamp: chrono::Utc::now().to_rfc3339(),
        requires_approval: false,
        risk_level: RiskLevel::Low,
    };
    Ok(ToolResponse::success(data, metadata))
}

/// Node Diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDiagnostics {
    pub node_name: String,
    pub events: String,
    pub status: String,
}

pub async fn describe_node(node_name: &str) -> Result<ToolResponse<NodeDiagnostics>> {
    let start = std::time::Instant::now();
    tracing::info!("Describing Kubernetes node: {}", node_name);
    
    let output = Command::new("kubectl")
        .args(&["describe", "node", node_name])
        .output()
        .await?;
        
    let content = String::from_utf8_lossy(&output.stdout).to_string();
    
    let duration = start.elapsed().as_millis() as u64;
    let data = NodeDiagnostics {
        node_name: node_name.to_string(),
        events: content.clone(),
        status: if output.status.success() { "Described".to_string() } else { "Failed".to_string() },
    };
    
    let metadata = ToolMetadata {
        tool_name: "describe_node".to_string(),
        execution_time_ms: duration,
        timestamp: chrono::Utc::now().to_rfc3339(),
        requires_approval: false,
        risk_level: RiskLevel::Low,
    };
    Ok(ToolResponse::success(data, metadata))
}

/// TLS Certificate Verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsVerification {
    pub endpoint: String,
    pub days_until_expiry: i64,
    pub output: String,
}

pub async fn check_tls_certificate(endpoint: &str) -> Result<ToolResponse<TlsVerification>> {
    let start = std::time::Instant::now();
    tracing::info!("Checking TLS certificate for: {}", endpoint);
    
    // Mock implementation for checking TLS certificate
    let data = TlsVerification {
        endpoint: endpoint.to_string(),
        days_until_expiry: 7,
        output: "Certificate expires in 7 days".to_string(),
    };
    
    let duration = start.elapsed().as_millis() as u64;
    let metadata = ToolMetadata {
        tool_name: "check_tls_certificate".to_string(),
        execution_time_ms: duration,
        timestamp: chrono::Utc::now().to_rfc3339(),
        requires_approval: false,
        risk_level: RiskLevel::Low,
    };
    Ok(ToolResponse::success(data, metadata))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_disk_usage() {
        let result = get_disk_usage("/").await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.success);
        assert!(response.data.is_some());
    }

    #[tokio::test]
    async fn test_execute_dry_run() {
        let result = execute_remediation_script(
            "echo",
            &["test".to_string()],
            true
        ).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.success);
        assert!(response.data.unwrap().stdout.contains("DRY RUN"));
    }
}

// Made with Bob
