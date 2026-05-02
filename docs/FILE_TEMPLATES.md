# File Templates for Sentinel-MCP

This document contains templates for all non-markdown files needed in the project. When implementing with Bob in Code mode, use these templates as starting points.

---

## .env.example

```env
# IBM watsonx.ai Configuration
WATSONX_API_KEY=your_api_key_here
WATSONX_PROJECT_ID=your_project_id_here
WATSONX_URL=https://us-south.ml.cloud.ibm.com
WATSONX_MODEL=ibm/granite-13b-instruct-v2

# MCP Server Configuration
MCP_SERVER_PORT=3000
MCP_AUTH_TOKEN=generate_secure_token_here
MCP_LOG_LEVEL=info

# Security Settings
APPROVAL_REQUIRED=true
AUTO_APPROVE_LOW_RISK=true
AUTO_APPROVE_MEDIUM_RISK=false
DRY_RUN_MODE=false
MAX_EXECUTION_TIME_SECONDS=300
APPROVAL_TIMEOUT_SECONDS=300

# Kubernetes Configuration (optional)
KUBECONFIG=/home/user/.kube/config
K8S_NAMESPACE=default

# Logging
LOG_DIR=./logs
LOG_FILE=sentinel-mcp.log
REMEDIATION_LOG_DIR=./logs/remediations

# Prometheus/AlertManager (optional)
PROMETHEUS_URL=http://localhost:9090
ALERTMANAGER_URL=http://localhost:9093
```

See actual file: `.env.example` in project root

---

## Usage Instructions

When implementing with Bob in Code mode:

1. **Copy the template** for the file you need
2. **Paste it into Bob's prompt** with context about what you're building
3. **Ask Bob to customize** it for your specific needs
4. **Review and test** the generated file

Example prompt:
```
Bob, create a .env.example file for Sentinel-MCP using this template:
[paste template here]

Customize it to include all necessary environment variables for our watsonx.ai integration.
```

All actual configuration files are created in the project structure.