# Prompt 6: Testing and Demo Preparation

## Context
Create comprehensive tests and demo materials to showcase Sentinel-MCP's capabilities.

## Prompt for Bob (Code Mode)

```
Bob, create a comprehensive test suite and demo materials for Sentinel-MCP:

## Part A: Test Suite

1. **Unit Tests** (tests/unit/)

Create unit tests for each component:

```rust
// tests/unit/mcp_tools_test.rs
#[cfg(test)]
mod mcp_tools_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_read_system_logs() {
        let client = McpClient::new().unwrap();
        let result = client.read_system_logs("syslog", 10, None).await;
        assert!(result.is_ok());
        let logs = result.unwrap();
        assert!(logs.len() <= 10);
    }
    
    #[tokio::test]
    async fn test_get_disk_usage() {
        let client = McpClient::new().unwrap();
        let result = client.get_disk_usage("/").await;
        assert!(result.is_ok());
        let usage = result.unwrap();
        assert!(usage.percentage >= 0.0 && usage.percentage <= 100.0);
    }
    
    #[tokio::test]
    async fn test_command_validation() {
        let validator = SecurityValidator::new();
        
        // Should pass
        assert!(validator.validate_command("systemctl restart nginx").is_ok());
        
        // Should fail
        assert!(validator.validate_command("rm -rf /").is_err());
        assert!(validator.validate_command("DROP DATABASE production").is_err());
    }
}

// tests/unit/watsonx_test.rs
#[cfg(test)]
mod watsonx_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_analyze_logs() {
        let client = WatsonxClient::new().unwrap();
        let logs = "ERROR: Disk space critical on /var - 95% used";
        let result = client.analyze_logs(logs, "{}").await;
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert!(!analysis.root_cause.is_empty());
    }
    
    #[tokio::test]
    async fn test_suggest_remediation() {
        let client = WatsonxClient::new().unwrap();
        let root_cause = "Disk space exhausted due to log accumulation";
        let result = client.suggest_remediation(root_cause, "{}").await;
        assert!(result.is_ok());
        let suggestions = result.unwrap();
        assert!(!suggestions.is_empty());
    }
}
```

2. **Integration Tests** (tests/integration/)

```rust
// tests/integration/full_workflow_test.rs
#[tokio::test]
async fn test_disk_full_remediation() {
    // Setup
    let config = create_test_config();
    let engine = ReasoningEngine::new(config).await.unwrap();
    
    // Create test alert
    let alert = Alert {
        status: "firing".to_string(),
        labels: hashmap! {
            "alertname" => "DiskSpaceLow",
            "severity" => "warning",
            "instance" => "test-server",
            "filesystem" => "/var",
        },
        annotations: hashmap! {
            "summary" => "Disk space is critically low",
            "description" => "Filesystem /var is at 95% capacity",
        },
        starts_at: Utc::now().to_rfc3339(),
        ends_at: None,
        generator_url: "http://prometheus:9090".to_string(),
        fingerprint: "abc123".to_string(),
    };
    
    // Process alert
    let result = engine.process_alert(alert).await;
    
    // Verify
    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.execution_result.status, RemediationStatus::Success);
    assert!(report.verification.success);
}

#[tokio::test]
async fn test_service_crash_recovery() {
    let config = create_test_config();
    let engine = ReasoningEngine::new(config).await.unwrap();
    
    let alert = Alert {
        status: "firing".to_string(),
        labels: hashmap! {
            "alertname" => "ServiceDown",
            "severity" => "critical",
            "service" => "nginx",
        },
        annotations: hashmap! {
            "summary" => "Nginx service is down",
        },
        starts_at: Utc::now().to_rfc3339(),
        ends_at: None,
        generator_url: "http://prometheus:9090".to_string(),
        fingerprint: "def456".to_string(),
    };
    
    let result = engine.process_alert(alert).await;
    assert!(result.is_ok());
}
```

3. **Mock Tests** (tests/mocks/)

```rust
// tests/mocks/watsonx_mock.rs
pub struct MockWatsonxClient {
    responses: HashMap<String, Analysis>,
}

impl MockWatsonxClient {
    pub fn new() -> Self {
        let mut responses = HashMap::new();
        
        // Disk space scenario
        responses.insert(
            "disk".to_string(),
            Analysis {
                root_cause: "Disk space exhausted due to old log files in /var/log".to_string(),
                affected_components: vec!["/var filesystem".to_string()],
                impact: "Service degradation, potential outages".to_string(),
                urgency: "high".to_string(),
                lessons_learned: Some("Implement log rotation policy".to_string()),
            }
        );
        
        Self { responses }
    }
    
    pub async fn analyze_logs(&self, logs: &str, _context: &str) -> Result<Analysis> {
        if logs.contains("disk") || logs.contains("space") {
            Ok(self.responses.get("disk").unwrap().clone())
        } else {
            Ok(Analysis::default())
        }
    }
}
```

## Part B: Failure Scenarios (scripts/test-failure.sh)

Create scripts to inject failures for testing:

```bash
#!/bin/bash
# scripts/test-failure.sh

set -e

SCENARIO=$1

case $SCENARIO in
    "disk-full")
        echo "Injecting disk full scenario..."
        # Create large file to fill disk
        sudo fallocate -l 10G /tmp/test-large-file
        # Create old log files
        sudo mkdir -p /var/log/old-logs
        for i in {1..100}; do
            sudo dd if=/dev/zero of=/var/log/old-logs/app-$i.log bs=1M count=10
        done
        echo "Disk full scenario injected. Current usage:"
        df -h /var
        ;;
        
    "service-crash")
        SERVICE=$2
        echo "Crashing service: $SERVICE"
        sudo systemctl stop $SERVICE
        echo "Service $SERVICE stopped"
        sudo systemctl status $SERVICE || true
        ;;
        
    "pod-crashloop")
        echo "Creating crashloop pod..."
        kubectl apply -f - <<EOF
apiVersion: v1
kind: Pod
metadata:
  name: crashloop-test
  namespace: default
spec:
  containers:
  - name: app
    image: busybox
    command: ["sh", "-c", "exit 1"]
  restartPolicy: Always
EOF
        echo "Crashloop pod created. Watch with: kubectl get pod crashloop-test -w"
        ;;
        
    "memory-leak")
        echo "Simulating memory leak..."
        stress --vm 1 --vm-bytes 2G --timeout 300s &
        echo "Memory stress test started (PID: $!)"
        ;;
        
    "high-cpu")
        echo "Simulating high CPU usage..."
        stress --cpu 4 --timeout 300s &
        echo "CPU stress test started (PID: $!)"
        ;;
        
    *)
        echo "Unknown scenario: $SCENARIO"
        echo "Available scenarios:"
        echo "  disk-full       - Fill disk to 95%"
        echo "  service-crash   - Stop a systemd service"
        echo "  pod-crashloop   - Create crashlooping pod"
        echo "  memory-leak     - Simulate memory leak"
        echo "  high-cpu        - Simulate high CPU usage"
        exit 1
        ;;
esac
```

## Part C: Demo Script (scripts/demo.sh)

```bash
#!/bin/bash
# scripts/demo.sh - Automated demo of Sentinel-MCP

set -e

echo "========================================="
echo "Sentinel-MCP Demo"
echo "========================================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if Sentinel-MCP is running
if ! curl -s http://localhost:3000/api/v1/health > /dev/null; then
    echo -e "${RED}Error: Sentinel-MCP is not running${NC}"
    echo "Start it with: cargo run --release"
    exit 1
fi

echo -e "${GREEN}✓ Sentinel-MCP is running${NC}"
echo ""

# Demo scenario selection
echo "Select demo scenario:"
echo "1. Disk Space Crisis"
echo "2. Service Crash Recovery"
echo "3. Kubernetes Pod Failure"
read -p "Enter choice (1-3): " CHOICE

case $CHOICE in
    1)
        echo ""
        echo -e "${YELLOW}=== Demo: Disk Space Crisis ===${NC}"
        echo ""
        echo "Step 1: Showing current disk usage..."
        df -h /var
        echo ""
        
        echo "Step 2: Injecting failure (filling disk to 95%)..."
        ./scripts/test-failure.sh disk-full
        echo ""
        
        echo "Step 3: Triggering alert..."
        curl -X POST http://localhost:3000/api/v1/alerts \
            -H "Content-Type: application/json" \
            -d @examples/alerts/disk-space-low.json
        echo ""
        
        echo "Step 4: Watching Sentinel-MCP logs..."
        echo "Press Ctrl+C to stop watching"
        tail -f logs/sentinel-mcp.log
        ;;
        
    2)
        echo ""
        echo -e "${YELLOW}=== Demo: Service Crash Recovery ===${NC}"
        echo ""
        echo "Step 1: Checking nginx status..."
        sudo systemctl status nginx || true
        echo ""
        
        echo "Step 2: Crashing nginx service..."
        ./scripts/test-failure.sh service-crash nginx
        echo ""
        
        echo "Step 3: Triggering alert..."
        curl -X POST http://localhost:3000/api/v1/alerts \
            -H "Content-Type: application/json" \
            -d @examples/alerts/service-down.json
        echo ""
        
        echo "Step 4: Watching recovery..."
        sleep 5
        sudo systemctl status nginx
        ;;
        
    3)
        echo ""
        echo -e "${YELLOW}=== Demo: Kubernetes Pod Failure ===${NC}"
        echo ""
        echo "Step 1: Creating crashloop pod..."
        ./scripts/test-failure.sh pod-crashloop
        echo ""
        
        echo "Step 2: Waiting for CrashLoopBackOff..."
        sleep 10
        kubectl get pod crashloop-test
        echo ""
        
        echo "Step 3: Triggering alert..."
        curl -X POST http://localhost:3000/api/v1/alerts \
            -H "Content-Type: application/json" \
            -d @examples/alerts/pod-crashloop.json
        echo ""
        
        echo "Step 4: Watching remediation..."
        kubectl logs -f deployment/sentinel-mcp -n sentinel-system
        ;;
esac

echo ""
echo -e "${GREEN}Demo complete!${NC}"
echo ""
echo "View the remediation report:"
echo "  cat logs/remediations/REMEDIATION_LOG_*.md"
```

## Part D: Example Alert Payloads (examples/alerts/)

Create example alert files:

```json
// examples/alerts/disk-space-low.json
{
  "version": "4",
  "groupKey": "{}:{alertname=\"DiskSpaceLow\"}",
  "status": "firing",
  "receiver": "sentinel-mcp",
  "groupLabels": {
    "alertname": "DiskSpaceLow"
  },
  "commonLabels": {
    "alertname": "DiskSpaceLow",
    "severity": "warning",
    "instance": "server-01",
    "filesystem": "/var"
  },
  "commonAnnotations": {
    "summary": "Disk space is critically low",
    "description": "Filesystem /var is at 95% capacity on server-01"
  },
  "externalURL": "http://prometheus:9090",
  "alerts": [{
    "status": "firing",
    "labels": {
      "alertname": "DiskSpaceLow",
      "severity": "warning",
      "instance": "server-01",
      "filesystem": "/var"
    },
    "annotations": {
      "summary": "Disk space is critically low",
      "description": "Filesystem /var is at 95% capacity on server-01"
    },
    "startsAt": "2026-05-02T18:00:00Z",
    "endsAt": "0001-01-01T00:00:00Z",
    "generatorURL": "http://prometheus:9090/graph?g0.expr=...",
    "fingerprint": "abc123def456"
  }]
}
```

Create similar files for:
- service-down.json
- pod-crashloop.json
- memory-high.json
- cpu-high.json

## Part E: Performance Tests (tests/performance/)

```rust
// tests/performance/load_test.rs
#[tokio::test]
async fn test_concurrent_alerts() {
    let config = create_test_config();
    let receiver = AlertReceiver::new(config);
    
    // Send 100 concurrent alerts
    let mut handles = vec![];
    for i in 0..100 {
        let receiver = receiver.clone();
        let handle = tokio::spawn(async move {
            let alert = create_test_alert(i);
            receiver.process_alert(alert).await
        });
        handles.push(handle);
    }
    
    // Wait for all to complete
    let results = futures::future::join_all(handles).await;
    
    // Verify all succeeded
    for result in results {
        assert!(result.is_ok());
    }
}
```

Include comprehensive test coverage and clear demo instructions.
```

## Expected Output

Bob should create:
1. Complete test suite with unit, integration, and performance tests
2. Failure injection scripts
3. Automated demo script
4. Example alert payloads
5. Test documentation

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test integration

# Run with coverage
cargo tarpaulin --out Html
```

## Next Steps

After Bob completes this, the project is ready for final documentation and video demo preparation.