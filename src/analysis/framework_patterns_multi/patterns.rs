//! Framework Pattern Types and Language Support

use serde::{Deserialize, Serialize};

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    Rust,
    Python,
}

impl Language {
    /// Parse language from string (custom implementation, not FromStr trait)
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        match s.to_lowercase().as_str() {
            "rust" => Ok(Language::Rust),
            "python" => Ok(Language::Python),
            _ => Err(anyhow::anyhow!("Unknown language: {}", s)),
        }
    }

    /// Get canonical name
    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
        }
    }
}

/// Pattern matcher type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PatternMatcher {
    /// Match import statements
    Import { pattern: String },
    /// Match decorators (Python/TypeScript)
    Decorator { pattern: String },
    /// Match attributes (Rust)
    Attribute { pattern: String },
    /// Match derive macros (Rust)
    Derive { pattern: String },
    /// Match function parameters
    Parameter { pattern: String },
    /// Match return type
    ReturnType { pattern: String },
    /// Match function name
    Name { pattern: String },
    /// Match function calls in body
    Call { pattern: String },
    /// Match file path
    FilePath { pattern: String },
}

/// Framework pattern definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkPattern {
    /// Framework name
    pub name: String,
    /// Responsibility category
    pub category: String,
    /// Pattern matchers
    pub patterns: Vec<PatternMatcher>,
}

/// Framework detection result
#[derive(Debug, Clone)]
pub struct FrameworkMatch {
    /// Framework name
    pub framework: String,
    /// Responsibility category
    pub category: String,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
    /// Evidence for the match
    pub evidence: Vec<String>,
}

impl FrameworkMatch {
    /// Create a new framework match
    pub fn new(framework: String, category: String, confidence: f64) -> Self {
        Self {
            framework,
            category,
            confidence,
            evidence: Vec::new(),
        }
    }

    /// Add evidence to the match
    pub fn with_evidence(mut self, evidence: String) -> Self {
        self.evidence.push(evidence);
        self
    }
}
