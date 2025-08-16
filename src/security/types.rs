use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SecurityVulnerability {
    HardcodedSecret {
        secret_type: SecretType,
        confidence: f64,
        value_preview: String,
        entropy: f64,
        line: usize,
        file: PathBuf,
    },
    SqlInjection {
        injection_type: SqlInjectionType,
        taint_source: Option<TaintSource>,
        confidence: f64,
        severity: Severity,
        line: usize,
        file: PathBuf,
    },
    InputValidationGap {
        input_source: InputSource,
        sink_operation: SinkOperation,
        taint_path: Vec<String>,
        severity: Severity,
        line: usize,
        file: PathBuf,
    },
    ExternalToolFinding {
        tool: String,
        original_severity: String,
        normalized_severity: Severity,
        description: String,
        remediation: Option<String>,
        line: usize,
        file: PathBuf,
    },
    UnsafeUsage {
        description: String,
        severity: Severity,
        line: usize,
        file: PathBuf,
    },
    CryptoMisuse {
        issue_type: CryptoIssueType,
        description: String,
        severity: Severity,
        line: usize,
        file: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, Serialize, Deserialize)]
pub enum SecretType {
    ApiKey,
    Password,
    PrivateKey,
    DatabaseCredential,
    AuthToken,
    JwtSecret,
    WebhookSecret,
    EncryptionKey,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize)]
pub enum SqlInjectionType {
    StringConcatenation,
    FormatString,
    DynamicQuery,
    TemplateInjection,
}

#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize)]
pub enum TaintSource {
    HttpRequest,
    CliArgument,
    Environment,
    FileInput,
    DatabaseQuery,
    UserControlled,
}

#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize)]
pub enum InputSource {
    HttpRequest,
    CliArgument,
    Environment,
    FileInput,
    ExternalApi,
    UserInput,
}

#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize)]
pub enum SinkOperation {
    SqlQuery,
    FileSystem,
    ProcessExecution,
    NetworkRequest,
    Deserialization,
    CryptoOperation,
}

#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize)]
pub enum CryptoIssueType {
    WeakAlgorithm,
    InsecureRandom,
    HardcodedSalt,
    ShortKeyLength,
    MissingAuthentication,
}

#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize, Ord, PartialOrd, Eq)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn to_priority(&self) -> crate::core::Priority {
        match self {
            Severity::Critical => crate::core::Priority::Critical,
            Severity::High => crate::core::Priority::High,
            Severity::Medium => crate::core::Priority::Medium,
            Severity::Low => crate::core::Priority::Low,
        }
    }
}

pub trait SecurityDetector {
    fn detect_vulnerabilities(
        &self,
        file: &syn::File,
        path: &std::path::Path,
    ) -> Vec<SecurityVulnerability>;
    fn detector_name(&self) -> &'static str;
}
