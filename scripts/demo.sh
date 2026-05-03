#!/bin/bash
# scripts/demo.sh - Automated demo of Sentinel-MCP

set -e

echo "========================================="
echo "Sentinel-MCP Demo"
echo "Autonomous Infrastructure Repair Agent"
echo "========================================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SENTINEL_URL="${SENTINEL_URL:-http://localhost:3000}"
EXAMPLES_DIR="$(dirname "$0")/../examples/alerts"

# Check if Sentinel-MCP is running
echo -n "Checking if Sentinel-MCP is running... "
if curl -s "${SENTINEL_URL}/api/v1/health" > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    echo ""
    echo -e "${RED}Error: Sentinel-MCP is not running${NC}"
    echo "Start it with: cargo run --release"
    echo "Or: RUST_LOG=info cargo run"
    exit 1
fi

# Get current status
echo -n "Getting system status... "
STATUS=$(curl -s "${SENTINEL_URL}/api/v1/status")
echo -e "${GREEN}✓${NC}"
echo "$STATUS" | jq '.' 2>/dev/null || echo "$STATUS"
echo ""

# Demo scenario selection
echo "Select demo scenario:"
echo "1. 💾 Disk Space Crisis"
echo "2. 🔧 Service Crash Recovery"
echo "3. ☸️  Kubernetes Pod Failure"
echo "4. 📊 View Recent Reports"
echo "5. 🧪 Send Custom Alert"
read -p "Enter choice (1-5): " CHOICE

case $CHOICE in
    1)
        echo ""
        echo -e "${YELLOW}=== Demo: Disk Space Crisis ===${NC}"
        echo ""
        echo "This demo simulates a disk space alert and shows how Sentinel-MCP"
        echo "automatically analyzes the issue and proposes remediation."
        echo ""
        
        echo "Step 1: Showing current disk usage..."
        df -h / 2>/dev/null || echo "Unable to show disk usage"
        echo ""
        
        echo "Step 2: Sending disk space alert to Sentinel-MCP..."
        if [ -f "${EXAMPLES_DIR}/disk-space-low.json" ]; then
            RESPONSE=$(curl -s -X POST "${SENTINEL_URL}/api/v1/alerts" \
                -H "Content-Type: application/json" \
                -d @"${EXAMPLES_DIR}/disk-space-low.json")
            echo -e "${GREEN}✓ Alert sent${NC}"
            echo "$RESPONSE" | jq '.' 2>/dev/null || echo "$RESPONSE"
        else
            echo -e "${RED}Error: Alert file not found${NC}"
            exit 1
        fi
        echo ""
        
        echo "Step 3: Sentinel-MCP is now:"
        echo "  - Gathering system context (logs, disk usage)"
        echo "  - Analyzing with IBM watsonx.ai"
        echo "  - Generating remediation plan"
        echo "  - Executing safe remediation steps"
        echo ""
        
        echo -e "${BLUE}Watch the logs in another terminal:${NC}"
        echo "  tail -f logs/sentinel-mcp.log"
        echo ""
        echo -e "${BLUE}View the remediation report:${NC}"
        echo "  ls -la logs/remediations/"
        echo "  cat logs/remediations/REMEDIATION_LOG.md"
        ;;
        
    2)
        echo ""
        echo -e "${YELLOW}=== Demo: Service Crash Recovery ===${NC}"
        echo ""
        echo "This demo simulates a service failure and shows automated recovery."
        echo ""
        
        echo "Step 1: Sending service down alert..."
        if [ -f "${EXAMPLES_DIR}/service-down.json" ]; then
            RESPONSE=$(curl -s -X POST "${SENTINEL_URL}/api/v1/alerts" \
                -H "Content-Type: application/json" \
                -d @"${EXAMPLES_DIR}/service-down.json")
            echo -e "${GREEN}✓ Alert sent${NC}"
            echo "$RESPONSE" | jq '.' 2>/dev/null || echo "$RESPONSE"
        else
            echo -e "${RED}Error: Alert file not found${NC}"
            exit 1
        fi
        echo ""
        
        echo "Step 2: Sentinel-MCP is now:"
        echo "  - Checking service status via systemd"
        echo "  - Analyzing service logs"
        echo "  - Determining root cause with AI"
        echo "  - Restarting service if safe"
        echo ""
        
        echo -e "${BLUE}Monitor the process:${NC}"
        echo "  watch -n 1 'curl -s ${SENTINEL_URL}/api/v1/status | jq'"
        ;;
        
    3)
        echo ""
        echo -e "${YELLOW}=== Demo: Kubernetes Pod Failure ===${NC}"
        echo ""
        echo "This demo simulates a Kubernetes pod crash loop."
        echo ""
        
        echo "Step 1: Sending pod crashloop alert..."
        if [ -f "${EXAMPLES_DIR}/pod-crashloop.json" ]; then
            RESPONSE=$(curl -s -X POST "${SENTINEL_URL}/api/v1/alerts" \
                -H "Content-Type: application/json" \
                -d @"${EXAMPLES_DIR}/pod-crashloop.json")
            echo -e "${GREEN}✓ Alert sent${NC}"
            echo "$RESPONSE" | jq '.' 2>/dev/null || echo "$RESPONSE"
        else
            echo -e "${RED}Error: Alert file not found${NC}"
            exit 1
        fi
        echo ""
        
        echo "Step 2: Sentinel-MCP is now:"
        echo "  - Querying Kubernetes API"
        echo "  - Analyzing pod logs"
        echo "  - Identifying crash cause with AI"
        echo "  - Proposing remediation strategy"
        echo ""
        
        echo -e "${BLUE}Check Kubernetes status:${NC}"
        echo "  kubectl get pods -A"
        echo "  kubectl describe pod <pod-name>"
        ;;
        
    4)
        echo ""
        echo -e "${YELLOW}=== Recent Remediation Reports ===${NC}"
        echo ""
        
        if [ -d "logs/remediations" ]; then
            echo "Available reports:"
            ls -lht logs/remediations/ | head -10
            echo ""
            
            LATEST=$(ls -t logs/remediations/REMEDIATION_LOG*.md 2>/dev/null | head -1)
            if [ -n "$LATEST" ]; then
                echo -e "${BLUE}Latest report:${NC}"
                echo "---"
                cat "$LATEST"
                echo "---"
            else
                echo "No reports found yet."
            fi
        else
            echo "No remediation reports directory found."
            echo "Reports will be created in: logs/remediations/"
        fi
        ;;
        
    5)
        echo ""
        echo -e "${YELLOW}=== Send Custom Alert ===${NC}"
        echo ""
        echo "Enter alert details:"
        read -p "Alert name: " ALERT_NAME
        read -p "Severity (warning/critical): " SEVERITY
        read -p "Description: " DESCRIPTION
        
        CUSTOM_ALERT=$(cat <<EOF
{
  "version": "4",
  "groupKey": "{}:{alertname=\"${ALERT_NAME}\"}",
  "status": "firing",
  "receiver": "sentinel-mcp",
  "groupLabels": {
    "alertname": "${ALERT_NAME}"
  },
  "commonLabels": {
    "alertname": "${ALERT_NAME}",
    "severity": "${SEVERITY}"
  },
  "commonAnnotations": {
    "summary": "${DESCRIPTION}"
  },
  "externalURL": "http://prometheus:9090",
  "alerts": [{
    "status": "firing",
    "labels": {
      "alertname": "${ALERT_NAME}",
      "severity": "${SEVERITY}"
    },
    "annotations": {
      "summary": "${DESCRIPTION}"
    },
    "startsAt": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "endsAt": "0001-01-01T00:00:00Z",
    "generatorURL": "http://prometheus:9090",
    "fingerprint": "$(uuidgen | tr -d '-' | head -c 12)"
  }]
}
EOF
)
        
        echo ""
        echo "Sending custom alert..."
        RESPONSE=$(curl -s -X POST "${SENTINEL_URL}/api/v1/alerts" \
            -H "Content-Type: application/json" \
            -d "$CUSTOM_ALERT")
        echo -e "${GREEN}✓ Alert sent${NC}"
        echo "$RESPONSE" | jq '.' 2>/dev/null || echo "$RESPONSE"
        ;;
        
    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}Demo complete!${NC}"
echo ""
echo -e "${BLUE}Useful commands:${NC}"
echo "  # View system status"
echo "  curl ${SENTINEL_URL}/api/v1/status | jq"
echo ""
echo "  # View health check"
echo "  curl ${SENTINEL_URL}/api/v1/health | jq"
echo ""
echo "  # View logs"
echo "  tail -f logs/sentinel-mcp.log"
echo ""
echo "  # View remediation reports"
echo "  ls -la logs/remediations/"
echo "  cat logs/remediations/REMEDIATION_LOG.md"

# Made with Bob
