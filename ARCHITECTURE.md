# Sentinel-MCP Architecture Plan

## Executive Summary

Sentinel-MCP is an autonomous infrastructure repair agent that bridges monitoring alerts with intelligent remediation using IBM Bob and watsonx.ai. The system uses the Model Context Protocol (MCP) to enable Bob to interact with live infrastructure environments.

## System Architecture

```mermaid
graph TB
    A[Prometheus AlertManager] -->|Webhook| B[Alert Receiver]
    B --> C[Sentinel-MCP Core]
    C --> D[MCP Server]
    D --> E[System Tools]
    E --> F[Linux Operations]
    E --> G[Kubernetes Operations]
    C --> H[watsonx.ai Integration]
    H --> I[IBM Granite Models]
    C --> J[🧠 Reasoning Engine]
    J <--> N[📚 MemPalace (Long-Term Memory)]
    J --> K[Security Validator]
    K --> L[Remediation Executor]
    L --> M[Documentation Generator]
    M --> N
```

## Core Components

### 1. MCP Server (Rust)
**Purpose**: Expose system operations as MCP tools that Bob can invoke

**Key Tools**:
- `read_system_logs`: Read system logs (syslog, journalctl, application logs)
- `execute_remediation_script`: Execute approved remediation commands
- `check_kubernetes_status`: Query Kubernetes cluster state
- `get_disk_usage`: Check filesystem usage
- `list_systemd_services`: List and check service status
- `get_process_info`: Retrieve process information

**Security Features**:
- Command whitelist/blacklist
- Dry-run mode for all operations
- Audit logging for all tool invocations
- User approval workflow for destructive operations

### 2. Alert Receiver
**Purpose**: Accept and parse Prometheus AlertManager webhooks

**Functionality**:
- HTTP endpoint for AlertManager webhooks
- Alert normalization and enrichment
- Priority classification
- Alert deduplication

### 3. watsonx.ai Integration Module
**Purpose**: Leverage IBM Granite models for intelligent log analysis

**Capabilities**:
- Log summarization and pattern recognition
- Root cause analysis
- Remediation suggestion generation
- Historical incident correlation

### 4. MemPalace Integration
**Purpose**: Provide long-term, semantic memory for the agent.

**Capabilities**:
- **Recall**: Retrieve historical remediation reports relevant to a new alert.
- **Memorize**: Store the outcome of every new remediation, building a knowledge base over time.
- **Self-Learning**: Enables the agent to improve by consulting past successes and failures.

### 5. Reasoning Engine
**Purpose**: Orchestrate the analysis and remediation workflow

**Workflow**:
1. Receive alert from Alert Receiver
2. **Check circuit breaker** — block if alert type is in Open state
3. Gather context via MCP tools (logs, disk usage, service/K8s state)
4. **Recall from MemPalace** — query for historical context on similar alerts
5. Analyze context with watsonx.ai, now including historical data from MemPalace
6. **Check Runbook Registry** — use pre-tested runbook if alert pattern matches (Tier 1)
7. Fall back to AI-generated plan if no runbook match (Tier 2, requires approval)
8. **Generate rollback commands** for every reversible step
9. Validate remediation against security constraints
10. Request user approval for medium/high-risk steps
11. Execute remediation steps
12. **Deep verify** — re-probe disk/service state, calculate confidence score
13. **Update circuit breaker** — record success or failure; trip open after threshold
14. Generate documentation report and **memorize in MemPalace**

### 5. Security Validator
**Purpose**: Ensure all remediation actions are safe and authorized

**Validation Rules**:
- No destructive commands without approval
- No privilege escalation without review
- All database operations require explicit approval
- Kubernetes operations limited to specific namespaces

### 6. Remediation Executor
**Purpose**: Execute approved remediation actions safely

**Execution Modes**:
- **Dry-run**: Simulate execution and report expected changes
- **Interactive**: Execute with step-by-step confirmation
- **Autonomous**: Execute automatically for approved actions

### 7. Documentation Generator
**Purpose**: Auto-generate comprehensive remediation reports

## Technology Stack

### Core Technologies
- **Language**: Rust (for MCP server and core logic)
- **MCP SDK**: Rust MCP implementation
- **HTTP Server**: Axum (async Rust web framework)
- **Kubernetes Client**: kube-rs
- **System Operations**: tokio-process, sysinfo

### AI/ML Integration
- **IBM watsonx.ai**: IBM Granite models
- **HTTP Client**: reqwest (for API calls)

### Infrastructure
- **Container Runtime**: Docker
- **Orchestration**: Kubernetes (for deployment)
- **Monitoring**: Prometheus + AlertManager

## Repository Structure

```
sentinel-mcp/
├── src/
│   ├── main.rs                 # Entry point
│   ├── mcp/
│   │   ├── mod.rs              # MCP server implementation
│   │   ├── tools.rs            # MCP tool definitions
│   │   └── security.rs         # Security validator
│   ├── mempalace/
│   │   └── mod.rs              # MemPalace client for long-term memory
│   ├── alert/
│   │   └── mod.rs              # Alert receiver and deduplication
│   ├── watsonx/
│   │   ├── mod.rs              # watsonx.ai client (IBM Granite)
│   │   └── prompts.rs          # Prompt templates
│   ├── reasoning/
│   │   └── mod.rs              # Reasoning engine + state machine
│   ├── circuit_breaker/
│   │   └── mod.rs              # Per-alert-type circuit breakers
│   ├── runbook/
│   │   └── mod.rs              # Pre-tested runbook registry
│   ├── snapshot/
│   │   └── mod.rs              # System state capture for rollback
│   └── executor/
│       ├── mod.rs              # Remediation executor
│       └── documentation.rs    # Report generation
├── tests/
│   ├── integration/            # Integration tests
│   └── scenarios/              # Failure scenarios
├── prompts/
│   ├── 01-scaffold.md          # Bob prompt for scaffolding
│   ├── 02-mcp-tools.md         # Bob prompt for MCP tools
│   ├── 03-watsonx.md           # Bob prompt for watsonx integration
│   └── 04-testing.md           # Bob prompt for testing
├── docs/
│   ├── ARCHITECTURE.md         # This file
│   ├── API.md                  # API documentation
│   ├── SECURITY.md             # Security guidelines
│   └── bob-export.md           # IBM Bob exported report
├── k8s/
│   ├── deployment.yaml         # Kubernetes deployment
│   ├── service.yaml            # Kubernetes service
│   ├── rbac.yaml               # RBAC configuration
│   └── configmap.yaml          # Configuration
├── examples/
│   ├── alerts/                 # Example alert payloads
│   ├── logs/                   # Example log files
│   └── remediations/           # Example remediation logs
├── scripts/
│   ├── setup.sh                # Setup script
│   ├── demo.sh                 # Demo script
│   └── test-failure.sh         # Failure injection script
├── Cargo.toml                  # Rust dependencies
├── Dockerfile                  # Container image
├── README.md                   # Project documentation
└── LICENSE                     # License file
```

## Safety Architecture: Write Authority & Rollback Boundaries

Remediation agents need clean rollback boundaries when a diagnosis is wrong. Sentinel-MCP uses a **graduated trust model** and three defensive layers:

### Graduated Trust Model

```
Tier 1 — Runbook Registry   (src/runbook/)
  Exact alert-pattern match → pre-tested steps with rollback commands
  Success rate tracked per runbook (e.g. ServiceDown: 85 %)

Tier 2 — AI-Generated Steps  (src/reasoning/)
  No runbook match → watsonx.ai generates steps dynamically
  Requires explicit human approval for medium/high-risk ops
  Rollback commands auto-derived where possible (systemctl, apt-get, etc.)
```

### Circuit Breakers  (`src/circuit_breaker/`)

Each alert type has its own circuit breaker:
- **Closed** — normal execution allowed
- **Open** — execution blocked after N consecutive failures; alert escalated to human
- **HalfOpen** — single probe allowed after timeout; closes on success, re-opens on failure

### System Snapshots  (`src/snapshot/`)

Pre-execution snapshots capture filesystem metadata, service states, and K8s resources. The `SnapshotDiff` API compares before/after states to detect unexpected side effects and provide clean rollback data.

### Deep Verification  (`src/reasoning::verify_remediation`)

Post-execution verification re-probes the system (disk %, service active status) and reports a `confidence_score` (0–100 %). A score below 50 % counts as failure for circuit-breaker purposes even if the command exited 0.

---

## Implementation Timeline

### Phase 1: Foundation
- Project scaffolding
- MCP server basic implementation
- Alert receiver setup

### Phase 2: Core Logic
- watsonx.ai integration
- Reasoning engine
- Security validator

### Phase 3: Execution
- Remediation executor
- Documentation generator
- Rollback support

### Phase 4: Testing & Demo
- Test suite development
- Demo scenario preparation
- Documentation finalization