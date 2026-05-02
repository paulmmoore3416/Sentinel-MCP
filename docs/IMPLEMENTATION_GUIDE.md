# Sentinel-MCP Implementation Guide

## Overview

This guide provides step-by-step instructions for implementing Sentinel-MCP using IBM Bob. The project is designed to be built incrementally, with each phase building upon the previous one.

## Prerequisites

Before starting, ensure you have:

- ✅ Rust 1.75+ installed (`rustup` recommended)
- ✅ Docker and Docker Compose
- ✅ Kubernetes cluster access (for K8s features)
- ✅ IBM Cloud account with watsonx.ai access
- ✅ IBM Bob access (via VS Code extension or CLI)
- ✅ Git configured with GitHub access

## Implementation Phases

### Phase 1: Project Foundation (Week 1)

**Goal**: Set up the basic project structure and dependencies

#### Step 1.1: Initialize Project with Bob

Use the prompt from [`prompts/01-scaffold.md`](../prompts/01-scaffold.md):

```bash
# Open Bob in Orchestrator Mode
# Copy and paste the prompt from 01-scaffold.md
```

**Expected Deliverables**:
- `Cargo.toml` with dependencies
- Basic directory structure
- `main.rs` with server initialization
- `.env.example` configuration template

**Validation**:
```bash
cargo build
cargo test
```

#### Step 1.2: Set Up Environment

Create `.env` file:
```bash
cp .env.example .env
# Edit .env with your credentials
```

Required variables:
```env
WATSONX_API_KEY=your_api_key
WATSONX_PROJECT_ID=your_project_id
WATSONX_URL=https://us-south.ml.cloud.ibm.com
MCP_SERVER_PORT=3000
MCP_AUTH_TOKEN=generate_secure_token
```

#### Step 1.3: Verify Setup

```bash
cargo run
# Should start server on port 3000
curl http://localhost:3000/api/v1/health
```

---

### Phase 2: Core MCP Tools (Week 1-2)

**Goal**: Implement the MCP tools for system operations

#### Step 2.1: Implement MCP Tools

Use the prompt from [`prompts/02-mcp-tools.md`](../prompts/02-mcp-tools.md):

```bash
# Switch Bob to Code Mode
# Copy and paste the prompt from 02-mcp-tools.md
```

**Expected Deliverables**:
- `src/mcp/tools.rs` - All 5 MCP tools
- `src/mcp/security.rs` - Security validation
- `src/mcp/mod.rs` - Module exports

**Testing Each Tool**:

1. **read_system_logs**:
```bash
curl -X POST http://localhost:3000/mcp/tools/read_system_logs \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${MCP_AUTH_TOKEN}" \
  -d '{"source": "syslog", "lines": 50}'
```

2. **get_disk_usage**:
```bash
curl -X POST http://localhost:3000/mcp/tools/get_disk_usage \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${MCP_AUTH_TOKEN}" \
  -d '{"path": "/var"}'
```

3. **list_systemd_services**:
```bash
curl -X POST http://localhost:3000/mcp/tools/list_systemd_services \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${MCP_AUTH_TOKEN}" \
  -d '{"filter": "nginx"}'
```

#### Step 2.2: Implement Security Validation

Test security rules:
```bash
# Should succeed (low risk)
curl -X POST http://localhost:3000/mcp/tools/execute_remediation_script \
  -d '{"command": "systemctl restart nginx", "dry_run": true}'

# Should fail (high risk)
curl -X POST http://localhost:3000/mcp/tools/execute_remediation_script \
  -d '{"command": "rm -rf /", "dry_run": true}'
```

---

### Phase 3: watsonx.ai Integration (Week 2)

**Goal**: Integrate IBM watsonx.ai for intelligent analysis

#### Step 3.1: Implement watsonx Client

Use the prompt from [`prompts/03-watsonx.md`](../prompts/03-watsonx.md):

```bash
# Bob in Code Mode
# Copy and paste the prompt from 03-watsonx.md
```

**Expected Deliverables**:
- `src/watsonx/mod.rs` - Main client
- `src/watsonx/prompts.rs` - Prompt templates
- `src/watsonx/types.rs` - Request/response types

#### Step 3.2: Test watsonx Integration

Create test file `tests/watsonx_integration.rs`:
```rust
#[tokio::test]
async fn test_watsonx_connection() {
    let client = WatsonxClient::new().unwrap();
    let result = client.analyze_logs(
        "ERROR: Disk space critical",
        "{}"
    ).await;
    assert!(result.is_ok());
}
```

Run test:
```bash
cargo test test_watsonx_connection
```

#### Step 3.3: Verify API Calls

Check logs for successful API calls:
```bash
tail -f logs/sentinel-mcp.log | grep watsonx
```

---

### Phase 4: Reasoning Engine (Week 2-3)

**Goal**: Implement the orchestration and workflow logic

#### Step 4.1: Design Workflow

Use the prompt from [`prompts/04-reasoning-engine.md`](../prompts/04-reasoning-engine.md) Part A:

```bash
# Bob in Plan Mode
# Copy and paste Part A from 04-reasoning-engine.md
```

Review Bob's workflow design and approve.

#### Step 4.2: Implement Reasoning Engine

Use Part B of the same prompt:

```bash
# Bob in Code Mode
# Copy and paste Part B from 04-reasoning-engine.md
```

**Expected Deliverables**:
- `src/reasoning/mod.rs` - Main engine
- `src/reasoning/workflow.rs` - State machine
- `src/reasoning/types.rs` - Workflow types

#### Step 4.3: Test Workflow

Create integration test:
```rust
#[tokio::test]
async fn test_full_workflow() {
    let engine = ReasoningEngine::new(test_config()).await.unwrap();
    let alert = create_test_alert();
    let result = engine.process_alert(alert).await;
    assert!(result.is_ok());
}
```

---

### Phase 5: Alert Receiver & Documentation (Week 3)

**Goal**: Implement alert handling and auto-documentation

#### Step 5.1: Implement Alert Receiver

Use the prompt from [`prompts/05-alert-and-docs.md`](../prompts/05-alert-and-docs.md):

```bash
# Bob in Code Mode
# Copy and paste the prompt from 05-alert-and-docs.md
```

**Expected Deliverables**:
- `src/alert/mod.rs` - HTTP server
- `src/alert/parser.rs` - Alert parsing
- `src/executor/documentation.rs` - Doc generator

#### Step 5.2: Test Alert Reception

Send test alert:
```bash
curl -X POST http://localhost:3000/api/v1/alerts \
  -H "Content-Type: application/json" \
  -d @examples/alerts/disk-space-low.json
```

Verify processing:
```bash
curl http://localhost:3000/api/v1/status
```

#### Step 5.3: Verify Documentation

Check generated report:
```bash
ls -la logs/remediations/
cat logs/remediations/REMEDIATION_LOG_*.md
```

---

### Phase 6: Testing & Demo (Week 3-4)

**Goal**: Create comprehensive tests and demo materials

#### Step 6.1: Implement Test Suite

Use the prompt from [`prompts/06-testing-and-demo.md`](../prompts/06-testing-and-demo.md):

```bash
# Bob in Code Mode
# Copy and paste the prompt from 06-testing-and-demo.md
```

**Expected Deliverables**:
- Unit tests in `tests/unit/`
- Integration tests in `tests/integration/`
- Failure injection scripts in `scripts/`
- Demo script in `scripts/demo.sh`

#### Step 6.2: Run Test Suite

```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --out Html

# View coverage report
open tarpaulin-report.html
```

#### Step 6.3: Test Demo Scenarios

```bash
# Make scripts executable
chmod +x scripts/*.sh

# Run demo
./scripts/demo.sh
```

---

### Phase 7: Deployment & Documentation (Week 4)

**Goal**: Deploy to Kubernetes and finalize documentation

#### Step 7.1: Create Kubernetes Manifests

Create `k8s/deployment.yaml`:
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sentinel-mcp
  namespace: sentinel-system
spec:
  replicas: 1
  selector:
    matchLabels:
      app: sentinel-mcp
  template:
    metadata:
      labels:
        app: sentinel-mcp
    spec:
      serviceAccountName: sentinel-mcp
      containers:
      - name: sentinel-mcp
        image: sentinel-mcp:latest
        ports:
        - containerPort: 3000
        env:
        - name: WATSONX_API_KEY
          valueFrom:
            secretKeyRef:
              name: watsonx-credentials
              key: api-key
        - name: WATSONX_PROJECT_ID
          valueFrom:
            secretKeyRef:
              name: watsonx-credentials
              key: project-id
```

#### Step 7.2: Deploy to Kubernetes

```bash
# Create namespace
kubectl create namespace sentinel-system

# Create secrets
kubectl create secret generic watsonx-credentials \
  --from-literal=api-key=${WATSONX_API_KEY} \
  --from-literal=project-id=${WATSONX_PROJECT_ID} \
  -n sentinel-system

# Deploy
kubectl apply -f k8s/

# Verify
kubectl get pods -n sentinel-system
kubectl logs -f deployment/sentinel-mcp -n sentinel-system
```

#### Step 7.3: Configure Prometheus Integration

Update AlertManager config:
```yaml
receivers:
  - name: 'sentinel-mcp'
    webhook_configs:
      - url: 'http://sentinel-mcp.sentinel-system.svc.cluster.local:3000/api/v1/alerts'
        send_resolved: true
```

---

## Troubleshooting

### Common Issues

#### Issue: watsonx.ai API Authentication Failed

**Solution**:
```bash
# Verify credentials
echo $WATSONX_API_KEY
echo $WATSONX_PROJECT_ID

# Test API directly
curl -X POST "${WATSONX_URL}/ml/v1/text/generation?version=2023-05-29" \
  -H "Authorization: Bearer ${WATSONX_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"model_id": "ibm/granite-13b-instruct-v2", "input": "test"}'
```

#### Issue: MCP Tools Not Working

**Solution**:
```bash
# Check permissions
ls -la /var/log/
sudo chmod +r /var/log/syslog

# Test tool directly
cargo test test_read_system_logs -- --nocapture
```

#### Issue: Kubernetes Deployment Fails

**Solution**:
```bash
# Check RBAC
kubectl get serviceaccount sentinel-mcp -n sentinel-system
kubectl describe clusterrole sentinel-mcp

# Check secrets
kubectl get secret watsonx-credentials -n sentinel-system -o yaml
```

---

## Best Practices

### Development Workflow

1. **Use Bob Incrementally**: Don't try to build everything at once
2. **Test After Each Phase**: Verify each component works before moving on
3. **Document as You Go**: Update docs with any deviations from plan
4. **Commit Frequently**: Use meaningful commit messages

### Code Quality

1. **Run Clippy**: `cargo clippy -- -D warnings`
2. **Format Code**: `cargo fmt`
3. **Check Coverage**: Aim for >80% test coverage
4. **Review Security**: Audit all command execution paths

### Security Considerations

1. **Never commit secrets**: Use `.env` and `.gitignore`
2. **Validate all inputs**: Especially commands and file paths
3. **Use least privilege**: Kubernetes RBAC should be minimal
4. **Audit all actions**: Log everything to audit trail

---

## Success Metrics

Track these metrics to measure success:

- **MTTR**: Mean Time to Recovery (target: <5 minutes)
- **Automation Rate**: % of alerts handled automatically (target: >70%)
- **Success Rate**: % of successful remediations (target: >95%)
- **Documentation**: % of incidents documented (target: 100%)

---

## Next Steps After Implementation

1. **Create Video Demo**: Follow [`docs/VIDEO_DEMO_SCRIPT.md`](VIDEO_DEMO_SCRIPT.md)
2. **Export Bob Report**: Document all Bob interactions
3. **Write Blog Post**: Share your experience
4. **Submit to Hackathon**: Prepare submission materials

---

## Support

If you encounter issues:

1. Check the [troubleshooting section](#troubleshooting)
2. Review Bob's suggestions and error messages
3. Check project documentation in `/docs`
4. Review example code in `/examples`

---

## Timeline Summary

| Week | Phase | Deliverables |
|------|-------|--------------|
| 1 | Foundation & MCP Tools | Project scaffold, basic tools |
| 2 | watsonx & Reasoning | AI integration, workflow engine |
| 3 | Alerts & Docs | Alert receiver, documentation |
| 4 | Testing & Deploy | Tests, demo, K8s deployment |

Total estimated time: **4 weeks** for full implementation

---

**Remember**: This is an iterative process. Use Bob to help you at each step, and don't hesitate to ask for clarification or improvements!