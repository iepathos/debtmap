use crate::security::types::{SecurityVulnerability, Severity};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFinding {
    pub tool: String,
    pub severity: String,
    pub rule_id: String,
    pub file: PathBuf,
    pub line: usize,
    pub column: Option<usize>,
    pub message: String,
    pub remediation: Option<String>,
}

pub trait SecurityToolAdapter: Send + Sync {
    fn run_tool(&self, path: &Path) -> Result<Vec<ToolFinding>>;
    fn tool_name(&self) -> &str;
    fn supported_languages(&self) -> Vec<&str>;
}

pub struct ClippyAdapter;

impl SecurityToolAdapter for ClippyAdapter {
    fn run_tool(&self, path: &Path) -> Result<Vec<ToolFinding>> {
        let output = Command::new("cargo")
            .args(&[
                "clippy",
                "--message-format=json",
                "--",
                "-W",
                "clippy::unwrap_used",
                "-W",
                "clippy::expect_used",
                "-W",
                "clippy::panic",
                "-W",
                "clippy::todo",
                "-W",
                "clippy::unimplemented",
            ])
            .current_dir(path)
            .output()
            .context("Failed to run cargo clippy")?;

        self.parse_clippy_output(&output.stdout)
    }

    fn tool_name(&self) -> &str {
        "clippy"
    }

    fn supported_languages(&self) -> Vec<&str> {
        vec!["rust"]
    }
}

impl ClippyAdapter {
    fn parse_clippy_output(&self, output: &[u8]) -> Result<Vec<ToolFinding>> {
        let mut findings = Vec::new();
        let output_str = String::from_utf8_lossy(output);

        for line in output_str.lines() {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
                if msg["reason"] == "compiler-message" {
                    if let Some(message) = msg["message"].as_object() {
                        if let Some(code) = message["code"].as_object() {
                            let rule_id = code["code"].as_str().unwrap_or("unknown").to_string();

                            // Only process security-relevant lints
                            if !rule_id.contains("unwrap")
                                && !rule_id.contains("panic")
                                && !rule_id.contains("expect")
                            {
                                continue;
                            }

                            if let Some(spans) = message["spans"].as_array() {
                                if let Some(primary_span) =
                                    spans.iter().find(|s| s["is_primary"] == true)
                                {
                                    findings.push(ToolFinding {
                                        tool: "clippy".to_string(),
                                        severity: message["level"]
                                            .as_str()
                                            .unwrap_or("warning")
                                            .to_string(),
                                        rule_id,
                                        file: PathBuf::from(
                                            primary_span["file_name"].as_str().unwrap_or(""),
                                        ),
                                        line: primary_span["line_start"].as_u64().unwrap_or(0)
                                            as usize,
                                        column: primary_span["column_start"]
                                            .as_u64()
                                            .map(|c| c as usize),
                                        message: message["message"]
                                            .as_str()
                                            .unwrap_or("")
                                            .to_string(),
                                        remediation: message["children"]
                                            .as_array()
                                            .and_then(|children| children.first())
                                            .and_then(|child| child["message"].as_str())
                                            .map(String::from),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(findings)
    }
}

pub struct BanditAdapter;

impl SecurityToolAdapter for BanditAdapter {
    fn run_tool(&self, path: &Path) -> Result<Vec<ToolFinding>> {
        let output = Command::new("bandit")
            .args(&["-r", path.to_str().unwrap(), "-f", "json"])
            .output()
            .context("Failed to run bandit")?;

        self.parse_bandit_output(&output.stdout)
    }

    fn tool_name(&self) -> &str {
        "bandit"
    }

    fn supported_languages(&self) -> Vec<&str> {
        vec!["python"]
    }
}

impl BanditAdapter {
    fn parse_bandit_output(&self, output: &[u8]) -> Result<Vec<ToolFinding>> {
        let json: serde_json::Value =
            serde_json::from_slice(output).context("Failed to parse bandit output")?;

        let mut findings = Vec::new();

        if let Some(results) = json["results"].as_array() {
            for result in results {
                findings.push(ToolFinding {
                    tool: "bandit".to_string(),
                    severity: result["issue_severity"]
                        .as_str()
                        .unwrap_or("MEDIUM")
                        .to_string(),
                    rule_id: result["test_id"].as_str().unwrap_or("").to_string(),
                    file: PathBuf::from(result["filename"].as_str().unwrap_or("")),
                    line: result["line_number"].as_u64().unwrap_or(0) as usize,
                    column: result["col_offset"].as_u64().map(|c| c as usize),
                    message: result["issue_text"].as_str().unwrap_or("").to_string(),
                    remediation: result["more_info"].as_str().map(String::from),
                });
            }
        }

        Ok(findings)
    }
}

#[derive(Default)]
pub struct ToolIntegrationManager {
    adapters: Vec<Box<dyn SecurityToolAdapter>>,
    cache: HashMap<PathBuf, CachedFindings>,
}

#[derive(Clone)]
struct CachedFindings {
    findings: Vec<SecurityVulnerability>,
    timestamp: std::time::SystemTime,
}

impl ToolIntegrationManager {
    pub fn new() -> Self {
        Self {
            adapters: vec![Box::new(ClippyAdapter), Box::new(BanditAdapter)],
            cache: HashMap::new(),
        }
    }

    pub fn add_adapter(&mut self, adapter: Box<dyn SecurityToolAdapter>) {
        self.adapters.push(adapter);
    }

    pub fn run_external_tools(
        &mut self,
        path: &Path,
        language: &str,
    ) -> Result<Vec<SecurityVulnerability>> {
        // Check cache first
        if let Some(cached) = self.cache.get(path) {
            if let Ok(elapsed) = cached.timestamp.elapsed() {
                if elapsed.as_secs() < 3600 {
                    // Cache is still valid (1 hour)
                    return Ok(cached.findings.clone());
                }
            }
        }

        let mut all_findings = Vec::new();

        for adapter in &self.adapters {
            if adapter.supported_languages().contains(&language) {
                match adapter.run_tool(path) {
                    Ok(findings) => {
                        all_findings.extend(self.normalize_findings(findings));
                    }
                    Err(e) => {
                        log::warn!("Failed to run {}: {}", adapter.tool_name(), e);
                    }
                }
            }
        }

        let deduplicated = self.deduplicate_findings(all_findings);

        // Update cache
        self.cache.insert(
            path.to_path_buf(),
            CachedFindings {
                findings: deduplicated.clone(),
                timestamp: std::time::SystemTime::now(),
            },
        );

        Ok(deduplicated)
    }

    fn normalize_findings(&self, findings: Vec<ToolFinding>) -> Vec<SecurityVulnerability> {
        findings
            .into_iter()
            .map(|finding| SecurityVulnerability::ExternalToolFinding {
                tool: finding.tool,
                original_severity: finding.severity.clone(),
                normalized_severity: self.normalize_severity(&finding.severity),
                description: finding.message,
                remediation: finding.remediation,
                line: finding.line,
                file: finding.file,
            })
            .collect()
    }

    fn normalize_severity(&self, severity: &str) -> Severity {
        match severity.to_lowercase().as_str() {
            "critical" | "high" | "error" => Severity::High,
            "medium" | "moderate" | "warning" => Severity::Medium,
            "low" | "info" | "note" => Severity::Low,
            _ => Severity::Low,
        }
    }

    fn deduplicate_findings(
        &self,
        findings: Vec<SecurityVulnerability>,
    ) -> Vec<SecurityVulnerability> {
        let mut seen = HashMap::new();
        let mut deduplicated = Vec::new();

        for finding in findings {
            if let SecurityVulnerability::ExternalToolFinding {
                ref file,
                line,
                ref description,
                ..
            } = finding
            {
                let key = format!("{:?}:{}:{}", file, line, description);

                if !seen.contains_key(&key) {
                    seen.insert(key, true);
                    deduplicated.push(finding);
                }
            } else {
                deduplicated.push(finding);
            }
        }

        deduplicated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_normalization() {
        let manager = ToolIntegrationManager::new();

        assert_eq!(manager.normalize_severity("CRITICAL"), Severity::High);
        assert_eq!(manager.normalize_severity("error"), Severity::High);
        assert_eq!(manager.normalize_severity("warning"), Severity::Medium);
        assert_eq!(manager.normalize_severity("info"), Severity::Low);
        assert_eq!(manager.normalize_severity("unknown"), Severity::Low);
    }

    #[test]
    fn test_deduplication() {
        let manager = ToolIntegrationManager::new();

        let findings = vec![
            SecurityVulnerability::ExternalToolFinding {
                tool: "clippy".to_string(),
                original_severity: "warning".to_string(),
                normalized_severity: Severity::Medium,
                description: "unwrap used".to_string(),
                remediation: None,
                line: 10,
                file: PathBuf::from("test.rs"),
            },
            SecurityVulnerability::ExternalToolFinding {
                tool: "clippy".to_string(),
                original_severity: "warning".to_string(),
                normalized_severity: Severity::Medium,
                description: "unwrap used".to_string(),
                remediation: None,
                line: 10,
                file: PathBuf::from("test.rs"),
            },
        ];

        let deduplicated = manager.deduplicate_findings(findings);
        assert_eq!(deduplicated.len(), 1);
    }
}
