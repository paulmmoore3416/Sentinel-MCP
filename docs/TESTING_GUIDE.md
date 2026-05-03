# Sentinel-MCP Testing Guide

This guide covers testing and demonstration of Sentinel-MCP's autonomous remediation capabilities.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Running Tests](#running-tests)
3. [Demo Scenarios](#demo-scenarios)
4. [Failure Injection](#failure-injection)
5. [Manual Testing](#manual-testing)
6. [Troubleshooting](#troubleshooting)

## Quick Start

### Prerequisites

```bash
# Install required tools
sudo apt-get update
sudo apt-get install -y curl jq stress

# Optional: For Kubernetes scenarios
# Install kubectl and configure cluster access
```

### Start Sentinel-MCP

```bash
# Set up environment
cp .env.example .env
# Edit .env with your IBM watsonx.ai credentials

# Build and run
cargo build --release
RUST_LOG=info cargo run --release
```

In another terminal, verify it's running:

```bash
curl http://localhost:3000/api/v1/health | jq
```

## Running Tests

### Unit Tests

Run all unit tests:

```bash
cargo test
```

Run specific test module:

```bash
cargo test --lib mcp::security
cargo test --lib watsonx
cargo test --lib reasoning
```

### Integration Tests

```bash
# Run all integration tests
cargo test --test '*'

# Run specific integration test
cargo test --test full_workflow
```

### Test Coverage

Generate test coverage report:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --out Html --output-dir coverage/
```

View the report:

```bash
open coverage/index.html
```

## Demo Scenarios

### Interactive Demo Script

The demo script provides an interactive way to test Sentinel-MCP:

```bash
./scripts/demo.sh
```

**Available Scenarios:**

1. **💾 Disk Space Crisis**
   - Simulates disk space alert
   - Shows log analysis and cleanup remediation
   - Demonstrates auto-documentation

2. **🔧 Service Crash Recovery**
   - Simulates service failure
   - Shows service restart remediation
   - Demonstrates approval workflow

3. **☸️ Kubernetes Pod Failure**
   - Simulates pod crash loop
   - Shows K8s integration
   - Demonstrates pod remediation

4. **📊 View Recent Reports**
   - Lists all remediation reports
   - Shows latest report details

5. **🧪 Send Custom Alert**
   - Create custom alert payload
   - Test with your own scenarios

### Scenario 1: Disk Space Crisis

**Step-by-Step:**

```bash
# Terminal 1: Start Sentinel-MCP
RUST_LOG=info cargo run --release

# Terminal 2: Watch logs
tail -f logs/sentinel-mcp.log

# Terminal 3: Run demo
./scripts/demo.sh
# Select option 1
```

**What Happens:**
1. Alert sent to Sentinel-MCP
2. System gathers disk usage and logs
3. watsonx.ai analyzes root cause
4. Remediation plan generated
5. Safe cleanup commands executed
6. Report generated in `logs/remediations/`

**Expected Output:**
```
✓ Alert sent
✓ Context gathered
✓ Analysis complete: "Disk space exhausted due to old log files"
✓ Remediation plan: Rotate logs, clean temp files
✓ Execution successful
✓ Report generated: logs/remediations/REMEDIATION_LOG_20260502.md
```

### Scenario 2: Service Crash Recovery

```bash
# Run demo
./scripts/demo.sh
# Select option 2

# Or manually:
curl -X POST http://localhost:3000/api/v1/alerts \
  -H "Content-Type: application/json" \
  -d @examples/alerts/service-down.json
```

**What Happens:**
1. Service down alert received
2. Systemd status checked
3. Service logs analyzed
4. Restart command proposed
5. User approval requested (if configured)
6. Service restarted
7. Verification performed

### Scenario 3: Kubernetes Pod Failure

```bash
# Requires kubectl access
./scripts/demo.sh
# Select option 3

# Or manually:
curl -X POST http://localhost:3000/api/v1/alerts \
  -H "Content-Type: application/json" \
  -d @examples/alerts/pod-crashloop.json
```

**What Happens:**
1. Pod crash loop detected
2. K8s API queried for pod status
3. Pod logs analyzed
4. Root cause identified
5. Remediation strategy proposed
6. Pod restarted or recreated

## Failure Injection

The `test-failure.sh` script helps inject realistic failures for testing.

### Disk Full Scenario

```bash
# Inject failure
sudo ./scripts/test-failure.sh disk-full

# Check disk usage
df -h /tmp

# Trigger alert
curl -X POST http://localhost:3000/api/v1/alerts \
  -H "Content-Type: application/json" \
  -d @examples/alerts/disk-space-low.json

# Clean up
sudo ./scripts/test-failure.sh cleanup
```

### Service Crash Scenario

```bash
# Stop nginx service
sudo ./scripts/test-failure.sh service-crash nginx

# Check status
sudo systemctl status nginx

# Trigger alert
curl -X POST http://localhost:3000/api/v1/alerts \
  -H "Content-Type: application/json" \
  -d @examples/alerts/service-down.json

# Restore manually if needed
sudo systemctl start nginx
```

### Kubernetes Pod Crash

```bash
# Create crashloop pod
./scripts/test-failure.sh pod-crashloop

# Watch pod status
kubectl get pod crashloop-test -w

# Trigger alert
curl -X POST http://localhost:3000/api/v1/alerts \
  -H "Content-Type: application/json" \
  -d @examples/alerts/pod-crashloop.json

# Clean up
kubectl delete pod crashloop-test
```

### Memory Leak Simulation

```bash
# Start memory stress
./scripts/test-failure.sh memory-leak

# Monitor memory
watch -n 1 free -h

# Stop stress test
pkill stress
```

### High CPU Simulation

```bash
# Start CPU stress
./scripts/test-failure.sh high-cpu

# Monitor CPU
top
# or
htop

# Stop stress test
pkill stress
```

## Manual Testing

### Test Alert Receiver

```bash
# Send test alert
curl -X POST http://localhost:3000/api/v1/alerts \
  -H "Content-Type: application/json" \
  -d '{
    "version": "4",
    "status": "firing",
    "alerts": [{
      "status": "firing",
      "labels": {
        "alertname": "TestAlert",
        "severity": "warning"
      },
      "annotations": {
        "summary": "Test alert"
      },
      "startsAt": "2026-05-02T18:00:00Z",
      "fingerprint": "test123"
    }]
  }'
```

### Test MCP Tools

```bash
# Test in Rust
cargo test --lib mcp::tools

# Or use the demo to trigger tool usage
```

### Test watsonx.ai Integration

```bash
# Requires valid API credentials in .env
cargo test --lib watsonx -- --nocapture
```

### Test Documentation Generator

```bash
# Run reasoning engine test which generates reports
cargo test --lib reasoning -- --nocapture

# Check generated reports
ls -la logs/remediations/
cat logs/remediations/REMEDIATION_LOG.md
```

## Monitoring and Debugging

### View Logs

```bash
# Real-time logs
tail -f logs/sentinel-mcp.log

# Filter by level
tail -f logs/sentinel-mcp.log | grep ERROR
tail -f logs/sentinel-mcp.log | grep WARN

# View specific component
tail -f logs/sentinel-mcp.log | grep "reasoning"
tail -f logs/sentinel-mcp.log | grep "watsonx"
```

### Check System Status

```bash
# Get current status
curl http://localhost:3000/api/v1/status | jq

# Watch status continuously
watch -n 2 'curl -s http://localhost:3000/api/v1/status | jq'
```

### View Remediation Reports

```bash
# List all reports
ls -lht logs/remediations/

# View latest markdown report
cat logs/remediations/REMEDIATION_LOG.md

# View JSON report
cat logs/remediations/remediation_report.json | jq

# Open HTML report in browser
xdg-open logs/remediations/remediation_report.html
```

## Troubleshooting

### Sentinel-MCP Won't Start

```bash
# Check if port is in use
lsof -i :3000

# Check environment variables
cat .env

# Run with debug logging
RUST_LOG=debug cargo run
```

### watsonx.ai Connection Issues

```bash
# Verify credentials
echo $WATSONX_API_KEY
echo $WATSONX_PROJECT_ID

# Test API connectivity
curl -H "Authorization: Bearer $WATSONX_API_KEY" \
  https://us-south.ml.cloud.ibm.com/ml/v1/deployments
```

### Alerts Not Processing

```bash
# Check alert receiver status
curl http://localhost:3000/api/v1/status | jq

# Verify alert format
cat examples/alerts/disk-space-low.json | jq

# Check logs for errors
tail -f logs/sentinel-mcp.log | grep ERROR
```

### MCP Tools Failing

```bash
# Test individual tools
cargo test test_read_system_logs -- --nocapture
cargo test test_get_disk_usage -- --nocapture

# Check permissions
ls -la /var/log/syslog
```

### Reports Not Generated

```bash
# Check output directory exists
mkdir -p logs/remediations

# Check permissions
ls -la logs/

# Run with verbose logging
RUST_LOG=sentinel_mcp::executor=debug cargo run
```

## Performance Testing

### Load Test

```bash
# Send multiple concurrent alerts
for i in {1..10}; do
  curl -X POST http://localhost:3000/api/v1/alerts \
    -H "Content-Type: application/json" \
    -d @examples/alerts/disk-space-low.json &
done

# Monitor processing
watch -n 1 'curl -s http://localhost:3000/api/v1/status | jq'
```

### Stress Test

```bash
# High volume alert test
for i in {1..100}; do
  curl -s -X POST http://localhost:3000/api/v1/alerts \
    -H "Content-Type: application/json" \
    -d @examples/alerts/service-down.json > /dev/null &
  
  if [ $((i % 10)) -eq 0 ]; then
    echo "Sent $i alerts"
    sleep 1
  fi
done

echo "Load test complete"
```

## Best Practices

1. **Always test in dry-run mode first**
   ```bash
   # Set in .env
   DRY_RUN_MODE=true
   ```

2. **Monitor logs during testing**
   ```bash
   tail -f logs/sentinel-mcp.log
   ```

3. **Clean up after tests**
   ```bash
   ./scripts/test-failure.sh cleanup
   ```

4. **Verify remediation success**
   ```bash
   # Check system state after remediation
   df -h
   systemctl status nginx
   kubectl get pods
   ```

5. **Review generated reports**
   ```bash
   cat logs/remediations/REMEDIATION_LOG.md
   ```

## Next Steps

- Review [VIDEO_DEMO_SCRIPT.md](VIDEO_DEMO_SCRIPT.md) for recording demos
- Check [HACKATHON_SUBMISSION.md](HACKATHON_SUBMISSION.md) for submission checklist
- See [IMPLEMENTATION_GUIDE.md](IMPLEMENTATION_GUIDE.md) for development details

## Support

For issues or questions:
- Check logs: `logs/sentinel-mcp.log`
- Review documentation: `docs/`
- Open an issue on GitHub