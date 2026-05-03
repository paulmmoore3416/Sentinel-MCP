# Sentinel-MCP: Write Authority & Remediation Boundaries

## Question Analysis

**Core Constraint**: Detection is easy; **write authority** is the challenge. A remediation agent needs clean rollback boundaries when diagnosis is wrong.

**Key Question**: Are fixes limited to safe runbooks, or can Sentinel-MCP create new remediation steps?

---

## Current State: Sentinel-MCP's Remediation Model

### ✅ What We Have

1. **Security Validation** ([`src/mcp/security.rs`](../src/mcp/security.rs))
   - Command whitelisting/blacklisting
   - Risk classification (Low/Medium/High)
   - Approval workflows for risky operations

2. **AI-Generated Remediation Plans** ([`src/reasoning/mod.rs`](../src/reasoning/mod.rs))
   - watsonx.ai analyzes logs and system state
   - Generates remediation steps dynamically
   - Not limited to pre-defined runbooks

3. **Execution Modes**
   - Dry-run: Simulate without executing
   - Interactive: Step-by-step approval
   - Autonomous: Auto-execute low-risk operations

### ❌ Critical Gaps Identified

1. **No Rollback Implementation** (Line 366 in [`reasoning/mod.rs`](../src/reasoning/mod.rs:366))
   ```rust
   rollback_command: None, // TODO: Generate rollback commands
   ```

2. **Missing Circuit Breaker Logic**
   - Mentioned in README but not implemented
   - No failure rate tracking
   - No automatic degradation

3. **No State Snapshots**
   - Cannot capture pre-remediation state
   - No clean rollback boundary
   - Risk of cascading failures

4. **Limited Verification**
   - Basic success/failure checking only
   - No deep system state validation
   - Cannot detect partial failures

---

## The Write Authority Problem

### Current Approach: **Hybrid Model**

Sentinel-MCP uses a **hybrid approach** between safe runbooks and dynamic remediation:

```
┌─────────────────────────────────────────────────────────┐
│                  Remediation Spectrum                    │
├─────────────────────────────────────────────────────────┤
│                                                           │
│  Safe Runbooks ←──────────────────────→ Dynamic AI      │
│  (Pre-defined)                          (Generated)      │
│                                                           │
│  ✓ Predictable                          ✓ Adaptive       │
│  ✓ Tested                               ✓ Context-aware  │
│  ✓ Rollback-ready                       ✗ Unpredictable  │
│  ✗ Inflexible                           ✗ Risky          │
│                                                           │
│         Sentinel-MCP Current Position: ────┐             │
│                                             ▼             │
│                                        [HYBRID]           │
│                                                           │
└─────────────────────────────────────────────────────────┘
```

**Current Reality**: Sentinel-MCP **can create new remediation steps** via watsonx.ai, but lacks the safety boundaries to do so reliably.

---

## Required Fixes: Production-Ready Write Authority

### 1. **Implement Rollback Boundaries** 🔴 CRITICAL

**Problem**: Line 366 shows `rollback_command: None`

**Solution**: State snapshot + rollback generation

```rust
pub struct RemediationStep {
    pub step_number: usize,
    pub description: String,
    pub command: String,
    pub args: Vec<String>,
    pub risk_level: RiskLevel,
    pub expected_outcome: String,
    
    // NEW: Rollback support
    pub pre_execution_snapshot: Option<SystemSnapshot>,
    pub rollback_command: Option<String>,
    pub rollback_verified: bool,
}

pub struct SystemSnapshot {
    pub timestamp: String,
    pub filesystem_state: HashMap<String, FileMetadata>,
    pub service_states: Vec<ServiceStatus>,
    pub k8s_resources: Option<K8sSnapshot>,
    pub checksum: String,
}
```

**Implementation Priority**: HIGH - Required for production use

---

### 2. **Add Circuit Breakers** 🔴 CRITICAL

**Problem**: No failure tracking or automatic degradation

**Solution**: Per-alert-type circuit breaker

```rust
pub struct CircuitBreaker {
    pub alert_type: String,
    pub failure_count: u32,
    pub success_count: u32,
    pub state: CircuitState,
    pub last_failure: Option<DateTime<Utc>>,
    pub threshold: CircuitBreakerConfig,
}

pub enum CircuitState {
    Closed,      // Normal operation
    Open,        // Too many failures, block execution
    HalfOpen,    // Testing if system recovered
}

impl CircuitBreaker {
    pub fn should_allow_execution(&self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if cooldown period elapsed
                self.last_failure
                    .map(|t| Utc::now() - t > Duration::minutes(5))
                    .unwrap_or(false)
            }
            CircuitState::HalfOpen => true, // Allow one test
        }
    }
}
```

**Implementation Priority**: HIGH - Prevents cascading failures

---

### 3. **Implement Remediation Runbook Registry** 🟡 MEDIUM

**Problem**: No way to constrain AI to tested patterns

**Solution**: Hybrid approach with runbook templates

```rust
pub struct RunbookRegistry {
    pub runbooks: HashMap<String, Runbook>,
}

pub struct Runbook {
    pub id: String,
    pub alert_pattern: String,
    pub steps: Vec<RunbookStep>,
    pub tested: bool,
    pub success_rate: f64,
    pub rollback_verified: bool,
}

pub enum RemediationMode {
    RunbookOnly,        // Only use pre-defined runbooks
    AIAssisted,         // AI suggests from runbook templates
    AIGenerated,        // AI creates new steps (requires approval)
}
```

**Benefits**:
- Start with safe, tested runbooks
- Gradually expand to AI-assisted
- Full AI generation only for approved scenarios

**Implementation Priority**: MEDIUM - Improves safety

---

### 4. **Add Deep Verification** 🟡 MEDIUM

**Problem**: Current verification is superficial (line 469-486 in [`reasoning/mod.rs`](../src/reasoning/mod.rs:469-486))

**Solution**: Multi-level verification

```rust
pub struct VerificationResult {
    pub success: bool,
    pub details: String,
    pub metrics: HashMap<String, String>,
    
    // NEW: Deep verification
    pub system_state_valid: bool,
    pub alert_resolved: bool,
    pub no_side_effects: bool,
    pub rollback_tested: bool,
    pub confidence_score: f64,
}

impl ReasoningEngine {
    async fn verify_remediation_deep(
        &self,
        alert: &Alert,
        pre_snapshot: &SystemSnapshot,
        execution_result: &ExecutionResult,
    ) -> Result<VerificationResult> {
        // 1. Check alert is resolved
        let alert_resolved = self.check_alert_cleared(alert).await?;
        
        // 2. Verify no unexpected changes
        let post_snapshot = self.capture_snapshot().await?;
        let no_side_effects = self.compare_snapshots(
            pre_snapshot, 
            &post_snapshot
        )?;
        
        // 3. Test rollback (in staging)
        let rollback_tested = if self.config.verify_rollback {
            self.test_rollback_in_staging(alert).await?
        } else {
            false
        };
        
        // 4. Calculate confidence
        let confidence = self.calculate_confidence(
            alert_resolved,
            no_side_effects,
            execution_result.success,
        );
        
        Ok(VerificationResult {
            success: execution_result.success && alert_resolved,
            system_state_valid: no_side_effects,
            alert_resolved,
            no_side_effects,
            rollback_tested,
            confidence_score: confidence,
            details: format!("Verification complete: confidence {:.2}%", confidence * 100.0),
            metrics: HashMap::new(),
        })
    }
}
```

**Implementation Priority**: MEDIUM - Improves reliability

---

### 5. **Add Prometheus Metrics for Safety** 🟢 LOW

**Problem**: No observability into remediation safety

**Solution**: Safety-focused metrics

```rust
// Track remediation safety
sentinel_remediation_rollback_success_total
sentinel_remediation_rollback_failure_total
sentinel_circuit_breaker_trips_total
sentinel_ai_generated_steps_total
sentinel_runbook_steps_total
sentinel_verification_confidence_score
```

**Implementation Priority**: LOW - Nice to have

---

## Recommended Response to the Question

### **Short Answer**

Sentinel-MCP **can create new remediation steps** via watsonx.ai, but currently lacks production-ready rollback boundaries. The system needs:

1. ✅ State snapshots before execution
2. ✅ Automatic rollback command generation
3. ✅ Circuit breakers to prevent cascading failures
4. ✅ Deep verification of remediation success

### **Detailed Answer**

**Current Capability**: Sentinel-MCP uses a **hybrid model**:

- **AI-Generated Steps**: watsonx.ai analyzes logs and creates remediation plans dynamically
- **Security Constraints**: Commands are validated against risk levels
- **Approval Workflows**: High/medium risk operations require approval

**The Write Authority Constraint**:

The real constraint isn't whether we *can* generate new steps (we can), but whether we can do so **safely**. The question correctly identifies that rollback boundaries are critical.

**What's Missing**:

1. **Rollback Implementation** (Line 366: `rollback_command: None`)
   - No pre-execution state snapshots
   - No automatic rollback generation
   - No rollback verification

2. **Circuit Breakers**
   - No failure rate tracking per alert type
   - No automatic degradation when diagnosis is wrong
   - Risk of repeated failures

3. **Verification Depth**
   - Basic success/failure only
   - No deep system state validation
   - Cannot detect partial failures or side effects

**Recommended Architecture**:

```
┌─────────────────────────────────────────────────────────┐
│              Remediation Decision Tree                   │
├─────────────────────────────────────────────────────────┤
│                                                           │
│  Alert Received                                          │
│       │                                                   │
│       ├─→ Check Circuit Breaker                          │
│       │   └─→ OPEN? → Escalate to human                  │
│       │                                                   │
│       ├─→ Check Runbook Registry                         │
│       │   ├─→ Exact match? → Use tested runbook          │
│       │   └─→ No match? → AI generation                  │
│       │                                                   │
│       ├─→ Capture System Snapshot                        │
│       │   └─→ Filesystem, services, K8s state            │
│       │                                                   │
│       ├─→ Generate Remediation Plan                      │
│       │   ├─→ AI suggests steps                          │
│       │   └─→ Generate rollback for each step            │
│       │                                                   │
│       ├─→ Validate & Approve                             │
│       │   └─→ Risk assessment + human approval           │
│       │                                                   │
│       ├─→ Execute with Rollback Ready                    │
│       │   └─→ Each step has rollback command             │
│       │                                                   │
│       ├─→ Deep Verification                              │
│       │   ├─→ Alert resolved?                            │
│       │   ├─→ No side effects?                           │
│       │   └─→ Rollback tested?                           │
│       │                                                   │
│       └─→ Update Circuit Breaker                         │
│           ├─→ Success → Reset failure count              │
│           └─→ Failure → Increment, maybe trip            │
│                                                           │
└─────────────────────────────────────────────────────────┘
```

**Answer to "Are fixes limited to safe runbooks?"**

**No** - Sentinel-MCP can create new remediation steps via AI, but should implement a **graduated trust model**:

1. **Tier 1 - Tested Runbooks**: Pre-defined, rollback-verified, auto-execute
2. **Tier 2 - AI-Assisted**: AI selects from runbook templates, requires approval
3. **Tier 3 - AI-Generated**: Fully dynamic steps, requires approval + rollback verification

**Production Recommendation**:

Start with Tier 1 (runbooks only), gradually expand to Tier 2 as confidence builds, reserve Tier 3 for non-critical environments or with human oversight.

---

## Implementation Roadmap

### Phase 1: Safety Foundations (Week 1-2)
- [ ] Implement system snapshot capture
- [ ] Add rollback command generation
- [ ] Create rollback verification tests

### Phase 2: Circuit Breakers (Week 3)
- [ ] Implement per-alert-type circuit breakers
- [ ] Add failure rate tracking
- [ ] Create automatic degradation logic

### Phase 3: Runbook Registry (Week 4)
- [ ] Create runbook template system
- [ ] Implement graduated trust model
- [ ] Add runbook testing framework

### Phase 4: Deep Verification (Week 5)
- [ ] Implement multi-level verification
- [ ] Add confidence scoring
- [ ] Create side-effect detection

### Phase 5: Observability (Week 6)
- [ ] Add safety-focused Prometheus metrics
- [ ] Create Grafana dashboards for safety
- [ ] Implement alerting on circuit breaker trips

---

## Conclusion

**The constraint is real**: Write authority requires clean rollback boundaries.

**Sentinel-MCP's answer**: We **can** create new remediation steps, but we **must** implement proper safety boundaries first:

1. State snapshots + rollback generation
2. Circuit breakers for failure containment
3. Deep verification of remediation success
4. Graduated trust model (runbooks → AI-assisted → AI-generated)

**Current Status**: Prototype with AI generation capability, but **not production-ready** for autonomous write operations without the safety improvements outlined above.

**Recommendation**: Implement Phase 1-2 (rollback + circuit breakers) before deploying to production with write authority.

---

*Document created: 2026-05-03*  
*Author: Paul Moore*  
*Project: Sentinel-MCP*