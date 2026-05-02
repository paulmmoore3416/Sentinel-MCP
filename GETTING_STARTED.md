# Getting Started with Sentinel-MCP

Welcome to Sentinel-MCP! This guide will help you get started with the project, whether you're implementing it from scratch or contributing to development.

## 📋 Quick Overview

**Sentinel-MCP** is an autonomous infrastructure repair agent that uses:
- **IBM Bob** for AI-native development and agentic reasoning
- **IBM watsonx.ai** with Granite models for intelligent log analysis
- **MCP (Model Context Protocol)** to interact with live systems

## 🎯 What You'll Build

By following this guide, you'll create a system that:
1. Receives alerts from Prometheus/AlertManager
2. Analyzes logs and system state using AI
3. Proposes and executes remediation steps
4. Documents everything automatically

## 📚 Documentation Structure

### Core Documentation
- **[README.md](README.md)** - Project overview and usage examples
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design and components
- **[IMPLEMENTATION_GUIDE.md](docs/IMPLEMENTATION_GUIDE.md)** - Step-by-step build instructions

### IBM Bob Prompts
All prompts are in the `/prompts` directory, ready to use with IBM Bob:
1. **[01-scaffold.md](prompts/01-scaffold.md)** - Project scaffolding
2. **[02-mcp-tools.md](prompts/02-mcp-tools.md)** - MCP tools implementation
3. **[03-watsonx.md](prompts/03-watsonx.md)** - watsonx.ai integration
4. **[04-reasoning-engine.md](prompts/04-reasoning-engine.md)** - Reasoning engine
5. **[05-alert-and-docs.md](prompts/05-alert-and-docs.md)** - Alert receiver & docs
6. **[06-testing-and-demo.md](prompts/06-testing-and-demo.md)** - Testing & demo

### Demo Materials
- **[VIDEO_DEMO_SCRIPT.md](docs/VIDEO_DEMO_SCRIPT.md)** - 3-minute demo script
- **[HACKATHON_SUBMISSION.md](docs/HACKATHON_SUBMISSION.md)** - Submission checklist

## 🚀 Quick Start Options

### Option 1: Build from Scratch with IBM Bob

**Best for**: Learning the full development process

1. **Prerequisites**:
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Verify installation
   rustc --version
   cargo --version
   ```

2. **Set up IBM Bob**:
   - Install IBM Bob VS Code extension
   - Or use Bob CLI if available

3. **Follow the Implementation Guide**:
   - Open [docs/IMPLEMENTATION_GUIDE.md](docs/IMPLEMENTATION_GUIDE.md)
   - Start with Phase 1: Project Foundation
   - Use the prompts in `/prompts` directory with Bob
   - Build incrementally, testing after each phase

4. **Configure Environment**:
   ```bash
   cp .env.example .env
   # Edit .env with your IBM watsonx.ai credentials
   ```

### Option 2: Use Pre-built Components

**Best for**: Quick deployment and testing

1. **Clone and configure**:
   ```bash
   git clone https://github.com/paulmmoore3416/Sentinel-MCP.git
   cd Sentinel-MCP
   cp .env.example .env
   # Edit .env with your credentials
   ```

2. **Build and run**:
   ```bash
   cargo build --release
   cargo run --release
   ```

3. **Test with example alert**:
   ```bash
   curl -X POST http://localhost:3000/api/v1/alerts \
     -H "Content-Type: application/json" \
     -d @examples/alerts/disk-space-low.json
   ```

### Option 3: Deploy to Kubernetes

**Best for**: Production-like environment

1. **Prerequisites**:
   - Kubernetes cluster access
   - kubectl configured
   - Docker installed

2. **Build container**:
   ```bash
   docker build -t sentinel-mcp:latest .
   ```

3. **Deploy**:
   ```bash
   kubectl create namespace sentinel-system
   kubectl create secret generic watsonx-credentials \
     --from-literal=api-key=$WATSONX_API_KEY \
     --from-literal=project-id=$WATSONX_PROJECT_ID \
     -n sentinel-system
   kubectl apply -f k8s/
   ```

4. **Verify**:
   ```bash
   kubectl get pods -n sentinel-system
   kubectl logs -f deployment/sentinel-mcp -n sentinel-system
   ```

## 🎓 Learning Path

### For Beginners

1. **Start with the README**: Understand what Sentinel-MCP does
2. **Review the Architecture**: Learn how components work together
3. **Try the Demo**: Run a simple failure scenario
4. **Read the Prompts**: See how Bob was used to build it

### For Developers

1. **Read Implementation Guide**: Understand the build process
2. **Study the Prompts**: Learn effective Bob usage patterns
3. **Build Phase by Phase**: Follow the 4-week timeline
4. **Contribute**: Add features or improvements

### For DevOps/SREs

1. **Deploy to Test Environment**: Get hands-on experience
2. **Configure for Your Stack**: Adapt to your infrastructure
3. **Create Custom Scenarios**: Test with your failure modes
4. **Integrate with Monitoring**: Connect to your Prometheus

## 🔧 Configuration

### Essential Environment Variables

```bash
# Required
WATSONX_API_KEY=your_key          # IBM watsonx.ai API key
WATSONX_PROJECT_ID=your_id        # IBM watsonx.ai project ID

# Optional but recommended
MCP_AUTH_TOKEN=secure_token       # Secure your API endpoint
APPROVAL_REQUIRED=true            # Require approval for actions
DRY_RUN_MODE=false               # Set true for testing
```

### Security Settings

Edit `config/security-rules.yaml` to customize:
- Command whitelists/blacklists
- Risk level classifications
- Approval requirements
- Allowed Kubernetes namespaces

## 🧪 Testing

### Run Tests
```bash
# All tests
cargo test

# Specific test
cargo test test_watsonx_connection

# With output
cargo test -- --nocapture

# With coverage
cargo tarpaulin --out Html
```

### Demo Scenarios
```bash
# Make scripts executable
chmod +x scripts/*.sh

# Run interactive demo
./scripts/demo.sh

# Inject specific failure
./scripts/test-failure.sh disk-full
```

## 📊 Monitoring

### Health Check
```bash
curl http://localhost:3000/api/v1/health
```

### Status
```bash
curl http://localhost:3000/api/v1/status
```

### Logs
```bash
# Application logs
tail -f logs/sentinel-mcp.log

# Remediation reports
ls -la logs/remediations/
cat logs/remediations/REMEDIATION_LOG_*.md
```

## 🎥 Creating Your Demo Video

Follow the [VIDEO_DEMO_SCRIPT.md](docs/VIDEO_DEMO_SCRIPT.md) for:
- 3-minute demo structure
- Recording tips
- Editing checklist
- B-roll suggestions

## 🏆 Hackathon Submission

Use [HACKATHON_SUBMISSION.md](docs/HACKATHON_SUBMISSION.md) as your checklist:
- ✅ Problem statement
- ✅ IBM Bob usage documentation
- ✅ watsonx.ai integration
- ✅ Implementation plan
- ✅ Video demo
- ✅ Code repository

## 🤝 Getting Help

### Documentation
- Check the `/docs` directory for detailed guides
- Review example files in `/examples`
- Read the prompts in `/prompts` for context

### Common Issues

**Issue**: watsonx.ai authentication fails
```bash
# Verify credentials
echo $WATSONX_API_KEY
# Test API directly
curl -X POST "${WATSONX_URL}/ml/v1/text/generation?version=2023-05-29" \
  -H "Authorization: Bearer ${WATSONX_API_KEY}"
```

**Issue**: MCP tools not working
```bash
# Check permissions
ls -la /var/log/
# Test tool directly
cargo test test_read_system_logs -- --nocapture
```

**Issue**: Kubernetes deployment fails
```bash
# Check RBAC
kubectl get serviceaccount sentinel-mcp -n sentinel-system
# Check secrets
kubectl get secret watsonx-credentials -n sentinel-system
```

### Support Channels
- GitHub Issues: Bug reports and feature requests
- GitHub Discussions: Questions and community support
- Documentation: Comprehensive guides in `/docs`

## 📈 Next Steps

### After Initial Setup
1. ✅ Verify all components work
2. ✅ Run test scenarios
3. ✅ Review generated documentation
4. ✅ Customize for your environment

### For Development
1. ✅ Read CONTRIBUTING.md
2. ✅ Set up development environment
3. ✅ Pick an issue or feature
4. ✅ Submit a pull request

### For Production
1. ✅ Security audit
2. ✅ Performance testing
3. ✅ Monitoring setup
4. ✅ Backup and recovery plan

## 🎯 Success Metrics

Track these to measure impact:
- **MTTR**: Mean Time to Recovery (target: <5 min)
- **Automation Rate**: % of alerts handled automatically (target: >70%)
- **Success Rate**: % of successful remediations (target: >95%)
- **Documentation**: % of incidents documented (target: 100%)

## 📝 Project Timeline

| Week | Phase | Focus |
|------|-------|-------|
| 1 | Foundation | Project setup, MCP tools |
| 2 | Intelligence | watsonx.ai, reasoning engine |
| 3 | Integration | Alerts, documentation |
| 4 | Polish | Testing, demo, deployment |

## 🌟 Key Features to Highlight

When demonstrating Sentinel-MCP:
1. **Autonomous Reasoning**: AI that thinks, not just pattern matches
2. **Safety First**: Built-in security and approval workflows
3. **Complete Audit Trail**: Every action documented automatically
4. **Production Ready**: Enterprise-grade error handling
5. **AI-Native Development**: Built entirely with IBM Bob

## 📞 Contact

- **Author**: Paul Moore
- **GitHub**: [@paulmmoore3416](https://github.com/paulmmoore3416)
- **Repository**: [Sentinel-MCP](https://github.com/paulmmoore3416/Sentinel-MCP)

---

**Ready to get started?** Pick your path above and dive in! 🚀

**Built with ❤️ using IBM Bob and watsonx.ai**