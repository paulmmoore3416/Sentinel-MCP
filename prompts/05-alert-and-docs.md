# Prompt 5: Alert Receiver and Documentation Generator

## Context
We need to implement the Prometheus AlertManager webhook receiver and the automatic documentation generator.

## Prompt for Bob (Code Mode)

```
Bob, implement the alert receiver and documentation generator for Sentinel-MCP:

## Part A: Alert Receiver (src/alert/mod.rs)

Create an HTTP server that receives Prometheus AlertManager webhooks:

1. **AlertReceiver struct**
   ```rust
   pub struct AlertReceiver {
       reasoning_engine: Arc<ReasoningEngine>,
       alert_queue: Arc<Mutex<VecDeque<Alert>>>,
       config: AlertConfig,
   }
   ```

2. **HTTP Endpoints**
   
   **POST /api/v1/alerts** - Receive AlertManager webhooks
   ```rust
   async fn receive_alerts(
       State(receiver): State<Arc<AlertReceiver>>,
       Json(payload): Json<AlertManagerPayload>,
   ) -> Result<Json<AlertResponse>, StatusCode> {
       // Parse and validate alerts
       for alert in payload.alerts {
           if alert.status == "firing" {
               receiver.process_alert(alert).await?;
           }
       }
       
       Ok(Json(AlertResponse {
           received: payload.alerts.len(),
           status: "processing",
       }))
   }
   ```
   
   **GET /api/v1/health** - Health check endpoint
   ```rust
   async fn health_check() -> Json<HealthResponse> {
       Json(HealthResponse {
           status: "healthy",
           version: env!("CARGO_PKG_VERSION"),
           uptime_seconds: get_uptime(),
       })
   }
   ```
   
   **GET /api/v1/status** - Get current processing status
   ```rust
   async fn get_status(
       State(receiver): State<Arc<AlertReceiver>>,
   ) -> Json<StatusResponse> {
       Json(StatusResponse {
           queue_length: receiver.alert_queue.lock().await.len(),
           active_remediations: receiver.get_active_count(),
           total_processed: receiver.get_total_processed(),
       })
   }
   ```

3. **Alert Parsing**
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct AlertManagerPayload {
       pub version: String,
       pub group_key: String,
       pub status: String,
       pub receiver: String,
       pub group_labels: HashMap<String, String>,
       pub common_labels: HashMap<String, String>,
       pub common_annotations: HashMap<String, String>,
       pub external_url: String,
       pub alerts: Vec<Alert>,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Alert {
       pub status: String,
       pub labels: HashMap<String, String>,
       pub annotations: HashMap<String, String>,
       #[serde(rename = "startsAt")]
       pub starts_at: String,
       #[serde(rename = "endsAt")]
       pub ends_at: Option<String>,
       #[serde(rename = "generatorURL")]
       pub generator_url: String,
       pub fingerprint: String,
   }
   ```

4. **Alert Processing**
   ```rust
   impl AlertReceiver {
       async fn process_alert(&self, alert: Alert) -> Result<()> {
           // Validate alert
           self.validate_alert(&alert)?;
           
           // Check for duplicates
           if self.is_duplicate(&alert).await {
               tracing::info!("Skipping duplicate alert: {}", alert.fingerprint);
               return Ok(());
           }
           
           // Add to queue
           self.alert_queue.lock().await.push_back(alert.clone());
           
           // Trigger reasoning engine
           tokio::spawn({
               let engine = self.reasoning_engine.clone();
               let alert = alert.clone();
               async move {
                   match engine.process_alert(alert).await {
                       Ok(report) => {
                           tracing::info!("Remediation completed: {}", report.id);
                       }
                       Err(e) => {
                           tracing::error!("Remediation failed: {}", e);
                       }
                   }
               }
           });
           
           Ok(())
       }
   }
   ```

5. **Server Setup**
   ```rust
   pub async fn start_server(config: AlertConfig) -> Result<()> {
       let receiver = Arc::new(AlertReceiver::new(config));
       
       let app = Router::new()
           .route("/api/v1/alerts", post(receive_alerts))
           .route("/api/v1/health", get(health_check))
           .route("/api/v1/status", get(get_status))
           .layer(
               ServiceBuilder::new()
                   .layer(TraceLayer::new_for_http())
                   .layer(TimeoutLayer::new(Duration::from_secs(30)))
           )
           .with_state(receiver);
       
       let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
       tracing::info!("Starting alert receiver on {}", addr);
       
       axum::Server::bind(&addr)
           .serve(app.into_make_service())
           .await?;
       
       Ok(())
   }
   ```

## Part B: Documentation Generator (src/executor/documentation.rs)

Create automatic documentation generation:

1. **DocumentationGenerator struct**
   ```rust
   pub struct DocumentationGenerator {
       output_dir: PathBuf,
       template: String,
   }
   ```

2. **generate_report() method**
   ```rust
   pub async fn generate_report(&self, context: &RemediationContext) -> Result<RemediationReport> {
       let report = RemediationReport {
           id: Uuid::new_v4().to_string(),
           timestamp: Utc::now(),
           alert: context.alert.clone(),
           analysis: context.analysis.clone(),
           plan: context.plan.clone(),
           execution_result: context.execution_result.clone(),
           verification: context.verification.clone(),
       };
       
       // Generate markdown
       let markdown = self.generate_markdown(&report)?;
       
       // Write to file
       let filename = format!("REMEDIATION_LOG_{}.md", report.timestamp.format("%Y%m%d_%H%M%S"));
       let filepath = self.output_dir.join(&filename);
       tokio::fs::write(&filepath, markdown).await?;
       
       tracing::info!("Generated remediation report: {}", filepath.display());
       
       Ok(report)
   }
   ```

3. **Markdown Template**
   ```rust
   fn generate_markdown(&self, report: &RemediationReport) -> Result<String> {
       let mut md = String::new();
       
       md.push_str(&format!("# Remediation Report: {}\n\n", report.alert.labels.get("alertname").unwrap_or(&"Unknown".to_string())));
       md.push_str(&format!("**Report ID**: {}\n", report.id));
       md.push_str(&format!("**Timestamp**: {}\n", report.timestamp.to_rfc3339()));
       md.push_str(&format!("**Status**: {:?}\n\n", report.execution_result.status));
       
       md.push_str("## Alert Details\n\n");
       md.push_str(&format!("- **Alert Name**: {}\n", report.alert.labels.get("alertname").unwrap_or(&"N/A".to_string())));
       md.push_str(&format!("- **Severity**: {}\n", report.alert.labels.get("severity").unwrap_or(&"N/A".to_string())));
       md.push_str(&format!("- **Instance**: {}\n", report.alert.labels.get("instance").unwrap_or(&"N/A".to_string())));
       md.push_str(&format!("- **Started At**: {}\n\n", report.alert.starts_at));
       
       md.push_str("## Root Cause Analysis\n\n");
       md.push_str(&format!("{}\n\n", report.analysis.root_cause));
       md.push_str("**Affected Components**:\n");
       for component in &report.analysis.affected_components {
           md.push_str(&format!("- {}\n", component));
       }
       md.push_str("\n");
       
       md.push_str("## Remediation Steps Executed\n\n");
       for (i, step) in report.plan.steps.iter().enumerate() {
           md.push_str(&format!("### Step {}: {}\n\n", i + 1, step.description));
           md.push_str(&format!("**Command**: `{}`\n\n", step.command));
           md.push_str(&format!("**Risk Level**: {:?}\n\n", step.risk_level));
           
           if let Some(result) = report.execution_result.step_results.get(i) {
               md.push_str(&format!("**Result**: {}\n", if result.success { "✅ Success" } else { "❌ Failed" }));
               if !result.stdout.is_empty() {
                   md.push_str(&format!("\n**Output**:\n```\n{}\n```\n", result.stdout));
               }
               if !result.stderr.is_empty() {
                   md.push_str(&format!("\n**Errors**:\n```\n{}\n```\n", result.stderr));
               }
           }
           md.push_str("\n");
       }
       
       md.push_str("## Verification\n\n");
       md.push_str(&format!("**Status**: {}\n", if report.verification.success { "✅ Verified" } else { "❌ Failed" }));
       md.push_str(&format!("**Details**: {}\n\n", report.verification.details));
       
       md.push_str("## Metrics\n\n");
       md.push_str(&format!("- **Total Duration**: {} seconds\n", report.execution_result.total_duration_seconds));
       md.push_str(&format!("- **Steps Executed**: {}\n", report.plan.steps.len()));
       md.push_str(&format!("- **Success Rate**: {:.1}%\n\n", report.execution_result.success_rate * 100.0));
       
       md.push_str("## Lessons Learned\n\n");
       md.push_str(&format!("{}\n\n", report.analysis.lessons_learned.unwrap_or_else(|| "N/A".to_string())));
       
       md.push_str("---\n");
       md.push_str(&format!("*Generated by Sentinel-MCP v{} using IBM watsonx.ai*\n", env!("CARGO_PKG_VERSION")));
       
       Ok(md)
   }
   ```

4. **Export Formats**
   ```rust
   pub async fn export_json(&self, report: &RemediationReport) -> Result<PathBuf> {
       let filename = format!("remediation_{}.json", report.id);
       let filepath = self.output_dir.join(&filename);
       let json = serde_json::to_string_pretty(report)?;
       tokio::fs::write(&filepath, json).await?;
       Ok(filepath)
   }
   
   pub async fn export_html(&self, report: &RemediationReport) -> Result<PathBuf> {
       let markdown = self.generate_markdown(report)?;
       let html = markdown_to_html(&markdown);
       let filename = format!("remediation_{}.html", report.id);
       let filepath = self.output_dir.join(&filename);
       tokio::fs::write(&filepath, html).await?;
       Ok(filepath)
   }
   ```

5. **Summary Report**
   ```rust
   pub async fn generate_summary(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<String> {
       // Load all reports in date range
       let reports = self.load_reports_in_range(start_date, end_date).await?;
       
       let mut summary = String::new();
       summary.push_str(&format!("# Remediation Summary Report\n\n"));
       summary.push_str(&format!("**Period**: {} to {}\n\n", start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d")));
       
       summary.push_str("## Statistics\n\n");
       summary.push_str(&format!("- **Total Incidents**: {}\n", reports.len()));
       summary.push_str(&format!("- **Successful Remediations**: {}\n", reports.iter().filter(|r| r.execution_result.status == RemediationStatus::Success).count()));
       summary.push_str(&format!("- **Failed Remediations**: {}\n", reports.iter().filter(|r| r.execution_result.status == RemediationStatus::Failed).count()));
       
       let avg_duration: f64 = reports.iter().map(|r| r.execution_result.total_duration_seconds).sum::<f64>() / reports.len() as f64;
       summary.push_str(&format!("- **Average MTTR**: {:.1} seconds\n\n", avg_duration));
       
       summary.push_str("## Top Issues\n\n");
       // Group by alert name and count
       let mut issue_counts: HashMap<String, usize> = HashMap::new();
       for report in &reports {
           *issue_counts.entry(report.alert.labels.get("alertname").unwrap_or(&"Unknown".to_string()).clone()).or_insert(0) += 1;
       }
       
       let mut sorted_issues: Vec<_> = issue_counts.iter().collect();
       sorted_issues.sort_by(|a, b| b.1.cmp(a.1));
       
       for (issue, count) in sorted_issues.iter().take(10) {
           summary.push_str(&format!("- **{}**: {} occurrences\n", issue, count));
       }
       
       Ok(summary)
   }
   ```

Include comprehensive error handling and logging for all operations.
```

## Expected Output

Bob should create:
1. `src/alert/mod.rs` - Alert receiver with HTTP server
2. `src/alert/parser.rs` - Alert parsing logic
3. `src/executor/documentation.rs` - Documentation generator
4. HTTP endpoints for receiving alerts
5. Markdown template for reports
6. Summary report generation

## Testing

Test the alert receiver:
```bash
# Send test alert
curl -X POST http://localhost:3000/api/v1/alerts \
  -H "Content-Type: application/json" \
  -d @examples/alerts/test-alert.json

# Check status
curl http://localhost:3000/api/v1/status
```

## Next Steps

After Bob completes this, proceed to Prompt 6 for testing and demo preparation.