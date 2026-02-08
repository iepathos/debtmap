//! Rust analyzer implementation
//!
//! The RustAnalyzer implements the Analyzer trait for Rust code.

use crate::analyzers::Analyzer;
use crate::complexity::threshold_manager::{ComplexityThresholds, ThresholdPreset};
use crate::core::{
    ast::{Ast, RustAst},
    ComplexityMetrics, FileMetrics, Language,
};
use anyhow::Result;
use std::path::PathBuf;
use tracing::{debug, debug_span};

use super::orchestration::analyze_rust_file;

/// Analyzer for Rust source code
pub struct RustAnalyzer {
    pub(crate) complexity_threshold: u32,
    pub(crate) enhanced_thresholds: ComplexityThresholds,
    pub(crate) use_enhanced_detection: bool,
    pub(crate) enable_functional_analysis: bool,
    pub(crate) enable_rust_patterns: bool,
}

impl RustAnalyzer {
    pub fn new() -> Self {
        Self {
            complexity_threshold: 10,
            enhanced_thresholds: ComplexityThresholds::from_preset(ThresholdPreset::Balanced),
            use_enhanced_detection: true,
            enable_functional_analysis: false,
            enable_rust_patterns: false,
        }
    }

    pub fn with_threshold_preset(preset: ThresholdPreset) -> Self {
        Self {
            complexity_threshold: 10,
            enhanced_thresholds: ComplexityThresholds::from_preset(preset),
            use_enhanced_detection: true,
            enable_functional_analysis: false,
            enable_rust_patterns: false,
        }
    }

    pub fn with_functional_analysis(mut self, enable: bool) -> Self {
        self.enable_functional_analysis = enable;
        self
    }

    pub fn with_rust_patterns(mut self, enable: bool) -> Self {
        self.enable_rust_patterns = enable;
        self
    }
}

impl Default for RustAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for RustAnalyzer {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast> {
        let _span = debug_span!("parse_file", path = %path.display()).entered();

        let start = std::time::Instant::now();
        let file = syn::parse_str::<syn::File>(content)?;
        let parse_time = start.elapsed();

        debug!(
            path = %path.display(),
            time_ms = parse_time.as_millis(),
            bytes = content.len(),
            "Parsed file"
        );

        Ok(Ast::Rust(RustAst {
            file,
            path,
            source: content.to_string(),
        }))
    }

    fn analyze(&self, ast: &Ast) -> FileMetrics {
        match ast {
            Ast::Rust(rust_ast) => {
                let _span = debug_span!("analyze_file", path = %rust_ast.path.display()).entered();
                let result = analyze_rust_file(
                    rust_ast,
                    self.complexity_threshold,
                    &self.enhanced_thresholds,
                    self.use_enhanced_detection,
                    self.enable_functional_analysis,
                    self.enable_rust_patterns,
                );
                debug!(
                    functions = result.complexity.functions.len(),
                    debt_items = result.debt_items.len(),
                    "File analysis complete"
                );
                result
            }
            _ => FileMetrics {
                path: PathBuf::new(),
                language: Language::Rust,
                complexity: ComplexityMetrics::default(),
                debt_items: vec![],
                dependencies: vec![],
                duplications: vec![],
                total_lines: 0,
                module_scope: None,
                classes: None,
            },
        }
    }

    fn language(&self) -> Language {
        Language::Rust
    }
}
