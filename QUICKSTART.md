# Sentinel-MCP Quick Start Guide

Get Sentinel-MCP running in 5 minutes and see autonomous infrastructure remediation in action!

## Prerequisites

```bash
# Required
- Rust 1.70+ (install from https://rustup.rs)
- curl, jq

# Optional (for specific scenarios)
- Docker/Kubernetes (for K8s demos)
- systemd (for service demos)
```

## Step 1: Clone and Setup (2 minutes)

```bash
# Clone the repository
git clone https://github.com/paulmmoore3416/Sentinel-MCP.git
cd Sentinel-MCP

# Set up environment
cp .env.example .env

# Edit .env with your IBM watsonx.ai credentials
nano .env
```

**Required Environment Variables:**
```bash
WATSONX_API_KEY=your_api_key_here
WATSONX_PROJECT_ID=your_project_id_here
WATSONX_BASE_URL=https://us-south.ml.cloud.ibm.com/ml/v1
```

## Step 2: Build and Run (2 minutes)

```bash
# Build the project
cargo build --release

# Start Sentinel-MCP
RUST_LOG=info cargo run --release
```

You should see:
```
INFO sentinel_mcp: Starting Sentinel-MCP server...
INFO sentinel_mcp: Listening on 0.0.0.0:3000
```

## Step 3: Verify It's Running (30 seconds)

Open a new terminal:

```bash
# Health check
curl http://localhost:3000/api/v1/health | jq

# Expected output:
{
  "status": "healthy",
  "version": "0.1.0",
  "service": "sentinel-mcp"
}
```

## Step 4: Run Your First Demo (30 seconds)

```bash
# Run the interactive demo
./scripts/demo.sh

# Select option 1: Disk Space Crisis
# Watch as Sentinel-MCP:
# 1. Receives the alert
# 2. Analyzes logs with IBM watsonx.ai
# 3. Generates remediation plan
# 4. Executes safe cleanup
# 5. Creates documentation
```

## What Just Happened?

1. **Alert Received**: Sentinel-MCP received a Prometheus-style alert
2. **Context Gathering**: MCP tools collected system logs and disk usage
3. **AI Analysis**: IBM Granite model analyzed the root cause
4. **Plan Generation**: AI suggested remediation steps with risk levels
5. **Execution**: Safe commands executed (with approval for high-risk)
6. **Documentation**: Auto-generated report in `logs/remediations/`

## View the Results

```bash
# Check the status
curl http://localhost:3000/api/v1/status | jq

# View the remediation report
cat logs/remediations/REMEDIATION_LOG.md

# View HTML report (prettier!)
xdg-open logs/remediations/remediation_report.html
```

## Try More Scenarios

### Service Crash Recovery
```bash
./scripts/demo.sh
# Select option 2
```

### Kubernetes Pod Failure
```bash
./scripts/demo.sh
# Select option 3
```

### Custom Alert
```bash
./scripts/demo.sh
# Select option 5
# Enter your own alert details
```

## Manual Testing

Send a custom alert:

```bash
curl -X POST http://localhost:3000/api/v1/alerts \
  -H "Content-Type: application/json" \
  -d @examples/alerts/disk-space-low.json
```

## Architecture Overview

```
┌─────────────────┐
│  Prometheus     │
│  AlertManager   │
└────────┬────────┘
         │ Webhook
         ▼
┌─────────────────────────────────────────┐
│         Sentinel-MCP                    │
│                                         │
│  ┌──────────────┐    ┌──────────────┐ │
│  │ Alert        │───▶│ Reasoning    │ │
│  │ Receiver     │    │ Engine       │ │
│  └──────────────┘    └──────┬───────┘ │
│                              │         │
│  ┌──────────────┐    ┌──────▼───────┐ │
│  │ MCP Tools    │◀───│ watsonx.ai   │ │
│  │ (System)     │    │ (Analysis)   │ │
│  └──────────────┘    └──────────────┘ │
│         │                              │
│         ▼                              │
│  ┌──────────────┐    ┌──────────────┐ │
│  │ Executor     │───▶│ Documentation│ │
│  │ (Commands)   │    │ Generator    │ │
│  └──────────────┘    └──────────────┘ │
└─────────────────────────────────────────┘
         │
         ▼
┌─────────────────┐
│  Infrastructure │
│  (Linux/K8s)    │
└─────────────────┘
```

## Key Features Demonstrated

✅ **Agentic Reasoning**: AI thinks through problems, not just scripts
✅ **MCP Integration**: Direct system access via Model Context Protocol
✅ **watsonx.ai Powered**: IBM Granite models for intelligent analysis
✅ **Security-First**: Risk-based approval workflows
✅ **Auto-Documentation**: Complete audit trail in 3 formats
✅ **Production-Ready**: Error handling, rollback, verification

## Troubleshooting

### Port Already in Use
```bash
# Change port in .env
MCP_SERVER_PORT=3001
```

### watsonx.ai Connection Error
```bash
# Verify credentials
echo $WATSONX_API_KEY
echo $WATSONX_PROJECT_ID

# Test API access
curl -H "Authorization: Bearer $WATSONX_API_KEY" \
  https://us-south.ml.cloud.ibm.com/ml/v1/deployments
```

### Build Errors
```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

## Next Steps

- 📖 Read the [Full Documentation](README.md)
- 🧪 Try [Advanced Testing](docs/TESTING_GUIDE.md)
- 🎥 Record a [Video Demo](docs/VIDEO_DEMO_SCRIPT.md)
- 🏆 Submit to [Hackathon](docs/HACKATHON_SUBMISSION.md)

## Support

- **Documentation**: `docs/`
- **Examples**: `examples/alerts/`
- **Logs**: `logs/sentinel-mcp.log`
- **GitHub**: https://github.com/paulmmoore3416/Sentinel-MCP

---

**Built with IBM Bob and watsonx.ai** 🤖✨