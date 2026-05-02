# Prompt 2: MCP Tools Implementation

## Context
Now that we have the project scaffold, we need to implement the core MCP tools that will allow Bob to interact with the infrastructure.

## Prompt for Bob (Code Mode)

```
Bob, let's implement the MCP tools for Sentinel-MCP. I need you to create the following tools in src/mcp/tools.rs:

1. **read_system_logs**
   - Parameters: source (syslog/journalctl/file), lines (number of lines), filter (optional regex)
   - Functionality: Read logs from various sources
   - Return: JSON with log lines and metadata
   - Security: Validate file paths, prevent directory traversal

2. **execute_remediation_script**
   - Parameters: command (string), args (array), dry_run (boolean)
   - Functionality: Execute approved commands with security validation
   - Return: Exit code, stdout, stderr
   - Security: 
     - Check against whitelist/blacklist
     - Require approval for high-risk commands
     - Log all executions to audit trail

3. **check_kubernetes_status**
   - Parameters: resource_type (pod/deployment/service), namespace, name (optional)
   - Functionality: Query Kubernetes cluster state
   - Return: Resource status, events, logs
   - Security: Limit to allowed namespaces

4. **get_disk_usage**
   - Parameters: path (filesystem path)
   - Functionality: Get disk usage statistics
   - Return: Total, used, available, percentage
   - Security: Validate path exists and is accessible

5. **list_systemd_services**
   - Parameters: filter (optional service name pattern)
   - Functionality: List systemd services and their status
   - Return: Array of services with name, status, enabled state
   - Security: Read-only operation

Each tool should:
- Use async/await with tokio
- Include comprehensive error handling
- Log all operations with tracing
- Return results in a consistent JSON format
- Include security validation

Also create src/mcp/security.rs with:
- Command whitelist/blacklist
- Risk level classification (low/medium/high)
- Approval requirement logic
- Audit logging functions

Use this structure for tool responses:
```rust
#[derive(Serialize, Deserialize)]
pub struct ToolResponse {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
    pub metadata: ToolMetadata,
}

#[derive(Serialize, Deserialize)]
pub struct ToolMetadata {
    pub tool_name: String,
    pub execution_time_ms: u64,
    pub timestamp: String,
    pub requires_approval: bool,
    pub risk_level: RiskLevel,
}
```

Please implement these tools with production-quality error handling and security validation.
```

## Expected Output

Bob should create:
1. `src/mcp/tools.rs` with all 5 tool implementations
2. `src/mcp/security.rs` with security validation logic
3. `src/mcp/mod.rs` to export the modules
4. Proper error types and handling
5. Comprehensive logging

## Validation

After implementation, test each tool:
```bash
# Test read_system_logs
curl -X POST http://localhost:3000/mcp/tools/read_system_logs \
  -H "Content-Type: application/json" \
  -d '{"source": "syslog", "lines": 50}'

# Test get_disk_usage
curl -X POST http://localhost:3000/mcp/tools/get_disk_usage \
  -H "Content-Type: application/json" \
  -d '{"path": "/var"}'
```

## Next Steps

After Bob completes this, proceed to Prompt 3 for watsonx.ai integration.