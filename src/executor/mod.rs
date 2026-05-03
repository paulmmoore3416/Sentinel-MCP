//! Remediation executor module
//! 
//! This module handles the execution of remediation commands and
//! generates documentation of the remediation process.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub mod documentation;

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// Remediation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationReport {
    pub id: String,
    pub timestamp: String,
    pub alert_name: String,
    pub root_cause: String,
    pub steps_executed: Vec<String>,
    pub success: bool,
}

impl RemediationReport {
    /// Save report to file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load report from file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let json = fs::read_to_string(path)?;
        let report = serde_json::from_str(&json)?;
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_save_and_load_report() {
        let report = RemediationReport {
            id: "test-123".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            alert_name: "DiskSpaceLow".to_string(),
            root_cause: "Log files filling disk".to_string(),
            steps_executed: vec!["Rotated logs".to_string()],
            success: true,
        };

        let mut temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Save
        report.save_to_file(path).unwrap();

        // Load
        let loaded = RemediationReport::load_from_file(path).unwrap();
        assert_eq!(loaded.id, report.id);
        assert_eq!(loaded.alert_name, report.alert_name);
    }
}

// Made with Bob
