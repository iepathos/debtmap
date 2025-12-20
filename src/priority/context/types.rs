//! Context suggestion data types (Spec 263).
//!
//! Defines types for suggesting code context that AI agents should read
//! to understand and fix debt items.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Suggested code context for an AI agent to read when fixing a debt item.
///
/// This provides explicit guidance on what files and line ranges the AI should read
/// to fully understand the debt before attempting a fix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSuggestion {
    /// Primary code to read - the debt item itself
    pub primary: FileRange,

    /// Related code that provides necessary context
    pub related: Vec<RelatedContext>,

    /// Estimated total lines to read
    pub total_lines: u32,

    /// Confidence that this context is sufficient (0.0-1.0)
    pub completeness_confidence: f32,
}

/// A file range with start and end lines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRange {
    pub file: PathBuf,
    pub start_line: u32,
    pub end_line: u32,
    /// Optional: Function/struct name for clarity
    pub symbol: Option<String>,
}

impl FileRange {
    /// Calculate the number of lines in this range.
    pub fn line_count(&self) -> u32 {
        if self.end_line >= self.start_line {
            self.end_line - self.start_line + 1
        } else {
            0
        }
    }
}

/// Related context with relationship information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedContext {
    pub range: FileRange,
    pub relationship: ContextRelationship,
    /// Why this context is relevant
    pub reason: String,
}

/// Type of relationship between related code and the primary scope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContextRelationship {
    /// Functions that call this function
    Caller,
    /// Functions this function calls
    Callee,
    /// Type definitions used by this code
    TypeDefinition,
    /// Test code for this function/module
    TestCode,
    /// Sibling functions in same impl block
    SiblingMethod,
    /// Trait definition this implements
    TraitDefinition,
    /// Module-level context (imports, constants)
    ModuleHeader,
}

impl std::fmt::Display for ContextRelationship {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextRelationship::Caller => write!(f, "Caller"),
            ContextRelationship::Callee => write!(f, "Callee"),
            ContextRelationship::TypeDefinition => write!(f, "TypeDefinition"),
            ContextRelationship::TestCode => write!(f, "TestCode"),
            ContextRelationship::SiblingMethod => write!(f, "SiblingMethod"),
            ContextRelationship::TraitDefinition => write!(f, "TraitDefinition"),
            ContextRelationship::ModuleHeader => write!(f, "ModuleHeader"),
        }
    }
}
