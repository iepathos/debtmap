use debtmap::analysis::data_flow::{BlockId, DataFlowAnalysis, ReachingDefinitions};
use debtmap::data_flow::{
    CfgAnalysisWithContext, DataFlowGraph, IoOperation, MutationInfo, PurityInfo,
};
use debtmap::priority::call_graph::FunctionId;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug)]
pub enum Fact {
    Purity,
    Dependencies,
    Io,
    Mutation,
    Cfg,
    CfgContext,
}

pub const ALL: [Fact; 6] = [
    Fact::Purity,
    Fact::Dependencies,
    Fact::Io,
    Fact::Mutation,
    Fact::Cfg,
    Fact::CfgContext,
];
pub const SERIALIZED: [Fact; 4] = [Fact::Purity, Fact::Dependencies, Fact::Io, Fact::Mutation];

fn cfg(marker: &str) -> DataFlowAnalysis {
    let number = match marker {
        "target" => 1,
        "unrelated" => 2,
        "tie" => 3,
        _ => 4,
    };
    DataFlowAnalysis {
        reaching_defs: ReachingDefinitions {
            reach_in: HashMap::from([(BlockId(number), HashSet::new())]),
            ..Default::default()
        },
    }
}

impl Fact {
    pub fn write(self, graph: &mut DataFlowGraph, id: FunctionId, marker: &str) {
        let marker = marker.to_owned();
        match self {
            Self::Purity => graph.set_purity_info(
                id,
                PurityInfo {
                    is_pure: false,
                    confidence: 1.0,
                    impurity_reasons: vec![marker],
                },
            ),
            Self::Dependencies => graph.add_variable_dependencies(id, HashSet::from([marker])),
            Self::Io => graph.add_io_operation(
                id,
                IoOperation {
                    operation_type: marker,
                    variables: vec![],
                    line: 10,
                },
            ),
            Self::Mutation => graph.set_mutation_info(
                id,
                MutationInfo {
                    has_mutations: true,
                    detected_mutations: vec![marker],
                },
            ),
            Self::Cfg => graph.set_cfg_analysis(id, cfg(&marker)),
            Self::CfgContext => graph.set_cfg_analysis_with_context(
                id,
                CfgAnalysisWithContext::new(vec![marker.clone()], cfg(&marker)),
            ),
        }
    }

    pub fn read(self, graph: &DataFlowGraph, id: &FunctionId) -> Option<Vec<String>> {
        match self {
            Self::Purity => graph
                .get_purity_info(id)
                .map(|value| value.impurity_reasons.clone()),
            Self::Dependencies => graph.get_variable_dependencies(id).map(|value| {
                let mut names: Vec<_> = value.iter().cloned().collect();
                names.sort();
                names
            }),
            Self::Io => graph.get_io_operations(id).map(|values| {
                values
                    .iter()
                    .map(|value| value.operation_type.clone())
                    .collect()
            }),
            Self::Mutation => graph
                .get_mutation_info(id)
                .map(|value| value.detected_mutations.clone()),
            Self::Cfg => graph.get_cfg_analysis(id).map(|value| {
                value
                    .reaching_defs
                    .reach_in
                    .keys()
                    .map(|id| {
                        match id.0 {
                            1 => "target",
                            2 => "unrelated",
                            3 => "tie",
                            _ => "incoming",
                        }
                        .to_owned()
                    })
                    .collect()
            }),
            Self::CfgContext => graph
                .get_cfg_analysis_with_context(id)
                .map(|value| value.var_names.clone()),
        }
    }

    pub fn after_matching_write(self) -> Option<Vec<String>> {
        Some(if matches!(self, Self::Io) {
            vec!["target".into(), "incoming".into()]
        } else {
            vec!["incoming".into()]
        })
    }

    pub fn field(self) -> &'static str {
        match self {
            Self::Purity => "purity_analysis",
            Self::Dependencies => "variable_deps",
            Self::Io => "io_operations",
            Self::Mutation => "mutation_analysis",
            Self::Cfg | Self::CfgContext => unreachable!("CFG is deliberately skipped by serde"),
        }
    }
}

pub fn marker(value: &str) -> Option<Vec<String>> {
    Some(vec![value.to_owned()])
}
