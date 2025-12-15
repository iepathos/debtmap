//! Metrics adapter for converting extracted data to FunctionMetrics and FileMetrics.
//!
//! This module provides pure conversion functions that transform `ExtractedFileData`
//! into the existing metrics types used by the analysis pipeline.
//!
//! # Design
//!
//! All functions in this module are pure (no I/O, no parsing). They perform O(n)
//! conversions where n is the number of items being converted.

use crate::core::{ComplexityMetrics, FileMetrics, FunctionMetrics, Language};
use crate::extraction::types::{ExtractedFileData, ExtractedFunctionData, PurityLevel};
use std::path::Path;

/// Convert extracted function data to FunctionMetrics.
///
/// This is a pure conversion with no file I/O.
///
/// # Arguments
///
/// * `file_path` - Path to the source file
/// * `extracted` - Extracted function data from single-pass parsing
///
/// # Returns
///
/// A `FunctionMetrics` struct populated from the extracted data.
pub fn to_function_metrics(file_path: &Path, extracted: &ExtractedFunctionData) -> FunctionMetrics {
    FunctionMetrics {
        name: extracted.name.clone(),
        file: file_path.to_path_buf(),
        line: extracted.line,
        cyclomatic: extracted.cyclomatic,
        cognitive: extracted.cognitive,
        nesting: extracted.nesting,
        length: extracted.length,

        // Purity from extraction
        is_pure: Some(extracted.purity_analysis.is_pure),
        purity_confidence: Some(extracted.purity_analysis.confidence),
        purity_level: Some(convert_purity_level(extracted.purity_analysis.purity_level)),

        // Metadata
        is_test: extracted.is_test,
        visibility: extracted.visibility.clone(),
        is_trait_method: extracted.is_trait_method,
        in_test_module: extracted.in_test_module,

        // Fields populated by other phases
        purity_reason: None,
        entropy_score: None,
        call_dependencies: None,
        upstream_callers: None,
        downstream_callees: None,
        detected_patterns: None,
        adjusted_complexity: None,
        composition_metrics: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
        language_specific: None,
        mapping_pattern_result: None,
    }
}

/// Convert extraction PurityLevel to core PurityLevel.
fn convert_purity_level(level: PurityLevel) -> crate::core::PurityLevel {
    match level {
        PurityLevel::StrictlyPure => crate::core::PurityLevel::StrictlyPure,
        PurityLevel::LocallyPure => crate::core::PurityLevel::LocallyPure,
        PurityLevel::ReadOnly => crate::core::PurityLevel::ReadOnly,
        PurityLevel::Impure => crate::core::PurityLevel::Impure,
    }
}

/// Convert all functions in extracted file to FunctionMetrics.
///
/// # Arguments
///
/// * `extracted` - Extracted file data containing functions
///
/// # Returns
///
/// A vector of `FunctionMetrics` for all functions in the file.
pub fn all_function_metrics(extracted: &ExtractedFileData) -> Vec<FunctionMetrics> {
    extracted
        .functions
        .iter()
        .map(|f| to_function_metrics(&extracted.path, f))
        .collect()
}

/// Convert extracted file data to FileMetrics.
///
/// This is a pure conversion that aggregates function metrics into file-level metrics.
///
/// # Arguments
///
/// * `extracted` - Extracted file data from single-pass parsing
///
/// # Returns
///
/// A `FileMetrics` struct with aggregated complexity data.
pub fn to_file_metrics(extracted: &ExtractedFileData) -> FileMetrics {
    let functions = all_function_metrics(extracted);

    let total_cyclomatic: u32 = extracted.functions.iter().map(|f| f.cyclomatic).sum();
    let total_cognitive: u32 = extracted.functions.iter().map(|f| f.cognitive).sum();

    FileMetrics {
        path: extracted.path.clone(),
        language: Language::Rust,
        complexity: ComplexityMetrics {
            functions,
            cyclomatic_complexity: total_cyclomatic,
            cognitive_complexity: total_cognitive,
        },
        debt_items: vec![], // Populated by debt detection phase
        dependencies: vec![],
        duplications: vec![],
        total_lines: extracted.total_lines,
        module_scope: None,
        classes: None,
    }
}

/// Convert all extracted files to function metrics.
///
/// # Arguments
///
/// * `extracted` - Map of file paths to extracted file data
///
/// # Returns
///
/// A flat vector of all `FunctionMetrics` across all files.
pub fn all_metrics_from_extracted(
    extracted: &std::collections::HashMap<std::path::PathBuf, ExtractedFileData>,
) -> Vec<FunctionMetrics> {
    extracted.values().flat_map(all_function_metrics).collect()
}

/// Convert all extracted files to file metrics.
///
/// # Arguments
///
/// * `extracted` - Map of file paths to extracted file data
///
/// # Returns
///
/// A vector of `FileMetrics` for all extracted files.
pub fn all_file_metrics_from_extracted(
    extracted: &std::collections::HashMap<std::path::PathBuf, ExtractedFileData>,
) -> Vec<FileMetrics> {
    extracted.values().map(to_file_metrics).collect()
}

/// Calculate aggregate complexity metrics across all files.
///
/// # Arguments
///
/// * `extracted` - Map of file paths to extracted file data
///
/// # Returns
///
/// Tuple of (total_cyclomatic, total_cognitive, max_nesting, function_count)
pub fn aggregate_complexity(
    extracted: &std::collections::HashMap<std::path::PathBuf, ExtractedFileData>,
) -> (u32, u32, u32, usize) {
    let mut total_cyclomatic: u32 = 0;
    let mut total_cognitive: u32 = 0;
    let mut max_nesting: u32 = 0;
    let mut function_count: usize = 0;

    for file_data in extracted.values() {
        for func in &file_data.functions {
            total_cyclomatic += func.cyclomatic;
            total_cognitive += func.cognitive;
            max_nesting = max_nesting.max(func.nesting);
            function_count += 1;
        }
    }

    (
        total_cyclomatic,
        total_cognitive,
        max_nesting,
        function_count,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extraction::types::{ExtractedFileData, ExtractedFunctionData, PurityAnalysisData};
    use std::path::PathBuf;

    fn create_test_function(name: &str, line: usize, cyclomatic: u32) -> ExtractedFunctionData {
        ExtractedFunctionData {
            name: name.to_string(),
            qualified_name: name.to_string(),
            line,
            end_line: line + 10,
            length: 10,
            cyclomatic,
            cognitive: cyclomatic / 2,
            nesting: 2,
            purity_analysis: PurityAnalysisData::pure(),
            io_operations: vec![],
            parameter_names: vec![],
            transformation_patterns: vec![],
            calls: vec![],
            is_test: false,
            is_async: false,
            visibility: Some("pub".to_string()),
            is_trait_method: false,
            in_test_module: false,
        }
    }

    fn create_test_file_data() -> ExtractedFileData {
        ExtractedFileData {
            path: PathBuf::from("src/test.rs"),
            functions: vec![
                create_test_function("foo", 1, 5),
                create_test_function("bar", 20, 3),
            ],
            structs: vec![],
            impls: vec![],
            imports: vec![],
            total_lines: 50,
        }
    }

    #[test]
    fn test_to_function_metrics_basic() {
        let func = create_test_function("test_fn", 10, 5);
        let path = PathBuf::from("src/main.rs");
        let metrics = to_function_metrics(&path, &func);

        assert_eq!(metrics.name, "test_fn");
        assert_eq!(metrics.file, path);
        assert_eq!(metrics.line, 10);
        assert_eq!(metrics.cyclomatic, 5);
        assert_eq!(metrics.length, 10);
        assert!(metrics.is_pure.unwrap());
    }

    #[test]
    fn test_to_function_metrics_preserves_metadata() {
        let mut func = create_test_function("method", 1, 2);
        func.is_test = true;
        func.is_trait_method = true;
        func.in_test_module = true;
        func.visibility = Some("pub(crate)".to_string());

        let path = PathBuf::from("src/lib.rs");
        let metrics = to_function_metrics(&path, &func);

        assert!(metrics.is_test);
        assert!(metrics.is_trait_method);
        assert!(metrics.in_test_module);
        assert_eq!(metrics.visibility, Some("pub(crate)".to_string()));
    }

    #[test]
    fn test_all_function_metrics() {
        let file_data = create_test_file_data();
        let metrics = all_function_metrics(&file_data);

        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].name, "foo");
        assert_eq!(metrics[1].name, "bar");
    }

    #[test]
    fn test_to_file_metrics() {
        let file_data = create_test_file_data();
        let metrics = to_file_metrics(&file_data);

        assert_eq!(metrics.path, PathBuf::from("src/test.rs"));
        assert_eq!(metrics.language, Language::Rust);
        assert_eq!(metrics.complexity.functions.len(), 2);
        assert_eq!(metrics.complexity.cyclomatic_complexity, 8); // 5 + 3
        assert_eq!(metrics.total_lines, 50);
    }

    #[test]
    fn test_all_metrics_from_extracted() {
        let mut extracted = std::collections::HashMap::new();
        extracted.insert(PathBuf::from("file1.rs"), create_test_file_data());

        let mut file2 = create_test_file_data();
        file2.path = PathBuf::from("file2.rs");
        file2.functions = vec![create_test_function("baz", 1, 7)];
        extracted.insert(PathBuf::from("file2.rs"), file2);

        let all_metrics = all_metrics_from_extracted(&extracted);

        assert_eq!(all_metrics.len(), 3); // 2 from file1, 1 from file2
    }

    #[test]
    fn test_aggregate_complexity() {
        let mut extracted = std::collections::HashMap::new();
        extracted.insert(PathBuf::from("file1.rs"), create_test_file_data());

        let (total_cyc, total_cog, max_nest, count) = aggregate_complexity(&extracted);

        assert_eq!(total_cyc, 8); // 5 + 3
        assert_eq!(total_cog, 3); // (5/2) + (3/2) = 2 + 1 = 3 (integer division)
        assert_eq!(max_nest, 2);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_empty_file() {
        let file_data = ExtractedFileData::empty(PathBuf::from("empty.rs"));
        let metrics = to_file_metrics(&file_data);

        assert!(metrics.complexity.functions.is_empty());
        assert_eq!(metrics.complexity.cyclomatic_complexity, 0);
        assert_eq!(metrics.total_lines, 0);
    }

    #[test]
    fn test_purity_level_conversion() {
        assert_eq!(
            convert_purity_level(PurityLevel::StrictlyPure),
            crate::core::PurityLevel::StrictlyPure
        );
        assert_eq!(
            convert_purity_level(PurityLevel::LocallyPure),
            crate::core::PurityLevel::LocallyPure
        );
        assert_eq!(
            convert_purity_level(PurityLevel::ReadOnly),
            crate::core::PurityLevel::ReadOnly
        );
        assert_eq!(
            convert_purity_level(PurityLevel::Impure),
            crate::core::PurityLevel::Impure
        );
    }

    #[test]
    fn test_impure_function_metrics() {
        let mut func = create_test_function("impure_fn", 1, 10);
        func.purity_analysis = PurityAnalysisData::impure("writes to global");
        func.purity_analysis.purity_level = PurityLevel::Impure;
        func.purity_analysis.confidence = 0.9;

        let path = PathBuf::from("src/io.rs");
        let metrics = to_function_metrics(&path, &func);

        assert!(!metrics.is_pure.unwrap());
        assert_eq!(metrics.purity_confidence.unwrap(), 0.9);
        assert_eq!(
            metrics.purity_level.unwrap(),
            crate::core::PurityLevel::Impure
        );
    }
}
