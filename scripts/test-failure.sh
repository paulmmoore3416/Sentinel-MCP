#!/bin/bash
# scripts/test-failure.sh - Inject failures for testing Sentinel-MCP

set -e

SCENARIO=$1

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}Sentinel-MCP Failure Injection Tool${NC}"
echo ""

case $SCENARIO in
    "disk-full")
        echo -e "${YELLOW}Injecting disk full scenario...${NC}"
        echo ""
        
        # Check if running as root
        if [ "$EUID" -ne 0 ]; then
            echo -e "${RED}This scenario requires root privileges${NC}"
            echo "Run with: sudo $0 $SCENARIO"
            exit 1
        fi
        
        # Create test directory
        TEST_DIR="/tmp/sentinel-test"
        mkdir -p "$TEST_DIR"
        
        echo "Creating large test files to simulate disk usage..."
        # Create 1GB of test files
        for i in {1..10}; do
            dd if=/dev/zero of="$TEST_DIR/testfile-$i.dat" bs=100M count=1 2>/dev/null
            echo -n "."
        done
        echo ""
        
        echo -e "${GREEN}✓ Test files created${NC}"
        echo ""
        echo "Current disk usage:"
        df -h /tmp
        echo ""
        echo -e "${YELLOW}To clean up:${NC}"
        echo "  sudo rm -rf $TEST_DIR"
        ;;
        
    "service-crash")
        SERVICE=${2:-"nginx"}
        echo -e "${YELLOW}Simulating service crash: $SERVICE${NC}"
        echo ""
        
        # Check if service exists
        if ! systemctl list-unit-files | grep -q "^$SERVICE.service"; then
            echo -e "${RED}Service $SERVICE not found${NC}"
            echo "Available services:"
            systemctl list-unit-files --type=service | grep -E "enabled|disabled" | head -10
            exit 1
        fi
        
        echo "Stopping service: $SERVICE"
        sudo systemctl stop "$SERVICE" 2>/dev/null || true
        
        echo ""
        echo "Service status:"
        sudo systemctl status "$SERVICE" --no-pager || true
        echo ""
        echo -e "${GREEN}✓ Service stopped${NC}"
        echo ""
        echo -e "${YELLOW}To restore:${NC}"
        echo "  sudo systemctl start $SERVICE"
        ;;
        
    "pod-crashloop")
        echo -e "${YELLOW}Creating crashloop pod in Kubernetes...${NC}"
        echo ""
        
        # Check if kubectl is available
        if ! command -v kubectl &> /dev/null; then
            echo -e "${RED}kubectl not found${NC}"
            echo "Install kubectl to use this scenario"
            exit 1
        fi
        
        # Check if cluster is accessible
        if ! kubectl cluster-info &> /dev/null; then
            echo -e "${RED}Cannot connect to Kubernetes cluster${NC}"
            echo "Make sure your kubeconfig is set up correctly"
            exit 1
        fi
        
        echo "Creating crashloop test pod..."
        kubectl apply -f - <<EOF
apiVersion: v1
kind: Pod
metadata:
  name: crashloop-test
  namespace: default
  labels:
    app: sentinel-test
spec:
  containers:
  - name: app
    image: busybox
    command: ["sh", "-c", "echo 'Crashing...'; exit 1"]
  restartPolicy: Always
EOF
        
        echo ""
        echo -e "${GREEN}✓ Crashloop pod created${NC}"
        echo ""
        echo "Watch the pod status:"
        echo "  kubectl get pod crashloop-test -w"
        echo ""
        echo "View pod logs:"
        echo "  kubectl logs crashloop-test"
        echo ""
        echo -e "${YELLOW}To clean up:${NC}"
        echo "  kubectl delete pod crashloop-test"
        ;;
        
    "memory-leak")
        echo -e "${YELLOW}Simulating memory leak...${NC}"
        echo ""
        
        # Check if stress tool is available
        if ! command -v stress &> /dev/null; then
            echo -e "${RED}stress tool not found${NC}"
            echo "Install with: sudo apt-get install stress"
            exit 1
        fi
        
        echo "Starting memory stress test (2GB for 5 minutes)..."
        stress --vm 1 --vm-bytes 2G --timeout 300s &
        STRESS_PID=$!
        
        echo -e "${GREEN}✓ Memory stress started (PID: $STRESS_PID)${NC}"
        echo ""
        echo "Monitor memory usage:"
        echo "  watch -n 1 free -h"
        echo ""
        echo -e "${YELLOW}To stop:${NC}"
        echo "  kill $STRESS_PID"
        ;;
        
    "high-cpu")
        echo -e "${YELLOW}Simulating high CPU usage...${NC}"
        echo ""
        
        # Check if stress tool is available
        if ! command -v stress &> /dev/null; then
            echo -e "${RED}stress tool not found${NC}"
            echo "Install with: sudo apt-get install stress"
            exit 1
        fi
        
        CPU_COUNT=$(nproc)
        echo "Starting CPU stress test ($CPU_COUNT cores for 5 minutes)..."
        stress --cpu "$CPU_COUNT" --timeout 300s &
        STRESS_PID=$!
        
        echo -e "${GREEN}✓ CPU stress started (PID: $STRESS_PID)${NC}"
        echo ""
        echo "Monitor CPU usage:"
        echo "  top"
        echo "  htop"
        echo ""
        echo -e "${YELLOW}To stop:${NC}"
        echo "  kill $STRESS_PID"
        ;;
        
    "network-issue")
        echo -e "${YELLOW}Simulating network connectivity issue...${NC}"
        echo ""
        
        TARGET=${2:-"8.8.8.8"}
        
        echo "Blocking traffic to $TARGET..."
        sudo iptables -A OUTPUT -d "$TARGET" -j DROP
        
        echo -e "${GREEN}✓ Network block applied${NC}"
        echo ""
        echo "Test connectivity:"
        echo "  ping $TARGET"
        echo ""
        echo -e "${YELLOW}To restore:${NC}"
        echo "  sudo iptables -D OUTPUT -d $TARGET -j DROP"
        ;;
        
    "cleanup")
        echo -e "${YELLOW}Cleaning up all test scenarios...${NC}"
        echo ""
        
        # Clean disk test files
        if [ -d "/tmp/sentinel-test" ]; then
            echo "Removing test files..."
            sudo rm -rf /tmp/sentinel-test
            echo -e "${GREEN}✓ Test files removed${NC}"
        fi
        
        # Kill stress processes
        if pgrep stress > /dev/null; then
            echo "Stopping stress tests..."
            sudo pkill stress
            echo -e "${GREEN}✓ Stress tests stopped${NC}"
        fi
        
        # Clean up Kubernetes test pod
        if command -v kubectl &> /dev/null; then
            if kubectl get pod crashloop-test &> /dev/null; then
                echo "Removing test pod..."
                kubectl delete pod crashloop-test
                echo -e "${GREEN}✓ Test pod removed${NC}"
            fi
        fi
        
        echo ""
        echo -e "${GREEN}Cleanup complete!${NC}"
        ;;
        
    *)
        echo -e "${RED}Unknown scenario: $SCENARIO${NC}"
        echo ""
        echo "Available scenarios:"
        echo "  ${GREEN}disk-full${NC}       - Fill disk to simulate space issues"
        echo "  ${GREEN}service-crash${NC}   - Stop a systemd service (requires service name)"
        echo "  ${GREEN}pod-crashloop${NC}   - Create crashlooping Kubernetes pod"
        echo "  ${GREEN}memory-leak${NC}     - Simulate memory leak"
        echo "  ${GREEN}high-cpu${NC}        - Simulate high CPU usage"
        echo "  ${GREEN}network-issue${NC}   - Block network traffic (requires target IP)"
        echo "  ${GREEN}cleanup${NC}         - Clean up all test scenarios"
        echo ""
        echo "Examples:"
        echo "  $0 disk-full"
        echo "  $0 service-crash nginx"
        echo "  $0 pod-crashloop"
        echo "  $0 network-issue 8.8.8.8"
        echo "  $0 cleanup"
        exit 1
        ;;
esac

echo ""
echo -e "${YELLOW}Scenario injected successfully!${NC}"
echo "Now trigger an alert to Sentinel-MCP to see autonomous remediation in action."

# Made with Bob
