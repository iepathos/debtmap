mod builder;
mod graph;
mod sinks;
mod sources;
mod taint;
mod validation;

pub use builder::DataFlowBuilder;
pub use graph::{DataFlowEdge, DataFlowGraph, DataFlowNode, ExpressionKind, NodeId};
pub use sinks::SinkDetector;
pub use sources::{OperationType, SourceDetector};
pub use taint::{TaintAnalysis, TaintAnalyzer, TaintState};
pub use validation::{ValidationDetector, ValidationGap};

use std::path::Path;
use syn::File;

/// Main data flow analyzer that orchestrates the analysis pipeline
pub struct DataFlowAnalyzer {
    builder: DataFlowBuilder,
    taint_analyzer: TaintAnalyzer,
    source_detector: SourceDetector,
    sink_detector: SinkDetector,
    validation_detector: ValidationDetector,
}

impl DataFlowAnalyzer {
    pub fn new() -> Self {
        Self {
            builder: DataFlowBuilder::new(),
            taint_analyzer: TaintAnalyzer::new(),
            source_detector: SourceDetector::new(),
            sink_detector: SinkDetector::new(),
            validation_detector: ValidationDetector::new(),
        }
    }

    /// Build a data flow graph from an AST
    pub fn build_graph(&mut self, file: &File, path: &Path) -> DataFlowGraph {
        self.builder.build(file, path)
    }

    /// Analyze taint propagation through the graph
    pub fn analyze_taint(&mut self, graph: &DataFlowGraph) -> TaintAnalysis {
        self.taint_analyzer.analyze(
            graph,
            &self.source_detector,
            &self.sink_detector,
            &self.validation_detector,
        )
    }

    /// Find validation gaps in the taint analysis
    pub fn find_validation_gaps(&self, analysis: &TaintAnalysis) -> Vec<ValidationGap> {
        self.validation_detector.find_gaps(analysis)
    }
}

impl Default for DataFlowAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
