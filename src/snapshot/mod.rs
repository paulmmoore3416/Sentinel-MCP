//! System snapshot module
//! 
//! This module provides functionality to capture and restore system state
//! for safe rollback of remediation operations.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};

/// Complete system snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub filesystem_state: FilesystemSnapshot,
    pub service_states: Vec<ServiceState>,
    pub k8s_resources: Option<K8sSnapshot>,
    pub environment_vars: HashMap<String, String>,
    pub checksum: String,
}

/// Filesystem snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemSnapshot {
    pub files: HashMap<PathBuf, FileMetadata>,
    pub directories: Vec<PathBuf>,
}

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub permissions: u32,
    pub checksum: String,
    pub content_backup: Option<Vec<u8>>,
}

/// Service state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceState {
    pub name: String,
    pub active: bool,
    pub enabled: bool,
    pub pid: Option<u32>,
    pub memory_usage: Option<u64>,
}

/// Kubernetes snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sSnapshot {
    pub namespace: String,
    pub pods: Vec<PodState>,
    pub services: Vec<K8sServiceState>,
    pub deployments: Vec<DeploymentState>,
}

/// Pod state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodState {
    pub name: String,
    pub namespace: String,
    pub phase: String,
    pub restart_count: i32,
    pub ready: bool,
}

/// Kubernetes service state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sServiceState {
    pub name: String,
    pub namespace: String,
    pub cluster_ip: String,
    pub ports: Vec<u16>,
}

/// Deployment state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentState {
    pub name: String,
    pub namespace: String,
    pub replicas: i32,
    pub ready_replicas: i32,
}

/// Snapshot manager
pub struct SnapshotManager {
    storage_path: PathBuf,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&storage_path)?;
        Ok(Self { storage_path })
    }

    /// Capture a complete system snapshot
    pub async fn capture_snapshot(
        &self,
        paths_to_monitor: &[PathBuf],
        services_to_monitor: &[String],
        k8s_namespace: Option<&str>,
    ) -> Result<SystemSnapshot> {
        tracing::info!("Capturing system snapshot");

        let filesystem_state = self.capture_filesystem_state(paths_to_monitor).await?;
        let service_states = self.capture_service_states(services_to_monitor).await?;
        let k8s_resources = if let Some(ns) = k8s_namespace {
            Some(self.capture_k8s_state(ns).await?)
        } else {
            None
        };
        let environment_vars = self.capture_environment_vars();

        let snapshot = SystemSnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            filesystem_state,
            service_states,
            k8s_resources,
            environment_vars,
            checksum: String::new(), // Will be calculated
        };

        // Calculate checksum
        let checksum = self.calculate_snapshot_checksum(&snapshot)?;
        let mut snapshot = snapshot;
        snapshot.checksum = checksum;

        // Save snapshot
        self.save_snapshot(&snapshot).await?;

        Ok(snapshot)
    }

    /// Capture filesystem state
    async fn capture_filesystem_state(
        &self,
        paths: &[PathBuf],
    ) -> Result<FilesystemSnapshot> {
        let mut files = HashMap::new();
        let mut directories = Vec::new();

        for path in paths {
            if path.is_file() {
                let metadata = self.capture_file_metadata(path).await?;
                files.insert(path.clone(), metadata);
            } else if path.is_dir() {
                directories.push(path.clone());
                // Recursively capture files in directory
                self.capture_directory_files(path, &mut files).await?;
            }
        }

        Ok(FilesystemSnapshot { files, directories })
    }

    /// Capture file metadata
    async fn capture_file_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let metadata = fs::metadata(path)?;
        let content = fs::read(path)?;
        let checksum = self.calculate_file_checksum(&content);

        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode()
        };
        
        #[cfg(not(unix))]
        let permissions = 0o644;

        Ok(FileMetadata {
            path: path.to_path_buf(),
            size: metadata.len(),
            modified: metadata.modified()?.into(),
            permissions,
            checksum,
            content_backup: Some(content),
        })
    }

    /// Capture directory files recursively
    fn capture_directory_files<'a>(
        &'a self,
        dir: &'a Path,
        files: &'a mut HashMap<PathBuf, FileMetadata>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    let metadata = self.capture_file_metadata(&path).await?;
                    files.insert(path, metadata);
                } else if path.is_dir() {
                    self.capture_directory_files(&path, files).await?;
                }
            }
            Ok(())
        })
    }

    /// Capture service states
    async fn capture_service_states(&self, services: &[String]) -> Result<Vec<ServiceState>> {
        let mut states = Vec::new();

        for service_name in services {
            if let Ok(state) = self.capture_service_state(service_name).await {
                states.push(state);
            }
        }

        Ok(states)
    }

    /// Capture single service state
    async fn capture_service_state(&self, service_name: &str) -> Result<ServiceState> {
        // Use systemctl to get service status
        let output = tokio::process::Command::new("systemctl")
            .args(&["is-active", service_name])
            .output()
            .await?;

        let active = output.status.success();

        let output = tokio::process::Command::new("systemctl")
            .args(&["is-enabled", service_name])
            .output()
            .await?;

        let enabled = output.status.success();

        // Get PID if service is active
        let pid = if active {
            let output = tokio::process::Command::new("systemctl")
                .args(&["show", service_name, "--property=MainPID", "--value"])
                .output()
                .await?;
            
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<u32>()
                .ok()
        } else {
            None
        };

        Ok(ServiceState {
            name: service_name.to_string(),
            active,
            enabled,
            pid,
            memory_usage: None, // TODO: Capture memory usage
        })
    }

    /// Capture Kubernetes state
    async fn capture_k8s_state(&self, namespace: &str) -> Result<K8sSnapshot> {
        use kube::{Api, Client};
        use k8s_openapi::api::core::v1::{Pod, Service};
        use k8s_openapi::api::apps::v1::Deployment;

        let client = Client::try_default().await?;

        // Capture pods
        let pods_api: Api<Pod> = Api::namespaced(client.clone(), namespace);
        let pods_list = pods_api.list(&Default::default()).await?;
        let pods = pods_list
            .items
            .iter()
            .map(|pod| {
                let status = pod.status.as_ref();
                PodState {
                    name: pod.metadata.name.clone().unwrap_or_default(),
                    namespace: namespace.to_string(),
                    phase: status
                        .and_then(|s| s.phase.clone())
                        .unwrap_or_else(|| "Unknown".to_string()),
                    restart_count: status
                        .and_then(|s| s.container_statuses.as_ref())
                        .and_then(|cs| cs.first())
                        .map(|c| c.restart_count)
                        .unwrap_or(0),
                    ready: status
                        .and_then(|s| s.conditions.as_ref())
                        .and_then(|conds| conds.iter().find(|c| c.type_ == "Ready"))
                        .map(|c| c.status == "True")
                        .unwrap_or(false),
                }
            })
            .collect();

        // Capture services
        let services_api: Api<Service> = Api::namespaced(client.clone(), namespace);
        let services_list = services_api.list(&Default::default()).await?;
        let services = services_list
            .items
            .iter()
            .map(|svc| {
                let spec = svc.spec.as_ref();
                K8sServiceState {
                    name: svc.metadata.name.clone().unwrap_or_default(),
                    namespace: namespace.to_string(),
                    cluster_ip: spec
                        .and_then(|s| s.cluster_ip.clone())
                        .unwrap_or_default(),
                    ports: spec
                        .and_then(|s| s.ports.as_ref())
                        .map(|ports| ports.iter().map(|p| p.port as u16).collect())
                        .unwrap_or_default(),
                }
            })
            .collect();

        // Capture deployments
        let deployments_api: Api<Deployment> = Api::namespaced(client, namespace);
        let deployments_list = deployments_api.list(&Default::default()).await?;
        let deployments = deployments_list
            .items
            .iter()
            .map(|dep| {
                let status = dep.status.as_ref();
                DeploymentState {
                    name: dep.metadata.name.clone().unwrap_or_default(),
                    namespace: namespace.to_string(),
                    replicas: dep.spec.as_ref().and_then(|s| s.replicas).unwrap_or(0),
                    ready_replicas: status.and_then(|s| s.ready_replicas).unwrap_or(0),
                }
            })
            .collect();

        Ok(K8sSnapshot {
            namespace: namespace.to_string(),
            pods,
            services,
            deployments,
        })
    }

    /// Capture environment variables
    fn capture_environment_vars(&self) -> HashMap<String, String> {
        std::env::vars().collect()
    }

    /// Calculate file checksum
    fn calculate_file_checksum(&self, content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    /// Calculate snapshot checksum
    fn calculate_snapshot_checksum(&self, snapshot: &SystemSnapshot) -> Result<String> {
        let json = serde_json::to_string(snapshot)?;
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Save snapshot to disk
    async fn save_snapshot(&self, snapshot: &SystemSnapshot) -> Result<()> {
        let path = self.storage_path.join(format!("{}.json", snapshot.id));
        let json = serde_json::to_string_pretty(snapshot)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    /// Load snapshot from disk
    pub async fn load_snapshot(&self, snapshot_id: &str) -> Result<SystemSnapshot> {
        let path = self.storage_path.join(format!("{}.json", snapshot_id));
        let json = tokio::fs::read_to_string(path).await?;
        let snapshot: SystemSnapshot = serde_json::from_str(&json)?;
        Ok(snapshot)
    }

    /// Compare two snapshots and detect changes
    pub fn compare_snapshots(
        &self,
        before: &SystemSnapshot,
        after: &SystemSnapshot,
    ) -> SnapshotDiff {
        let mut changed_files = Vec::new();
        let mut new_files = Vec::new();
        let mut deleted_files = Vec::new();

        // Check for changed and deleted files
        for (path, before_meta) in &before.filesystem_state.files {
            if let Some(after_meta) = after.filesystem_state.files.get(path) {
                if before_meta.checksum != after_meta.checksum {
                    changed_files.push(path.clone());
                }
            } else {
                deleted_files.push(path.clone());
            }
        }

        // Check for new files
        for path in after.filesystem_state.files.keys() {
            if !before.filesystem_state.files.contains_key(path) {
                new_files.push(path.clone());
            }
        }

        let service_changes = self.compare_service_states(
            &before.service_states,
            &after.service_states,
        );

        let has_changes = !changed_files.is_empty()
            || !new_files.is_empty()
            || !deleted_files.is_empty()
            || !service_changes.is_empty();

        SnapshotDiff {
            changed_files,
            new_files,
            deleted_files,
            service_changes,
            has_changes,
        }
    }

    /// Compare service states
    fn compare_service_states(
        &self,
        before: &[ServiceState],
        after: &[ServiceState],
    ) -> Vec<ServiceChange> {
        let mut changes = Vec::new();

        for before_state in before {
            if let Some(after_state) = after.iter().find(|s| s.name == before_state.name) {
                if before_state.active != after_state.active {
                    changes.push(ServiceChange {
                        service_name: before_state.name.clone(),
                        change_type: if after_state.active {
                            "started".to_string()
                        } else {
                            "stopped".to_string()
                        },
                    });
                }
            }
        }

        changes
    }

    /// Restore system to snapshot state
    pub async fn restore_snapshot(&self, snapshot: &SystemSnapshot) -> Result<()> {
        tracing::info!("Restoring system snapshot: {}", snapshot.id);

        // Restore files
        for (path, metadata) in &snapshot.filesystem_state.files {
            if let Some(content) = &metadata.content_backup {
                tokio::fs::write(path, content).await?;
                
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let permissions = std::fs::Permissions::from_mode(metadata.permissions);
                    tokio::fs::set_permissions(path, permissions).await?;
                }
            }
        }

        // Restore services
        for service_state in &snapshot.service_states {
            if service_state.active {
                let _ = tokio::process::Command::new("systemctl")
                    .args(&["start", &service_state.name])
                    .output()
                    .await;
            } else {
                let _ = tokio::process::Command::new("systemctl")
                    .args(&["stop", &service_state.name])
                    .output()
                    .await;
            }
        }

        tracing::info!("Snapshot restored successfully");
        Ok(())
    }
}

/// Snapshot comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub changed_files: Vec<PathBuf>,
    pub new_files: Vec<PathBuf>,
    pub deleted_files: Vec<PathBuf>,
    pub service_changes: Vec<ServiceChange>,
    pub has_changes: bool,
}

/// Service change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceChange {
    pub service_name: String,
    pub change_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_snapshot_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SnapshotManager::new(temp_dir.path().to_path_buf());
        assert!(manager.is_ok());
    }

    #[test]
    fn test_file_checksum() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SnapshotManager::new(temp_dir.path().to_path_buf()).unwrap();
        
        let content = b"test content";
        let checksum1 = manager.calculate_file_checksum(content);
        let checksum2 = manager.calculate_file_checksum(content);
        
        assert_eq!(checksum1, checksum2);
        assert!(!checksum1.is_empty());
    }
}

// Made with Bob