# Sentinel-MCP Video Demo Script (3 Minutes)

## Pre-Recording Checklist

- [ ] Sentinel-MCP server running
- [ ] Terminal with clear font and high contrast
- [ ] Screen recording software ready
- [ ] Test environment prepared
- [ ] Example alerts ready
- [ ] Background music (optional, subtle)

## Script Breakdown

### Opening (0:00 - 0:45) - The Hook

**Visual**: Split screen - Left: Terminal flooded with error messages, Right: Stressed DevOps engineer

**Narration**:
> "It's 3 AM. Your infrastructure is failing. Alerts are flooding in. Every second counts, but you're manually digging through logs, trying to find the root cause."

**Action**:
- Show Prometheus dashboard with multiple firing alerts
- Show terminal with scrolling error logs
- Show alert notification spam

**Narration continues**:
> "What if your infrastructure could heal itself? Meet Sentinel-MCP - the autonomous infrastructure repair agent powered by IBM Bob and watsonx.ai."

**Visual**: Sentinel-MCP logo animation

---

### The Action (0:45 - 2:15) - Live Demo

#### Scene 1: The Crisis (0:45 - 1:00)

**Visual**: Terminal showing healthy system

**Narration**:
> "Let's see it in action. Here's a production server running normally."

**Action**:
```bash
# Show healthy metrics
df -h /var
systemctl status nginx
```

**Narration**:
> "Now, disaster strikes - disk space fills up to 95%."

**Action**:
```bash
# Inject failure
./scripts/test-failure.sh disk-full
df -h /var  # Shows 95% usage
```

---

#### Scene 2: Detection (1:00 - 1:20)

**Visual**: Split screen - Left: Prometheus firing alert, Right: Sentinel-MCP logs

**Narration**:
> "Prometheus detects the issue and sends an alert to Sentinel-MCP."

**Action**:
- Show Prometheus alert firing
- Show AlertManager webhook being sent
- Show Sentinel-MCP receiving alert

**Terminal output**:
```
[INFO] Alert received: DiskSpaceLow on server-01
[INFO] Severity: WARNING
[INFO] Gathering system context...
```

---

#### Scene 3: AI Analysis (1:20 - 1:40)

**Visual**: Sentinel-MCP logs showing AI analysis

**Narration**:
> "Sentinel uses IBM watsonx.ai with Granite models to analyze thousands of log lines in seconds."

**Terminal output**:
```
[INFO] Reading system logs: /var/log/syslog (last 1000 lines)
[INFO] Analyzing with watsonx.ai...
[INFO] Root cause identified: Old log files consuming 8.5GB in /var/log/old-logs
[INFO] Affected components: /var filesystem, application logging
[INFO] Urgency: HIGH
```

**Visual**: Show watsonx.ai API call and response (briefly)

---

#### Scene 4: Intelligent Remediation (1:40 - 2:00)

**Visual**: Sentinel-MCP proposing fix

**Narration**:
> "Based on the analysis, Sentinel proposes a fix and asks for approval."

**Terminal output**:
```
=== REMEDIATION PLAN ===
Alert: DiskSpaceLow
Root Cause: Old log files consuming excessive space

Proposed Steps:
1. Archive old logs to /backup/logs [Risk: LOW]
   Command: tar -czf /backup/logs/archive-20260502.tar.gz /var/log/old-logs
   
2. Remove old log files [Risk: MEDIUM]
   Command: rm -rf /var/log/old-logs/*
   
3. Verify disk space recovered [Risk: LOW]
   Command: df -h /var

Approve? (yes/no): 
```

**Action**: Type "yes" and press Enter

---

#### Scene 5: Execution (2:00 - 2:15)

**Visual**: Commands executing with real-time output

**Narration**:
> "Sentinel executes the plan safely, with built-in rollback capabilities."

**Terminal output**:
```
[INFO] Executing step 1: Archive old logs
[INFO] ✓ Step 1 completed successfully
[INFO] Executing step 2: Remove old log files
[INFO] ✓ Step 2 completed successfully
[INFO] Executing step 3: Verify disk space
[INFO] ✓ Disk usage reduced from 95% to 42%
[INFO] Remediation successful!
```

**Visual**: Show disk usage before (95%) and after (42%)

---

### The Value (2:15 - 3:00) - Impact

#### Scene 6: Auto-Documentation (2:15 - 2:35)

**Visual**: Generated remediation report

**Narration**:
> "Every action is automatically documented with complete audit trails."

**Action**: Show generated REMEDIATION_LOG.md file

**Visual**: Scroll through report showing:
- Alert details
- Root cause analysis
- Steps executed
- Verification results
- Metrics (MTTR: 2 minutes vs 30 minutes manual)

---

#### Scene 7: The Results (2:35 - 2:50)

**Visual**: Metrics dashboard

**Narration**:
> "The results speak for themselves:"

**Visual**: Show metrics:
- **MTTR**: Reduced from 30 minutes to 2 minutes (93% improvement)
- **Manual Interventions**: Reduced by 70%
- **Documentation**: 100% automated
- **Success Rate**: 95% correct root cause identification

---

#### Scene 8: IBM Bob Integration (2:50 - 3:00)

**Visual**: IBM Bob interface showing conversation

**Narration**:
> "Built entirely using IBM Bob's AI-native development process, with watsonx.ai powering the intelligence."

**Visual**: Quick montage of:
- Bob prompts used
- Code generation
- Architecture design
- Testing

**Narration**:
> "Sentinel-MCP: Because your infrastructure shouldn't need a human to heal itself."

**Visual**: Sentinel-MCP logo with tagline

**Text overlay**:
```
Sentinel-MCP
Autonomous Infrastructure Repair

Built with IBM Bob & watsonx.ai
github.com/paulmmoore3416/Sentinel-MCP
```

---

## Recording Tips

### Technical Setup
1. **Resolution**: 1920x1080 minimum
2. **Frame Rate**: 30 fps minimum
3. **Terminal Settings**:
   - Font: Fira Code or JetBrains Mono, size 14-16
   - Theme: High contrast (e.g., Dracula, One Dark Pro)
   - Clear background, no distractions

### Narration Tips
1. Speak clearly and at moderate pace
2. Emphasize key metrics and benefits
3. Show enthusiasm but remain professional
4. Practice timing to hit 3-minute mark

### Visual Tips
1. Use smooth transitions between scenes
2. Highlight important terminal output
3. Use arrows or circles to draw attention
4. Keep text on screen long enough to read (3-5 seconds)

### Editing Checklist
- [ ] Remove dead air and long pauses
- [ ] Add subtle background music (low volume)
- [ ] Add text overlays for key points
- [ ] Add transitions between sections
- [ ] Verify audio levels are consistent
- [ ] Add captions/subtitles
- [ ] Export in high quality (1080p, H.264)

## Alternative Scenarios

If disk space demo doesn't work well, use these alternatives:

### Alternative 1: Service Crash
- Show nginx running
- Kill nginx service
- Sentinel detects and restarts
- Show service back online

### Alternative 2: Kubernetes Pod Failure
- Show healthy pod
- Create crashloop pod
- Sentinel analyzes logs
- Fixes configuration issue
- Pod becomes healthy

## B-Roll Footage Ideas

Record these for editing flexibility:
- Prometheus dashboard with various alerts
- Grafana metrics showing MTTR improvements
- Terminal with Sentinel-MCP logs
- Code snippets from the project
- Architecture diagrams
- IBM Bob interface

## Call to Action

End with:
> "Want to eliminate alert fatigue in your organization? Check out Sentinel-MCP on GitHub and see how AI-native development with IBM Bob and watsonx.ai can transform your infrastructure operations."

**Display**:
- GitHub URL: github.com/paulmmoore3416/Sentinel-MCP
- Documentation link
- Demo video link
- Contact information