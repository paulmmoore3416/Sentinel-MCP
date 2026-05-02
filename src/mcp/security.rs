//! Security validation for MCP operations
//! 
//! This module provides security checks and validation for all MCP tool operations,
//! ensuring that only safe, approved commands are executed.

use super::RiskLevel;
use anyhow::{anyhow, Result};

/// Security validator for MCP operations
pub struct SecurityValidator {
    high_risk_commands: Vec<String>,
    low_risk_commands: Vec<String>,
}

impl SecurityValidator {
    pub fn new() -> Self {
        Self {
            high_risk_commands: vec![
                "rm -rf".to_string(),
                "DROP DATABASE".to_string(),
                "kubectl delete namespace".to_string(),
                "mkfs".to_string(),
                "dd if=/dev/zero".to_string(),
            ],
            low_risk_commands: vec![
                "systemctl restart".to_string(),
                "kubectl rollout restart".to_string(),
                "docker restart".to_string(),
                "service restart".to_string(),
            ],
        }
    }

    /// Validate a command and classify its risk level
    pub fn validate_command(&self, command: &str) -> Result<RiskLevel> {
        // Check for high-risk patterns
        for pattern in &self.high_risk_commands {
            if command.contains(pattern) {
                return Err(anyhow!("High-risk command detected: {}", pattern));
            }
        }

        // Check for low-risk patterns
        for pattern in &self.low_risk_commands {
            if command.contains(pattern) {
                return Ok(RiskLevel::Low);
            }
        }

        // Default to medium risk
        Ok(RiskLevel::Medium)
    }

    /// Check if a command requires approval
    pub fn requires_approval(&self, risk_level: RiskLevel) -> bool {
        matches!(risk_level, RiskLevel::High | RiskLevel::Medium)
    }
}

impl Default for SecurityValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_high_risk_command() {
        let validator = SecurityValidator::new();
        let result = validator.validate_command("rm -rf /");
        assert!(result.is_err());
    }

    #[test]
    fn test_low_risk_command() {
        let validator = SecurityValidator::new();
        let result = validator.validate_command("systemctl restart nginx");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RiskLevel::Low);
    }

    #[test]
    fn test_medium_risk_command() {
        let validator = SecurityValidator::new();
        let result = validator.validate_command("kubectl delete pod test-pod");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RiskLevel::Medium);
    }
}

// Made with Bob
