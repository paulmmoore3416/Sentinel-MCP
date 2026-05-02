use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod alert;
mod executor;
mod mcp;
mod reasoning;
mod watsonx;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sentinel_mcp=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Sentinel-MCP server...");

    // Get configuration from environment
    let port = std::env::var("MCP_SERVER_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()?;

    // Build the application router
    let app = Router::new()
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/alerts", post(alert::receive_alerts))
        .route("/api/v1/status", get(get_status))
        .layer((
            TraceLayer::new_for_http(),
            TimeoutLayer::new(std::time::Duration::from_secs(30)),
        ));

    // Start the server
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

/// Health check endpoint
async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "service": "sentinel-mcp"
    }))
}

/// Status endpoint
async fn get_status() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "operational",
        "active_remediations": 0,
        "total_processed": 0
    }))
}

// Made with Bob
