//! Real-time notification system using WebSockets
//! 
//! Provides real-time updates on:
//! - Alert processing status
//! - Remediation progress
//! - System health
//! - Plugin events

use crate::error::{Result, SentinelError};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// Notification event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum NotificationEvent {
    /// Alert received
    AlertReceived {
        alert_id: String,
        alert_name: String,
        severity: String,
        timestamp: String,
    },
    
    /// Remediation started
    RemediationStarted {
        remediation_id: String,
        alert_name: String,
        estimated_duration: u64,
    },
    
    /// Remediation step progress
    RemediationProgress {
        remediation_id: String,
        step: usize,
        total_steps: usize,
        description: String,
        status: String,
    },
    
    /// Remediation completed
    RemediationCompleted {
        remediation_id: String,
        success: bool,
        duration_ms: u64,
        message: String,
    },
    
    /// System health update
    HealthUpdate {
        status: String,
        active_remediations: usize,
        queue_length: usize,
        metrics: HashMap<String, String>,
    },
    
    /// Plugin event
    PluginEvent {
        plugin_name: String,
        event_type: String,
        message: String,
    },
    
    /// Error occurred
    Error {
        error_type: String,
        message: String,
        timestamp: String,
    },
    
    /// Custom event
    Custom {
        event_name: String,
        payload: serde_json::Value,
    },
}

/// WebSocket client connection
struct WsClient {
    id: String,
    sender: tokio::sync::mpsc::UnboundedSender<Message>,
    subscriptions: Vec<String>,
}

/// Notification manager
pub struct NotificationManager {
    clients: Arc<RwLock<HashMap<String, WsClient>>>,
    broadcast_tx: broadcast::Sender<NotificationEvent>,
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
        }
    }
    
    /// Send a notification to all connected clients
    pub async fn notify(&self, event: NotificationEvent) -> Result<()> {
        tracing::debug!("Broadcasting notification: {:?}", event);
        
        // Broadcast to all subscribers
        if let Err(e) = self.broadcast_tx.send(event.clone()) {
            tracing::warn!("Failed to broadcast notification: {}", e);
        }
        
        // Also send directly to connected clients
        let clients = self.clients.read().await;
        let json = serde_json::to_string(&event)
            .map_err(|e| SentinelError::Serialization(e))?;
        
        for client in clients.values() {
            if let Err(e) = client.sender.send(Message::Text(json.clone())) {
                tracing::warn!("Failed to send to client {}: {}", client.id, e);
            }
        }
        
        Ok(())
    }
    
    /// Register a new WebSocket client
    async fn register_client(
        &self,
        sender: tokio::sync::mpsc::UnboundedSender<Message>,
    ) -> String {
        let client_id = Uuid::new_v4().to_string();
        
        let client = WsClient {
            id: client_id.clone(),
            sender,
            subscriptions: vec!["*".to_string()], // Subscribe to all by default
        };
        
        let mut clients = self.clients.write().await;
        clients.insert(client_id.clone(), client);
        
        tracing::info!("WebSocket client registered: {}", client_id);
        client_id
    }
    
    /// Unregister a WebSocket client
    async fn unregister_client(&self, client_id: &str) {
        let mut clients = self.clients.write().await;
        clients.remove(client_id);
        tracing::info!("WebSocket client unregistered: {}", client_id);
    }
    
    /// Get number of connected clients
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }
    
    /// Create router for WebSocket endpoints
    pub fn router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/ws", get(ws_handler))
            .with_state(self)
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket upgrade handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(manager): State<Arc<NotificationManager>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, manager))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, manager: Arc<NotificationManager>) {
    let (mut sender, mut receiver) = socket.split();
    
    // Create channel for sending messages to this client
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    
    // Register client
    let client_id = manager.register_client(tx).await;
    
    // Subscribe to broadcast channel
    let mut broadcast_rx = manager.broadcast_tx.subscribe();
    
    // Send welcome message
    let welcome = NotificationEvent::Custom {
        event_name: "connected".to_string(),
        payload: serde_json::json!({
            "client_id": client_id,
            "message": "Connected to Sentinel-MCP notifications"
        }),
    };
    
    if let Ok(json) = serde_json::to_string(&welcome) {
        let _ = sender.send(Message::Text(json)).await;
    }
    
    // Spawn task to forward broadcast messages
    let client_id_clone = client_id.clone();
    let manager_clone = Arc::clone(&manager);
    tokio::spawn(async move {
        while let Ok(event) = broadcast_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&event) {
                let clients = manager_clone.clients.read().await;
                if let Some(client) = clients.get(&client_id_clone) {
                    let _ = client.sender.send(Message::Text(json));
                }
            }
        }
    });
    
    // Spawn task to send messages from channel to WebSocket
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });
    
    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Handle client commands
                if let Ok(cmd) = serde_json::from_str::<ClientCommand>(&text) {
                    handle_client_command(&manager, &client_id, cmd).await;
                }
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Ok(Message::Ping(data)) => {
                // Respond to ping
                let clients = manager.clients.read().await;
                if let Some(client) = clients.get(&client_id) {
                    let _ = client.sender.send(Message::Pong(data));
                }
            }
            _ => {}
        }
    }
    
    // Unregister client on disconnect
    manager.unregister_client(&client_id).await;
}

/// Client command
#[derive(Debug, Deserialize)]
#[serde(tag = "command")]
enum ClientCommand {
    Subscribe { topics: Vec<String> },
    Unsubscribe { topics: Vec<String> },
    GetStatus,
}

/// Handle client command
async fn handle_client_command(
    manager: &Arc<NotificationManager>,
    client_id: &str,
    command: ClientCommand,
) {
    match command {
        ClientCommand::Subscribe { topics } => {
            let mut clients = manager.clients.write().await;
            if let Some(client) = clients.get_mut(client_id) {
                client.subscriptions.extend(topics.clone());
                tracing::info!("Client {} subscribed to: {:?}", client_id, topics);
            }
        }
        ClientCommand::Unsubscribe { topics } => {
            let mut clients = manager.clients.write().await;
            if let Some(client) = clients.get_mut(client_id) {
                client.subscriptions.retain(|t| !topics.contains(t));
                tracing::info!("Client {} unsubscribed from: {:?}", client_id, topics);
            }
        }
        ClientCommand::GetStatus => {
            let client_count = manager.client_count().await;
            let status = NotificationEvent::Custom {
                event_name: "status".to_string(),
                payload: serde_json::json!({
                    "connected_clients": client_count,
                    "server_status": "operational"
                }),
            };
            
            let clients = manager.clients.read().await;
            if let Some(client) = clients.get(client_id) {
                if let Ok(json) = serde_json::to_string(&status) {
                    let _ = client.sender.send(Message::Text(json));
                }
            }
        }
    }
}

/// Helper to create notification events
pub mod events {
    use super::*;
    
    pub fn alert_received(alert_id: String, alert_name: String, severity: String) -> NotificationEvent {
        NotificationEvent::AlertReceived {
            alert_id,
            alert_name,
            severity,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
    
    pub fn remediation_started(
        remediation_id: String,
        alert_name: String,
        estimated_duration: u64,
    ) -> NotificationEvent {
        NotificationEvent::RemediationStarted {
            remediation_id,
            alert_name,
            estimated_duration,
        }
    }
    
    pub fn remediation_progress(
        remediation_id: String,
        step: usize,
        total_steps: usize,
        description: String,
        status: String,
    ) -> NotificationEvent {
        NotificationEvent::RemediationProgress {
            remediation_id,
            step,
            total_steps,
            description,
            status,
        }
    }
    
    pub fn remediation_completed(
        remediation_id: String,
        success: bool,
        duration_ms: u64,
        message: String,
    ) -> NotificationEvent {
        NotificationEvent::RemediationCompleted {
            remediation_id,
            success,
            duration_ms,
            message,
        }
    }
    
    pub fn health_update(
        status: String,
        active_remediations: usize,
        queue_length: usize,
        metrics: HashMap<String, String>,
    ) -> NotificationEvent {
        NotificationEvent::HealthUpdate {
            status,
            active_remediations,
            queue_length,
            metrics,
        }
    }
    
    pub fn error(error_type: String, message: String) -> NotificationEvent {
        NotificationEvent::Error {
            error_type,
            message,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_notification_manager() {
        let manager = NotificationManager::new();
        assert_eq!(manager.client_count().await, 0);
    }
    
    #[test]
    fn test_notification_serialization() {
        let event = NotificationEvent::AlertReceived {
            alert_id: "test-123".to_string(),
            alert_name: "DiskSpaceLow".to_string(),
            severity: "warning".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };
        
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("AlertReceived"));
        assert!(json.contains("test-123"));
    }
}

// Made with Bob