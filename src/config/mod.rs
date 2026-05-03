//! Configuration management with hot-reload and validation
//! 
//! Provides:
//! - YAML/TOML configuration loading
//! - Environment variable overrides
//! - Configuration validation
//! - Hot-reload without restart
//! - Configuration versioning

use crate::error::{Result, SentinelError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub watsonx: WatsonxConfig,
    pub security: SecurityConfig,
    pub remediation: RemediationConfig,
    pub plugins: PluginsConfig,
    pub logging: LoggingConfig,
    pub metrics: MetricsConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub timeout_seconds: u64,
    pub enable_cors: bool,
    pub cors_origins: Vec<String>,
}

/// Watsonx configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatsonxConfig {
    pub api_key: String,
    pub project_id: String,
    pub url: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout_seconds: u64,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub auth_token: Option<String>,
    pub enable_tls: bool,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
    pub allowed_commands: Vec<String>,
    pub blocked_commands: Vec<String>,
    pub max_command_length: usize,
}

/// Remediation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationConfig {
    pub auto_approve_low_risk: bool,
    pub auto_approve_medium_risk: bool,
    pub approval_timeout_seconds: u64,
    pub max_execution_time_seconds: u64,
    pub enable_rollback: bool,
    pub dry_run_mode: bool,
    pub max_concurrent_remediations: usize,
}

/// Plugins configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginsConfig {
    pub enabled: bool,
    pub plugin_dir: PathBuf,
    pub auto_load: bool,
    pub enabled_plugins: Vec<String>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub output: String,
    pub file_path: Option<PathBuf>,
    pub max_file_size_mb: u64,
    pub max_files: usize,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub port: u16,
    pub path: String,
    pub collect_interval_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                workers: num_cpus::get(),
                timeout_seconds: 30,
                enable_cors: true,
                cors_origins: vec!["*".to_string()],
            },
            watsonx: WatsonxConfig {
                api_key: String::new(),
                project_id: String::new(),
                url: "https://us-south.ml.cloud.ibm.com".to_string(),
                model: "ibm/granite-13b-instruct-v2".to_string(),
                max_tokens: 1024,
                temperature: 0.7,
                timeout_seconds: 60,
            },
            security: SecurityConfig {
                auth_token: None,
                enable_tls: false,
                cert_path: None,
                key_path: None,
                allowed_commands: vec![
                    "systemctl".to_string(),
                    "kubectl".to_string(),
                    "docker".to_string(),
                ],
                blocked_commands: vec![
                    "rm -rf /".to_string(),
                    "dd".to_string(),
                    "mkfs".to_string(),
                ],
                max_command_length: 1000,
            },
            remediation: RemediationConfig {
                auto_approve_low_risk: true,
                auto_approve_medium_risk: false,
                approval_timeout_seconds: 300,
                max_execution_time_seconds: 600,
                enable_rollback: true,
                dry_run_mode: false,
                max_concurrent_remediations: 5,
            },
            plugins: PluginsConfig {
                enabled: true,
                plugin_dir: PathBuf::from("./plugins"),
                auto_load: true,
                enabled_plugins: vec![
                    "disk-cleanup".to_string(),
                    "service-restart".to_string(),
                ],
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                output: "stdout".to_string(),
                file_path: None,
                max_file_size_mb: 100,
                max_files: 10,
            },
            metrics: MetricsConfig {
                enabled: true,
                port: 9090,
                path: "/metrics".to_string(),
                collect_interval_seconds: 60,
            },
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| SentinelError::Config(format!("Failed to read config file: {}", e)))?;
        
        let config: Config = if path.as_ref().extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::from_str(&content)
                .map_err(|e| SentinelError::Config(format!("Failed to parse TOML: {}", e)))?
        } else {
            serde_yaml::from_str(&content)
                .map_err(|e| SentinelError::Config(format!("Failed to parse YAML: {}", e)))?
        };
        
        config.validate()?;
        Ok(config)
    }
    
    /// Load configuration with environment variable overrides
    pub fn from_file_with_env<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut config = Self::from_file(path)?;
        config.apply_env_overrides();
        config.validate()?;
        Ok(config)
    }
    
    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        if let Ok(port) = std::env::var("SENTINEL_PORT") {
            if let Ok(p) = port.parse() {
                self.server.port = p;
            }
        }
        
        if let Ok(api_key) = std::env::var("WATSONX_API_KEY") {
            self.watsonx.api_key = api_key;
        }
        
        if let Ok(project_id) = std::env::var("WATSONX_PROJECT_ID") {
            self.watsonx.project_id = project_id;
        }
        
        if let Ok(dry_run) = std::env::var("SENTINEL_DRY_RUN") {
            self.remediation.dry_run_mode = dry_run.to_lowercase() == "true";
        }
        
        if let Ok(log_level) = std::env::var("SENTINEL_LOG_LEVEL") {
            self.logging.level = log_level;
        }
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate server config
        if self.server.port == 0 {
            return Err(SentinelError::Config("Invalid server port".to_string()));
        }
        
        if self.server.workers == 0 {
            return Err(SentinelError::Config("Workers must be > 0".to_string()));
        }
        
        // Validate watsonx config
        if self.watsonx.api_key.is_empty() {
            return Err(SentinelError::Config("Watsonx API key is required".to_string()));
        }
        
        if self.watsonx.project_id.is_empty() {
            return Err(SentinelError::Config("Watsonx project ID is required".to_string()));
        }
        
        // Validate remediation config
        if self.remediation.max_concurrent_remediations == 0 {
            return Err(SentinelError::Config(
                "Max concurrent remediations must be > 0".to_string()
            ));
        }
        
        // Validate logging config
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(SentinelError::Config(format!(
                "Invalid log level: {}. Must be one of: {:?}",
                self.logging.level, valid_levels
            )));
        }
        
        Ok(())
    }
    
    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = if path.as_ref().extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::to_string_pretty(self)
                .map_err(|e| SentinelError::Config(format!("Failed to serialize TOML: {}", e)))?
        } else {
            serde_yaml::to_string(self)
                .map_err(|e| SentinelError::Config(format!("Failed to serialize YAML: {}", e)))?
        };
        
        std::fs::write(path, content)
            .map_err(|e| SentinelError::Config(format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }
}

/// Configuration manager with hot-reload
pub struct ConfigManager {
    config: Arc<RwLock<Config>>,
    config_path: PathBuf,
    last_modified: Arc<RwLock<std::time::SystemTime>>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(config_path: PathBuf) -> Result<Self> {
        let config = Config::from_file_with_env(&config_path)?;
        
        let metadata = std::fs::metadata(&config_path)
            .map_err(|e| SentinelError::Config(format!("Failed to get file metadata: {}", e)))?;
        let last_modified = metadata.modified()
            .map_err(|e| SentinelError::Config(format!("Failed to get modification time: {}", e)))?;
        
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
            last_modified: Arc::new(RwLock::new(last_modified)),
        })
    }
    
    /// Get current configuration
    pub async fn get(&self) -> Config {
        self.config.read().await.clone()
    }
    
    /// Reload configuration from file
    pub async fn reload(&self) -> Result<()> {
        tracing::info!("Reloading configuration from: {}", self.config_path.display());
        
        let new_config = Config::from_file_with_env(&self.config_path)?;
        
        let metadata = std::fs::metadata(&self.config_path)
            .map_err(|e| SentinelError::Config(format!("Failed to get file metadata: {}", e)))?;
        let modified = metadata.modified()
            .map_err(|e| SentinelError::Config(format!("Failed to get modification time: {}", e)))?;
        
        *self.config.write().await = new_config;
        *self.last_modified.write().await = modified;
        
        tracing::info!("Configuration reloaded successfully");
        Ok(())
    }
    
    /// Check if configuration file has been modified
    pub async fn check_for_changes(&self) -> Result<bool> {
        let metadata = std::fs::metadata(&self.config_path)
            .map_err(|e| SentinelError::Config(format!("Failed to get file metadata: {}", e)))?;
        let modified = metadata.modified()
            .map_err(|e| SentinelError::Config(format!("Failed to get modification time: {}", e)))?;
        
        let last_modified = *self.last_modified.read().await;
        Ok(modified > last_modified)
    }
    
    /// Start watching for configuration changes
    pub async fn watch(self: Arc<Self>, interval: Duration) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            
            loop {
                interval.tick().await;
                
                match self.check_for_changes().await {
                    Ok(true) => {
                        tracing::info!("Configuration file changed, reloading...");
                        if let Err(e) = self.reload().await {
                            tracing::error!("Failed to reload configuration: {}", e);
                        }
                    }
                    Ok(false) => {}
                    Err(e) => {
                        tracing::error!("Failed to check for configuration changes: {}", e);
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    #[test]
    fn test_default_config() {
        let mut config = Config::default();
        config.watsonx.api_key = "test".to_string();
        config.watsonx.project_id = "test".to_string();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        config.watsonx.api_key = String::new();
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_save_load() {
        let mut config = Config::default();
        config.watsonx.api_key = "test".to_string();
        config.watsonx.project_id = "test".to_string();
        let mut temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().with_extension("yaml");
        
        config.save_to_file(&path).unwrap();
        let loaded = Config::from_file(&path).unwrap();
        
        assert_eq!(config.server.port, loaded.server.port);
    }
}

// Made with Bob