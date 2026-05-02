# Prompt 1: Project Scaffolding

## Context
We're building Sentinel-MCP, an autonomous infrastructure repair agent that uses MCP (Model Context Protocol) to bridge IBM Bob with live system environments.

## Prompt for Bob (Orchestrator Mode)

```
Bob, help me scaffold a new Rust project for an MCP (Model Context Protocol) server called Sentinel-MCP. This server needs to:

1. Expose MCP tools for infrastructure operations:
   - read_system_logs: Read system logs from various sources
   - execute_remediation_script: Execute approved remediation commands
   - check_kubernetes_status: Query Kubernetes cluster state
   - get_disk_usage: Check filesystem usage
   - list_systemd_services: List and check systemd service status

2. Follow enterprise security standards:
   - Command validation and whitelisting
   - Audit logging for all operations
   - User approval workflow for destructive operations
   - Dry-run mode support

3. Use these Rust dependencies:
   - tokio (async runtime)
   - axum (HTTP server)
   - serde (JSON serialization)
   - kube (Kubernetes client)
   - sysinfo (system information)
   - tracing (logging)

4. Project structure:
   - src/main.rs: Entry point
   - src/mcp/: MCP server implementation
   - src/alert/: Alert receiver
   - src/watsonx/: watsonx.ai integration
   - src/reasoning/: Reasoning engine
   - src/executor/: Remediation executor

Please create:
- Cargo.toml with all necessary dependencies
- Basic project structure with module stubs
- .env.example with required environment variables
- .gitignore for Rust projects
- Basic main.rs that initializes the server

Focus on creating a solid foundation that we can build upon.
```

## Expected Output

Bob should create:
1. `Cargo.toml` with proper dependencies
2. Directory structure with module files
3. Basic `main.rs` with server initialization
4. `.env.example` with configuration template
5. `.gitignore` for Rust projects

## Next Steps

After Bob completes this, review the structure and proceed to Prompt 2 for implementing the MCP tools.