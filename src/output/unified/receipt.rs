//! Provenance for the versioned JSON analysis envelope.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisReceipt {
    pub analysis_target: Option<PathBuf>,
    pub source_revision: Option<SourceRevisionReceipt>,
    pub reference_time: Option<String>,
    pub policy: AnalysisPolicyReceipt,
    pub policy_fingerprint: String,
    pub evidence: EvidenceReceipt,
    pub selection: SelectionReceipt,
    pub execution: ExecutionReceipt,
    pub scope: ScopeReceipt,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceRevisionReceipt {
    pub commit: String,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisPolicyReceipt {
    pub languages: Vec<String>,
    pub language_policies: Vec<LanguagePolicyReceipt>,
    pub complexity_threshold: u32,
    pub duplication_threshold_lines: usize,
    pub duplication_similarity: f64,
    pub threshold_preset: Option<String>,
    pub semantic_analysis: bool,
    pub context_aware_scoring: bool,
    pub god_object_detection: bool,
    pub functional_analysis: bool,
    pub functional_analysis_profile: Option<String>,
    pub aggregation: bool,
    pub aggregation_method: Option<String>,
    pub minimum_problematic_functions: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LanguagePolicyReceipt {
    pub language: String,
    pub enabled: bool,
    pub detect_complexity: bool,
    pub detect_dead_code: bool,
    pub detect_duplication: bool,
    pub generated_code: String,
}

impl AnalysisPolicyReceipt {
    pub fn fingerprint(&self) -> Result<String, serde_json::Error> {
        let bytes = serde_json::to_vec(self)?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceReceipt {
    pub coverage_requested: bool,
    pub coverage_loaded: bool,
    pub coverage_source_kind: Option<String>,
    pub context_requested: bool,
    pub context_providers_requested: Option<Vec<String>>,
    pub context_providers_disabled: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectionReceipt {
    pub minimum_score_requested: Option<f64>,
    pub minimum_priority_requested: Option<String>,
    pub categories_requested: Vec<String>,
    pub aggregate_only: bool,
    pub top: Option<usize>,
    pub tail: Option<usize>,
    pub file_limit_requested: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionReceipt {
    pub parallel: bool,
    pub jobs: usize,
    pub multi_pass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScopeReceipt {
    pub discovered_files: Option<usize>,
    pub analyzed_files: usize,
    pub failed_files: Option<usize>,
    pub omitted_by_limit: Option<usize>,
    pub total_loc: usize,
    pub status: ScopeStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScopeStatus {
    Complete,
    Partial,
    Limited,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(languages: Vec<String>) -> AnalysisPolicyReceipt {
        AnalysisPolicyReceipt {
            languages,
            language_policies: Vec::new(),
            complexity_threshold: 10,
            duplication_threshold_lines: 50,
            duplication_similarity: 1.0,
            threshold_preset: None,
            semantic_analysis: true,
            context_aware_scoring: true,
            god_object_detection: true,
            functional_analysis: false,
            functional_analysis_profile: None,
            aggregation: true,
            aggregation_method: Some("weighted_sum".to_string()),
            minimum_problematic_functions: None,
        }
    }

    #[test]
    fn policy_fingerprint_is_deterministic_and_sensitive() {
        let first = policy(vec!["rust".to_string()]);
        let same = policy(vec!["rust".to_string()]);
        let changed = policy(vec!["python".to_string()]);

        assert_eq!(first.fingerprint().unwrap(), same.fingerprint().unwrap());
        assert_ne!(first.fingerprint().unwrap(), changed.fingerprint().unwrap());
    }
}
