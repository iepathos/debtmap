use std::path::PathBuf;

use crate::analyzers::solidity::effects::SolidityEffectSummary;
use crate::complexity::entropy_core::EntropyAnalysis;
use crate::core::ast::ClassDef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolidityFunctionKind {
    Function,
    Modifier,
    Constructor,
    Fallback,
    Receive,
}

#[derive(Debug, Clone)]
pub struct SolidityFunction {
    pub name: String,
    pub file: PathBuf,
    pub line: usize,
    pub length: usize,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub kind: SolidityFunctionKind,
    pub is_test: bool,
    pub visibility: Option<String>,
    pub calls: Vec<String>,
    pub advisory_patterns: Vec<String>,
    pub contract_name: Option<String>,
    pub entropy_analysis: Option<EntropyAnalysis>,
    pub state_mutability: Option<String>,
    pub effects: SolidityEffectSummary,
}

#[derive(Debug, Clone, Default)]
pub struct ContractInfo {
    pub name: String,
    pub kind: ContractKind,
    pub base_classes: Vec<String>,
    pub state_variable_count: usize,
    pub function_count: usize,
    pub state_variables: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ContractKind {
    #[default]
    Contract,
    Interface,
    Library,
}

#[derive(Debug, Clone, Default)]
pub struct SolidityAnalysis {
    pub contracts: Vec<ContractInfo>,
    pub functions: Vec<SolidityFunction>,
    pub is_test_file: bool,
    pub has_floating_pragma: bool,
}

impl ContractInfo {
    pub fn to_class_def(&self) -> ClassDef {
        ClassDef {
            name: self.name.clone(),
            base_classes: self.base_classes.clone(),
            methods: vec![],
            is_abstract: false,
            decorators: vec![],
            line: 0,
        }
    }
}
