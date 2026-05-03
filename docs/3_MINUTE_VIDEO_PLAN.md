# 🎥 Sentinel-MCP: 3-Minute Video Demo Plan

## 🎯 Video Objective
Demonstrate how Sentinel-MCP autonomously detects, analyzes, and fixes infrastructure issues using IBM Bob and watsonx.ai.

---

## ⏱️ Timeline Breakdown

### **0:00 - 0:30 | The Hook & Problem** (30 seconds)

**Visual:** Split screen showing:
- Left: Terminal flooded with error messages
- Right: Stressed DevOps engineer at 3 AM

**Script:**
> "It's 3 AM. Your infrastructure is failing. Alerts are flooding in. Your DevOps team is overwhelmed by alert fatigue, manually correlating logs, identifying root causes, and applying fixes. This leads to higher MTTR, operational burnout, and inconsistent remediation."

**On-Screen Text:**
- ⏱️ MTTR: 30+ minutes
- 😓 Manual intervention required
- 📉 Inconsistent practices

---

### **0:30 - 1:00 | The Solution** (30 seconds)

**Visual:** Show Sentinel-MCP logo and architecture diagram

**Script:**
> "Meet Sentinel-MCP: The Autonomous Infrastructure Repair Agent. Built with IBM Bob and watsonx.ai, it bridges monitoring alerts with autonomous remediation using the Model Context Protocol. Unlike static scripts, Sentinel uses AI to think through problems, adapting to unforeseen issues."

**On-Screen Text:**
- 🤖 AI-Powered Reasoning
- 🔒 Security-First Design
- 📝 Auto-Documentation
- ⚡ 2-Minute MTTR

**Show:** Quick pan through GitHub repository with logo and badges

---

### **1:00 - 2:15 | Live Demo** (75 seconds)

#### **Setup (10 seconds)**
**Visual:** Terminal with Sentinel-MCP running

**Script:**
> "Let's see it in action. Sentinel-MCP is running, monitoring our infrastructure."

**Commands to show:**
```bash
curl http://localhost:8484/api/v1/health | jq
```

---

#### **Trigger Failure (15 seconds)**
**Visual:** Split screen - Left: Terminal, Right: Disk usage

**Script:**
> "We'll simulate a disk space crisis - a common infrastructure issue."

**Commands to show:**
```bash
./scripts/test-failure.sh disk-full
df -h /var
```

**Show:** Disk usage at 95%

---

#### **AI Detection & Analysis (20 seconds)**
**Visual:** Split screen - Left: Sentinel logs, Right: watsonx.ai analysis

**Script:**
> "Sentinel immediately detects the alert, gathers system context using MCP tools, and sends logs to IBM watsonx.ai. The Granite model analyzes the root cause: old log files filling the disk."

**Show in logs:**
```
INFO: Alert received: DiskSpaceLow
INFO: Gathering context via MCP tools
INFO: Analyzing with watsonx.ai
INFO: Root cause identified: Log files at 95% capacity
```

---

#### **Remediation Plan (15 seconds)**
**Visual:** Show proposed remediation steps

**Script:**
> "Sentinel generates a remediation plan with risk-based security validation. Low-risk actions auto-execute, high-risk require approval."

**Show:**
```
Remediation Plan:
1. Rotate old logs [Risk: Low] ✓
2. Clean temp files [Risk: Low] ✓
3. Verify disk space [Risk: Low] ✓
```

---

#### **Execution & Verification (15 seconds)**
**Visual:** Show command execution and disk space recovery

**Script:**
> "Sentinel executes the remediation safely, verifies success, and disk space drops to 45%."

**Show:**
```bash
df -h /var
# Before: 95% used
# After: 45% used
```

---

### **2:15 - 2:45 | The Value** (30 seconds)

**Visual:** Show auto-generated documentation

**Script:**
> "But here's the game-changer: Sentinel automatically generates comprehensive documentation in three formats - Markdown, JSON, and HTML. Complete audit trail, zero manual work."

**Show:** Quick scroll through `REMEDIATION_LOG.md`

**On-Screen Comparison:**
```
Traditional Approach:
⏱️ MTTR: 30 minutes
👤 Manual intervention
📝 Manual documentation
⚠️ Human error risk

Sentinel-MCP:
⚡ MTTR: 2 minutes
🤖 Zero intervention
📄 Auto-documentation
✅ Consistent results
```

---

### **2:45 - 3:00 | IBM Technology & Call to Action** (15 seconds)

**Visual:** Show IBM Bob conversation export and watsonx.ai integration

**Script:**
> "Built entirely using IBM Bob for AI-native development and powered by IBM Granite models via watsonx.ai. This is the future of autonomous operations."

**On-Screen Text:**
- 🤖 Built with IBM Bob
- 💎 Powered by IBM Granite
- 🔌 Model Context Protocol
- 🌟 Open Source on GitHub

**Final Screen:**
```
Sentinel-MCP
github.com/paulmmoore3416/Sentinel-MCP

Built with ❤️ using IBM Bob and watsonx.ai
```

---

## 🎬 Recording Setup

### **Required Tools**
- Screen recording software (OBS Studio recommended)
- Terminal with good contrast
- Split-screen capability (tmux)
- Browser for showing documentation

### **Pre-Recording Checklist**
- [ ] Clean terminal history
- [ ] Increase terminal font size (16-18pt)
- [ ] Set up split-screen layout
- [ ] Have all commands ready in a script
- [ ] Test audio levels
- [ ] Close unnecessary applications
- [ ] Prepare GitHub repository view

### **Terminal Layout**
```
┌─────────────────────────────────────┐
│  Terminal 1: Sentinel Logs          │
│  (tail -f logs/sentinel-mcp.log)    │
├─────────────────────────────────────┤
│  Terminal 2: Commands & Status      │
│  (demo commands here)                │
└─────────────────────────────────────┘
```

---

## 📝 Exact Commands to Run (In Order)

### **Setup Phase**
```bash
# 1. Start server (already running)
curl http://localhost:8484/api/v1/health | jq

# 2. Show initial status
curl http://localhost:8484/api/v1/status | jq
```

### **Demo Phase**
```bash
# 3. Check disk before
df -h /var

# 4. Inject failure
./scripts/test-failure.sh disk-full

# 5. Check disk after injection
df -h /var

# 6. Send alert
curl -X POST http://localhost:8484/api/v1/alerts \
  -H "Content-Type: application/json" \
  -d @examples/alerts/disk-space-low.json

# 7. Watch logs (already running in split screen)
# tail -f logs/sentinel-mcp.log

# 8. Wait 10-15 seconds for processing

# 9. Check disk after remediation
df -h /var

# 10. Show generated report
cat logs/remediations/REMEDIATION_LOG.md | head -50
```

### **Wrap-up Phase**
```bash
# 11. Show final status
curl http://localhost:8484/api/v1/status | jq

# 12. Open GitHub in browser
# Show README with logo and badges
```

---

## 🎨 Visual Elements to Include

### **Graphics/Overlays**
1. **Title Card** (0:00)
   - Sentinel-MCP logo
   - "The Autonomous Infrastructure Repair Agent"

2. **Problem Stats** (0:15)
   - MTTR: 30+ minutes
   - Manual intervention required
   - Alert fatigue

3. **Architecture Diagram** (0:40)
   - Show Mermaid diagram from README

4. **Before/After Comparison** (2:20)
   - Traditional vs Sentinel-MCP

5. **Technology Stack** (2:50)
   - IBM Bob logo
   - IBM watsonx.ai logo
   - MCP logo

### **Text Overlays**
- Key metrics (MTTR, success rate)
- Command explanations
- Status indicators (✓, ⚡, 🔍)
- GitHub URL

---

## 🎤 Narration Tips

### **Tone**
- Professional but enthusiastic
- Clear and confident
- Emphasize key benefits
- Speak at moderate pace

### **Key Phrases to Emphasize**
- "Autonomous remediation"
- "AI-powered reasoning"
- "IBM watsonx.ai and Granite models"
- "Zero manual intervention"
- "Complete audit trail"
- "Built with IBM Bob"

### **Avoid**
- Technical jargon without explanation
- Speaking too fast
- Long pauses
- Filler words (um, uh, like)

---

## 📊 Success Metrics to Highlight

### **Performance**
- ⏱️ MTTR: 30 min → 2 min (93% reduction)
- 🤖 100% automated
- ✅ Consistent results

### **Features**
- 🧠 AI-powered analysis
- 🔒 Security validation
- 📝 Auto-documentation
- 🔄 Rollback support

### **Technology**
- 💎 IBM Granite models
- 🤖 IBM Bob development
- 🔌 MCP integration
- 🦀 Rust implementation

---

## 🎯 Call to Action

**End Screen Text:**
```
Try Sentinel-MCP Today!

⭐ Star on GitHub
📖 Read the Docs
🤝 Contribute
💬 Join the Discussion

github.com/paulmmoore3416/Sentinel-MCP

Built with IBM Bob and watsonx.ai
```

---

## 📋 Post-Recording Checklist

- [ ] Review for audio quality
- [ ] Check all text is readable
- [ ] Verify timing (exactly 3 minutes)
- [ ] Add title cards and overlays
- [ ] Include IBM branding
- [ ] Add background music (optional, low volume)
- [ ] Export in 1080p
- [ ] Test playback
- [ ] Upload to platform
- [ ] Add to README

---

## 🎬 Alternative: Quick Recording Script

If you want to record quickly, use this automated approach:

```bash
# Create recording script
cat > record_demo.sh << 'EOF'
#!/bin/bash
echo "=== Sentinel-MCP Demo ==="
sleep 2
echo "Checking server health..."
curl -s http://localhost:8484/api/v1/health | jq
sleep 3
echo "Injecting disk space failure..."
./scripts/test-failure.sh disk-full
sleep 3
echo "Sending alert..."
curl -X POST http://localhost:8484/api/v1/alerts \
  -H "Content-Type: application/json" \
  -d @examples/alerts/disk-space-low.json
sleep 15
echo "Checking remediation..."
df -h /var
sleep 2
echo "Viewing report..."
cat logs/remediations/REMEDIATION_LOG.md | head -30
EOF

chmod +x record_demo.sh
```

---

## 💡 Pro Tips

1. **Practice First:** Run through the demo 2-3 times before recording
2. **Use Markers:** Add visual markers for editing points
3. **Record in Segments:** Record in 30-second segments, easier to edit
4. **Have Backup:** Record multiple takes
5. **Check Audio:** Use a good microphone, minimize background noise
6. **Lighting:** Ensure screen is clearly visible
7. **Pace Yourself:** Speak clearly, don't rush
8. **Show Enthusiasm:** Your excitement is contagious!

---

**Ready to record your winning demo!** 🎥🏆