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
            .args([
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
    /// Check if a rule_id is security-relevant
    fn is_security_relevant(rule_id: &str) -> bool {
        rule_id.contains("unwrap") || rule_id.contains("panic") || rule_id.contains("expect")
    }

    /// Extract finding from clippy message
    fn extract_finding(
        message: &serde_json::Map<String, serde_json::Value>,
    ) -> Option<ToolFinding> {
        let code = message.get("code")?.as_object()?;
        let rule_id = code.get("code")?.as_str()?.to_string();

        if !Self::is_security_relevant(&rule_id) {
            return None;
        }

        let spans = message.get("spans")?.as_array()?;
        let primary_span = spans.iter().find(|s| s["is_primary"] == true)?;

        Some(ToolFinding {
            tool: "clippy".to_string(),
            severity: message
                .get("level")
                .and_then(|v| v.as_str())
                .unwrap_or("warning")
                .to_string(),
            rule_id,
            file: PathBuf::from(
                primary_span
                    .get("file_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
            ),
            line: primary_span
                .get("line_start")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            column: primary_span
                .get("column_start")
                .and_then(|v| v.as_u64())
                .map(|c| c as usize),
            message: message
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            remediation: message
                .get("children")
                .and_then(|v| v.as_array())
                .and_then(|children| children.first())
                .and_then(|child| child.get("message"))
                .and_then(|v| v.as_str())
                .map(String::from),
        })
    }

    fn parse_clippy_output(&self, output: &[u8]) -> Result<Vec<ToolFinding>> {
        let mut findings = Vec::new();
        let output_str = String::from_utf8_lossy(output);

        for line in output_str.lines() {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
                if msg["reason"] == "compiler-message" {
                    if let Some(message) = msg["message"].as_object() {
                        if let Some(finding) = Self::extract_finding(message) {
                            findings.push(finding);
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
            .args(["-r", path.to_str().unwrap(), "-f", "json"])
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

                if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(key) {
                    e.insert(true);
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

    #[test]
    fn test_is_security_relevant() {
        assert!(ClippyAdapter::is_security_relevant("clippy::unwrap_used"));
        assert!(ClippyAdapter::is_security_relevant("clippy::panic"));
        assert!(ClippyAdapter::is_security_relevant("clippy::expect_used"));
        assert!(ClippyAdapter::is_security_relevant("unwrap"));
        assert!(!ClippyAdapter::is_security_relevant(
            "clippy::needless_return"
        ));
        assert!(!ClippyAdapter::is_security_relevant("unused_variables"));
    }

    #[test]
    fn test_extract_finding_with_valid_message() {
        let mut message = serde_json::Map::new();
        message.insert(
            "code".to_string(),
            serde_json::json!({
                "code": "clippy::unwrap_used"
            }),
        );
        message.insert("level".to_string(), serde_json::json!("warning"));
        message.insert("message".to_string(), serde_json::json!("use of unwrap"));
        message.insert(
            "spans".to_string(),
            serde_json::json!([
                {
                    "is_primary": true,
                    "file_name": "src/main.rs",
                    "line_start": 42,
                    "column_start": 10
                }
            ]),
        );
        message.insert(
            "children".to_string(),
            serde_json::json!([
                {
                    "message": "consider using expect() instead"
                }
            ]),
        );

        let finding = ClippyAdapter::extract_finding(&message);
        assert!(finding.is_some());

        let finding = finding.unwrap();
        assert_eq!(finding.tool, "clippy");
        assert_eq!(finding.rule_id, "clippy::unwrap_used");
        assert_eq!(finding.severity, "warning");
        assert_eq!(finding.line, 42);
        assert_eq!(finding.column, Some(10));
        assert_eq!(finding.message, "use of unwrap");
        assert_eq!(
            finding.remediation,
            Some("consider using expect() instead".to_string())
        );
    }

    #[test]
    fn test_extract_finding_with_non_security_rule() {
        let mut message = serde_json::Map::new();
        message.insert(
            "code".to_string(),
            serde_json::json!({
                "code": "clippy::needless_return"
            }),
        );
        message.insert(
            "spans".to_string(),
            serde_json::json!([
                {
                    "is_primary": true,
                    "file_name": "src/main.rs",
                    "line_start": 10,
                    "column_start": 5
                }
            ]),
        );

        let finding = ClippyAdapter::extract_finding(&message);
        assert!(finding.is_none());
    }

    #[test]
    fn test_extract_finding_with_missing_spans() {
        let mut message = serde_json::Map::new();
        message.insert(
            "code".to_string(),
            serde_json::json!({
                "code": "clippy::unwrap_used"
            }),
        );

        let finding = ClippyAdapter::extract_finding(&message);
        assert!(finding.is_none());
    }

    #[test]
    fn test_extract_finding_with_no_primary_span() {
        let mut message = serde_json::Map::new();
        message.insert(
            "code".to_string(),
            serde_json::json!({
                "code": "clippy::panic"
            }),
        );
        message.insert(
            "spans".to_string(),
            serde_json::json!([
                {
                    "is_primary": false,
                    "file_name": "src/main.rs",
                    "line_start": 10
                }
            ]),
        );

        let finding = ClippyAdapter::extract_finding(&message);
        assert!(finding.is_none());
    }

    #[test]
    fn test_parse_clippy_output() {
        let adapter = ClippyAdapter;

        let output = r#"{"reason":"compiler-message","message":{"code":{"code":"clippy::unwrap_used"},"level":"warning","message":"use of unwrap","spans":[{"is_primary":true,"file_name":"src/main.rs","line_start":10,"column_start":5}],"children":[{"message":"try using expect"}]}}
{"reason":"compiler-message","message":{"code":{"code":"unused_variable"},"level":"warning","message":"unused variable","spans":[{"is_primary":true,"file_name":"src/lib.rs","line_start":20}]}}
{"reason":"compiler-message","message":{"code":{"code":"clippy::panic"},"level":"error","message":"panic in code","spans":[{"is_primary":true,"file_name":"src/test.rs","line_start":30,"column_start":15}]}}"#;

        let findings = adapter.parse_clippy_output(output.as_bytes()).unwrap();

        assert_eq!(findings.len(), 2);

        assert_eq!(findings[0].rule_id, "clippy::unwrap_used");
        assert_eq!(findings[0].line, 10);
        assert_eq!(findings[0].file, PathBuf::from("src/main.rs"));
        assert_eq!(
            findings[0].remediation,
            Some("try using expect".to_string())
        );

        assert_eq!(findings[1].rule_id, "clippy::panic");
        assert_eq!(findings[1].line, 30);
        assert_eq!(findings[1].severity, "error");
    }

    #[test]
    fn test_parse_clippy_output_with_invalid_json() {
        let adapter = ClippyAdapter;

        let output = b"not json\n{invalid json}\n";

        let findings = adapter.parse_clippy_output(output).unwrap();
        assert_eq!(findings.len(), 0);
    }

    #[test]
    fn test_parse_clippy_output_with_empty_output() {
        let adapter = ClippyAdapter;

        let findings = adapter.parse_clippy_output(b"").unwrap();
        assert_eq!(findings.len(), 0);
    }
}
