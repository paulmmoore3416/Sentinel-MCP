# Prompt 4: Reasoning Engine and Workflow Orchestration

## Context
The reasoning engine is the brain of Sentinel-MCP. It orchestrates the entire remediation workflow from alert receipt to documentation.

## Prompt for Bob (Plan Mode first, then Code Mode)

### Part A: Design the Workflow (Plan Mode)

```
Bob, let's design the reasoning engine workflow for Sentinel-MCP. I need you to create a state machine that handles the complete remediation lifecycle:

States:
1. ALERT_RECEIVED - Initial state when alert arrives
2. GATHERING_CONTEXT - Collecting logs and system state via MCP tools
3. ANALYZING - Sending data to watsonx.ai for analysis
4. PROPOSING_FIX - Generating remediation plan
5. AWAITING_APPROVAL - Waiting for user confirmation (if required)
6. EXECUTING - Running remediation commands
7. VERIFYING - Checking if remediation was successful
8. DOCUMENTING - Generating remediation report
9. COMPLETED - Final state
10. FAILED - Error state with rollback

Transitions:
- Each state can transition to the next or to FAILED
- AWAITING_APPROVAL can timeout and transition to FAILED
- EXECUTING can trigger rollback on failure
- FAILED state should attempt recovery or escalation

For each state, define:
1. Entry actions (what happens when entering this state)
2. Exit conditions (when to move to next state)
3. Error handling (what to do if something fails)
4. Timeout behavior (if applicable)

Also design the security validation logic:
- Low risk: Auto-approve (e.g., service restart, log rotation)
- Medium risk: Require confirmation (e.g., pod deletion, config changes)
- High risk: Require manual review (e.g., data deletion, namespace operations)

Create a detailed workflow diagram and decision tree for the reasoning engine.
```

### Part B: Implement the Engine (Code Mode)

```
Bob, now implement the reasoning engine in src/reasoning/mod.rs based on the design we created:

1. **ReasoningEngine struct**
   ```rust
   pub struct ReasoningEngine {
       mcp_client: Arc<McpClient>,
       watsonx_client: Arc<WatsonxClient>,
       executor: Arc<RemediationExecutor>,
       state: Arc<RwLock<WorkflowState>>,
       config: ReasoningConfig,
   }
   ```

2. **WorkflowState enum**
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum WorkflowState {
       AlertReceived { alert: Alert, timestamp: DateTime<Utc> },
       GatheringContext { alert: Alert, progress: u8 },
       Analyzing { alert: Alert, context: SystemContext },
       ProposingFix { alert: Alert, analysis: Analysis },
       AwaitingApproval { alert: Alert, plan: RemediationPlan, timeout: DateTime<Utc> },
       Executing { alert: Alert, plan: RemediationPlan, step: usize },
       Verifying { alert: Alert, execution_result: ExecutionResult },
       Documenting { alert: Alert, full_context: RemediationContext },
       Completed { alert: Alert, report: RemediationReport },
       Failed { alert: Alert, error: String, rollback_attempted: bool },
   }
   ```

3. **Core Methods**
   
   **process_alert()**
   ```rust
   pub async fn process_alert(&self, alert: Alert) -> Result<RemediationReport> {
       // Main entry point - orchestrates entire workflow
       self.transition_to(WorkflowState::AlertReceived { alert: alert.clone(), timestamp: Utc::now() }).await?;
       
       // Gather context
       let context = self.gather_context(&alert).await?;
       
       // Analyze with watsonx
       let analysis = self.analyze_with_ai(&alert, &context).await?;
       
       // Generate remediation plan
       let plan = self.generate_plan(&alert, &analysis).await?;
       
       // Check if approval needed
       if self.requires_approval(&plan) {
           self.request_approval(&plan).await?;
       }
       
       // Execute remediation
       let result = self.execute_plan(&plan).await?;
       
       // Verify success
       self.verify_remediation(&alert, &result).await?;
       
       // Generate documentation
       let report = self.generate_report(&alert, &analysis, &plan, &result).await?;
       
       Ok(report)
   }
   ```
   
   **gather_context()**
   ```rust
   async fn gather_context(&self, alert: &Alert) -> Result<SystemContext> {
       // Use MCP tools to gather relevant information
       let mut context = SystemContext::new();
       
       // Get logs based on alert type
       if alert.labels.contains_key("filesystem") {
           context.disk_usage = self.mcp_client.get_disk_usage(&alert.labels["filesystem"]).await?;
           context.logs = self.mcp_client.read_system_logs("syslog", 100, Some("disk")).await?;
       }
       
       if alert.labels.contains_key("service") {
           context.service_status = self.mcp_client.list_systemd_services(Some(&alert.labels["service"])).await?;
           context.logs = self.mcp_client.read_system_logs("journalctl", 100, Some(&alert.labels["service"])).await?;
       }
       
       if alert.labels.contains_key("namespace") {
           context.k8s_status = self.mcp_client.check_kubernetes_status(
               "pod",
               &alert.labels["namespace"],
               alert.labels.get("pod")
           ).await?;
       }
       
       Ok(context)
   }
   ```
   
   **analyze_with_ai()**
   ```rust
   async fn analyze_with_ai(&self, alert: &Alert, context: &SystemContext) -> Result<Analysis> {
       // Send to watsonx.ai for intelligent analysis
       let analysis = self.watsonx_client.analyze_logs(
           &context.logs,
           &serde_json::to_string(context)?
       ).await?;
       
       // Enrich with additional context
       let mut enriched = analysis;
       enriched.alert_name = alert.labels.get("alertname").cloned();
       enriched.severity = alert.labels.get("severity").cloned();
       
       Ok(enriched)
   }
   ```
   
   **generate_plan()**
   ```rust
   async fn generate_plan(&self, alert: &Alert, analysis: &Analysis) -> Result<RemediationPlan> {
       // Get remediation suggestions from watsonx
       let suggestions = self.watsonx_client.suggest_remediation(
           &analysis.root_cause,
           &serde_json::to_string(&analysis)?
       ).await?;
       
       // Convert to executable plan with security validation
       let mut plan = RemediationPlan::new(alert.clone(), analysis.clone());
       
       for suggestion in suggestions {
           let step = RemediationStep {
               description: suggestion.description,
               command: suggestion.command,
               risk_level: self.classify_risk(&suggestion.command),
               expected_outcome: suggestion.expected_outcome,
               rollback_command: self.generate_rollback(&suggestion.command),
           };
           plan.add_step(step);
       }
       
       Ok(plan)
   }
   ```
   
   **requires_approval()**
   ```rust
   fn requires_approval(&self, plan: &RemediationPlan) -> bool {
       // Check if any step requires approval based on risk level
       plan.steps.iter().any(|step| {
           match step.risk_level {
               RiskLevel::High => true,
               RiskLevel::Medium => !self.config.auto_approve_medium_risk,
               RiskLevel::Low => false,
           }
       })
   }
   ```
   
   **request_approval()**
   ```rust
   async fn request_approval(&self, plan: &RemediationPlan) -> Result<()> {
       // Display plan to user and wait for approval
       println!("\n=== REMEDIATION PLAN REQUIRES APPROVAL ===");
       println!("Alert: {}", plan.alert.labels.get("alertname").unwrap_or(&"Unknown".to_string()));
       println!("Root Cause: {}", plan.analysis.root_cause);
       println!("\nProposed Steps:");
       
       for (i, step) in plan.steps.iter().enumerate() {
           println!("{}. {} [Risk: {:?}]", i + 1, step.description, step.risk_level);
           println!("   Command: {}", step.command);
       }
       
       println!("\nApprove? (yes/no): ");
       
       // Wait for user input with timeout
       let timeout = Duration::from_secs(self.config.approval_timeout_seconds);
       let approval = tokio::time::timeout(timeout, self.wait_for_approval()).await??;
       
       if !approval {
           return Err(ReasoningError::ApprovalDenied);
       }
       
       Ok(())
   }
   ```

4. **Error Handling and Rollback**
   ```rust
   async fn handle_execution_failure(&self, plan: &RemediationPlan, step_index: usize, error: &str) -> Result<()> {
       tracing::error!("Execution failed at step {}: {}", step_index, error);
       
       // Attempt rollback of completed steps
       for i in (0..step_index).rev() {
           if let Some(rollback_cmd) = &plan.steps[i].rollback_command {
               tracing::info!("Rolling back step {}: {}", i, rollback_cmd);
               match self.executor.execute(rollback_cmd, true).await {
                   Ok(_) => tracing::info!("Rollback successful for step {}", i),
                   Err(e) => tracing::error!("Rollback failed for step {}: {}", i, e),
               }
           }
       }
       
       Err(ReasoningError::ExecutionFailed(error.to_string()))
   }
   ```

5. **Configuration**
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ReasoningConfig {
       pub auto_approve_low_risk: bool,
       pub auto_approve_medium_risk: bool,
       pub approval_timeout_seconds: u64,
       pub max_execution_time_seconds: u64,
       pub enable_rollback: bool,
       pub dry_run_mode: bool,
   }
   ```

Include comprehensive logging at each state transition and error handling for all failure scenarios.
```

## Expected Output

Bob should create:
1. `src/reasoning/mod.rs` - Main reasoning engine
2. `src/reasoning/workflow.rs` - State machine implementation
3. `src/reasoning/types.rs` - Workflow types and structs
4. State transition logic with proper error handling
5. Approval workflow implementation
6. Rollback mechanism

## Testing

Create integration tests:
```rust
#[tokio::test]
async fn test_full_workflow() {
    let engine = ReasoningEngine::new(config);
    let alert = create_test_alert();
    let report = engine.process_alert(alert).await.unwrap();
    assert_eq!(report.status, RemediationStatus::Success);
}
```

## Next Steps

After Bob completes this, proceed to Prompt 5 for the alert receiver and documentation generator.