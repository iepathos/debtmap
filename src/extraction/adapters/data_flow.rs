//! Data flow adapter for populating DataFlowGraph from extracted data.
//!
//! This module provides pure conversion functions that transform `ExtractedFileData`
//! into data flow analysis structures for the analysis pipeline.
//!
//! # Design
//!
//! All functions in this module are pure (no I/O, no parsing). They perform O(n)
//! population where n is the number of items being added.

use crate::data_flow::{DataFlowGraph, IoOperation as DFIoOperation, PurityInfo};
use crate::extraction::types::{
    ExtractedFileData, IoOperation, IoType, PatternType, TransformationPattern,
};
use crate::priority::call_graph::FunctionId;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Statistics about data flow population.
#[derive(Debug, Default, Clone)]
pub struct PopulationStats {
    pub purity_entries: usize,
    pub io_operations: usize,
    pub variable_deps: usize,
    pub transformations: usize,
}

/// Populate a DataFlowGraph from extracted file data.
///
/// This is a pure function with no file I/O.
///
/// # Arguments
///
/// * `graph` - The DataFlowGraph to populate
/// * `extracted` - Map of file paths to extracted file data
///
/// # Returns
///
/// Statistics about what was populated.
pub fn populate_data_flow(
    graph: &mut DataFlowGraph,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> PopulationStats {
    let mut stats = PopulationStats::default();

    for (path, file_data) in extracted {
        for func in &file_data.functions {
            let func_id = FunctionId::new(path.clone(), func.name.clone(), func.line);

            // Purity info
            stats.purity_entries += populate_purity(graph, &func_id, &func.purity_analysis);

            // I/O operations
            stats.io_operations += populate_io(graph, &func_id, &func.io_operations);

            // Variable dependencies (from parameters)
            stats.variable_deps += populate_deps(graph, &func_id, &func.parameter_names);

            // Transformation patterns
            stats.transformations +=
                populate_transformations(graph, &func_id, &func.transformation_patterns);
        }
    }

    stats
}

/// Populate purity information for a function.
fn populate_purity(
    graph: &mut DataFlowGraph,
    func_id: &FunctionId,
    purity: &crate::extraction::types::PurityAnalysisData,
) -> usize {
    let mut impurity_reasons = Vec::new();

    if !purity.is_pure {
        if purity.has_mutations {
            impurity_reasons.push("Has mutations".to_string());
        }
        if purity.has_io_operations {
            impurity_reasons.push("Has I/O operations".to_string());
        }
        if purity.has_unsafe {
            impurity_reasons.push("Contains unsafe code".to_string());
        }
    }

    let purity_info = PurityInfo {
        is_pure: purity.is_pure,
        confidence: purity.confidence,
        impurity_reasons,
    };

    graph.set_purity_info(func_id.clone(), purity_info);

    1
}

/// Populate I/O operations for a function.
fn populate_io(graph: &mut DataFlowGraph, func_id: &FunctionId, io_ops: &[IoOperation]) -> usize {
    for op in io_ops {
        let df_op = convert_io_operation(op);
        graph.add_io_operation(func_id.clone(), df_op);
    }
    io_ops.len()
}

/// Convert extracted IoOperation to DataFlowGraph IoOperation.
fn convert_io_operation(op: &IoOperation) -> DFIoOperation {
    DFIoOperation {
        operation_type: format_io_type(op.io_type),
        variables: vec![op.description.clone()],
        line: op.line,
    }
}

/// Format IoType as a string.
fn format_io_type(io_type: IoType) -> String {
    match io_type {
        IoType::File => "file".to_string(),
        IoType::Console => "console".to_string(),
        IoType::Network => "network".to_string(),
        IoType::Database => "database".to_string(),
        IoType::AsyncIO => "async_io".to_string(),
        IoType::Environment => "environment".to_string(),
        IoType::System => "system".to_string(),
    }
}

/// Populate variable dependencies for a function.
fn populate_deps(graph: &mut DataFlowGraph, func_id: &FunctionId, params: &[String]) -> usize {
    if params.is_empty() {
        return 0;
    }

    let deps: HashSet<String> = params.iter().cloned().collect();
    graph.add_variable_dependencies(func_id.clone(), deps);
    params.len()
}

/// Populate transformation patterns for a function.
fn populate_transformations(
    graph: &mut DataFlowGraph,
    func_id: &FunctionId,
    patterns: &[TransformationPattern],
) -> usize {
    for pattern in patterns {
        let pattern_name = pattern_type_to_string(pattern.pattern_type);

        // Add as a data transformation to self (indicating the pattern is used)
        // This is a simplified approach - full data flow would track actual transformations
        let transformation = crate::data_flow::DataTransformation {
            input_vars: vec![],
            output_vars: vec![],
            transformation_type: pattern_name,
        };
        graph.add_data_transformation(func_id.clone(), func_id.clone(), transformation);
    }
    patterns.len()
}

/// Convert PatternType to string representation.
fn pattern_type_to_string(pattern_type: PatternType) -> String {
    match pattern_type {
        PatternType::Map => "map".to_string(),
        PatternType::Filter => "filter".to_string(),
        PatternType::Fold => "fold".to_string(),
        PatternType::FlatMap => "flat_map".to_string(),
        PatternType::Collect => "collect".to_string(),
        PatternType::ForEach => "for_each".to_string(),
        PatternType::Find => "find".to_string(),
        PatternType::Any => "any".to_string(),
        PatternType::All => "all".to_string(),
        PatternType::Reduce => "reduce".to_string(),
    }
}

/// Populate data flow from a single file.
///
/// Convenience function for single-file analysis.
pub fn populate_from_single_file(
    graph: &mut DataFlowGraph,
    file_data: &ExtractedFileData,
) -> PopulationStats {
    let mut extracted = HashMap::new();
    extracted.insert(file_data.path.clone(), file_data.clone());
    populate_data_flow(graph, &extracted)
}

/// Get summary of I/O operations across all extracted files.
pub fn summarize_io_operations(
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> HashMap<String, usize> {
    let mut summary: HashMap<String, usize> = HashMap::new();

    for file_data in extracted.values() {
        for func in &file_data.functions {
            for op in &func.io_operations {
                let key = format_io_type(op.io_type);
                *summary.entry(key).or_insert(0) += 1;
            }
        }
    }

    summary
}

/// Count pure vs impure functions.
pub fn count_purity(extracted: &HashMap<PathBuf, ExtractedFileData>) -> (usize, usize) {
    let mut pure = 0;
    let mut impure = 0;

    for file_data in extracted.values() {
        for func in &file_data.functions {
            if func.purity_analysis.is_pure {
                pure += 1;
            } else {
                impure += 1;
            }
        }
    }

    (pure, impure)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extraction::types::{
        ExtractedFileData, ExtractedFunctionData, IoOperation, IoType, PatternType,
        PurityAnalysisData, TransformationPattern,
    };

    fn create_test_function(name: &str, line: usize) -> ExtractedFunctionData {
        ExtractedFunctionData {
            name: name.to_string(),
            qualified_name: name.to_string(),
            line,
            end_line: line + 10,
            length: 10,
            cyclomatic: 5,
            cognitive: 3,
            nesting: 2,
            purity_analysis: PurityAnalysisData::pure(),
            io_operations: vec![],
            parameter_names: vec![],
            transformation_patterns: vec![],
            calls: vec![],
            is_test: false,
            is_async: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
        }
    }

    fn create_test_file() -> ExtractedFileData {
        ExtractedFileData {
            path: PathBuf::from("src/main.rs"),
            functions: vec![create_test_function("foo", 1)],
            structs: vec![],
            impls: vec![],
            imports: vec![],
            total_lines: 50,
        }
    }

    #[test]
    fn test_populate_purity_pure_function() {
        let mut graph = DataFlowGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "pure_fn".to_string(), 1);
        let purity = PurityAnalysisData::pure();

        let count = populate_purity(&mut graph, &func_id, &purity);

        assert_eq!(count, 1);
        let info = graph.get_purity_info(&func_id).unwrap();
        assert!(info.is_pure);
        assert!(info.impurity_reasons.is_empty());
    }

    #[test]
    fn test_populate_purity_impure_function() {
        let mut graph = DataFlowGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "impure_fn".to_string(), 1);
        let mut purity = PurityAnalysisData::impure("test");
        purity.has_io_operations = true;
        purity.has_unsafe = true;

        populate_purity(&mut graph, &func_id, &purity);

        let info = graph.get_purity_info(&func_id).unwrap();
        assert!(!info.is_pure);
        assert_eq!(info.impurity_reasons.len(), 3); // mutations, io, unsafe
    }

    #[test]
    fn test_populate_io_operations() {
        let mut graph = DataFlowGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "io_fn".to_string(), 1);
        let io_ops = vec![
            IoOperation {
                io_type: IoType::File,
                description: "read config".to_string(),
                line: 5,
            },
            IoOperation {
                io_type: IoType::Console,
                description: "print output".to_string(),
                line: 10,
            },
        ];

        let count = populate_io(&mut graph, &func_id, &io_ops);

        assert_eq!(count, 2);
        let ops = graph.get_io_operations(&func_id).unwrap();
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].operation_type, "file");
        assert_eq!(ops[1].operation_type, "console");
    }

    #[test]
    fn test_populate_variable_deps() {
        let mut graph = DataFlowGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "fn_with_params".to_string(), 1);
        let params = vec!["self".to_string(), "x".to_string(), "y".to_string()];

        let count = populate_deps(&mut graph, &func_id, &params);

        assert_eq!(count, 3);
        let deps = graph.get_variable_dependencies(&func_id).unwrap();
        assert!(deps.contains("self"));
        assert!(deps.contains("x"));
        assert!(deps.contains("y"));
    }

    #[test]
    fn test_populate_empty_deps() {
        let mut graph = DataFlowGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "no_params".to_string(), 1);
        let params: Vec<String> = vec![];

        let count = populate_deps(&mut graph, &func_id, &params);

        assert_eq!(count, 0);
        assert!(graph.get_variable_dependencies(&func_id).is_none());
    }

    #[test]
    fn test_populate_transformations() {
        let mut graph = DataFlowGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "transform_fn".to_string(), 1);
        let patterns = vec![
            TransformationPattern {
                pattern_type: PatternType::Map,
                line: 5,
            },
            TransformationPattern {
                pattern_type: PatternType::Filter,
                line: 6,
            },
        ];

        let count = populate_transformations(&mut graph, &func_id, &patterns);

        assert_eq!(count, 2);
    }

    #[test]
    fn test_pattern_type_to_string() {
        assert_eq!(pattern_type_to_string(PatternType::Map), "map");
        assert_eq!(pattern_type_to_string(PatternType::Filter), "filter");
        assert_eq!(pattern_type_to_string(PatternType::Fold), "fold");
        assert_eq!(pattern_type_to_string(PatternType::FlatMap), "flat_map");
        assert_eq!(pattern_type_to_string(PatternType::Collect), "collect");
        assert_eq!(pattern_type_to_string(PatternType::ForEach), "for_each");
        assert_eq!(pattern_type_to_string(PatternType::Find), "find");
        assert_eq!(pattern_type_to_string(PatternType::Any), "any");
        assert_eq!(pattern_type_to_string(PatternType::All), "all");
        assert_eq!(pattern_type_to_string(PatternType::Reduce), "reduce");
    }

    #[test]
    fn test_format_io_type() {
        assert_eq!(format_io_type(IoType::File), "file");
        assert_eq!(format_io_type(IoType::Console), "console");
        assert_eq!(format_io_type(IoType::Network), "network");
        assert_eq!(format_io_type(IoType::Database), "database");
        assert_eq!(format_io_type(IoType::AsyncIO), "async_io");
        assert_eq!(format_io_type(IoType::Environment), "environment");
        assert_eq!(format_io_type(IoType::System), "system");
    }

    #[test]
    fn test_populate_data_flow_full() {
        let mut graph = DataFlowGraph::new();
        let mut extracted = HashMap::new();
        let mut file_data = create_test_file();

        // Add some data to the function
        file_data.functions[0].io_operations.push(IoOperation {
            io_type: IoType::File,
            description: "read".to_string(),
            line: 5,
        });
        file_data.functions[0].parameter_names = vec!["x".to_string()];
        file_data.functions[0]
            .transformation_patterns
            .push(TransformationPattern {
                pattern_type: PatternType::Map,
                line: 7,
            });

        extracted.insert(PathBuf::from("src/main.rs"), file_data);

        let stats = populate_data_flow(&mut graph, &extracted);

        assert_eq!(stats.purity_entries, 1);
        assert_eq!(stats.io_operations, 1);
        assert_eq!(stats.variable_deps, 1);
        assert_eq!(stats.transformations, 1);
    }

    #[test]
    fn test_populate_from_single_file() {
        let mut graph = DataFlowGraph::new();
        let file_data = create_test_file();

        let stats = populate_from_single_file(&mut graph, &file_data);

        assert_eq!(stats.purity_entries, 1);
    }

    #[test]
    fn test_summarize_io_operations() {
        let mut extracted = HashMap::new();
        let mut file_data = create_test_file();

        file_data.functions[0].io_operations = vec![
            IoOperation {
                io_type: IoType::File,
                description: "read".to_string(),
                line: 1,
            },
            IoOperation {
                io_type: IoType::File,
                description: "write".to_string(),
                line: 2,
            },
            IoOperation {
                io_type: IoType::Console,
                description: "log".to_string(),
                line: 3,
            },
        ];

        extracted.insert(PathBuf::from("src/main.rs"), file_data);

        let summary = summarize_io_operations(&extracted);

        assert_eq!(*summary.get("file").unwrap(), 2);
        assert_eq!(*summary.get("console").unwrap(), 1);
    }

    #[test]
    fn test_count_purity() {
        let mut extracted = HashMap::new();
        let mut file_data = create_test_file();

        // Add an impure function
        let mut impure_fn = create_test_function("impure", 20);
        impure_fn.purity_analysis = PurityAnalysisData::impure("mutation");
        file_data.functions.push(impure_fn);

        extracted.insert(PathBuf::from("src/main.rs"), file_data);

        let (pure, impure) = count_purity(&extracted);

        assert_eq!(pure, 1);
        assert_eq!(impure, 1);
    }
}
