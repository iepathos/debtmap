//! Data types for module structure analysis
//!
//! Contains all structs, enums, and their core implementations for
//! representing module structure, components, and analysis results.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Categorized function counts within a module
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionCounts {
    pub module_level_functions: usize,
    pub impl_methods: usize,
    pub trait_methods: usize,
    pub nested_module_functions: usize,
    pub public_functions: usize,
    pub private_functions: usize,
}

impl FunctionCounts {
    pub fn new() -> Self {
        Self {
            module_level_functions: 0,
            impl_methods: 0,
            trait_methods: 0,
            nested_module_functions: 0,
            public_functions: 0,
            private_functions: 0,
        }
    }

    pub fn total(&self) -> usize {
        self.module_level_functions
            + self.impl_methods
            + self.trait_methods
            + self.nested_module_functions
    }
}

impl Default for FunctionCounts {
    fn default() -> Self {
        Self::new()
    }
}

/// A component within a module (struct, enum, impl block, or function)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleComponent {
    Struct {
        name: String,
        fields: usize,
        methods: usize,
        public: bool,
        line_range: (usize, usize),
    },
    Enum {
        name: String,
        variants: usize,
        methods: usize,
        public: bool,
        line_range: (usize, usize),
    },
    ImplBlock {
        target: String,
        methods: usize,
        trait_impl: Option<String>,
        line_range: (usize, usize),
    },
    ModuleLevelFunction {
        name: String,
        public: bool,
        lines: usize,
        complexity: u32,
    },
    NestedModule {
        name: String,
        file_path: Option<PathBuf>,
        functions: usize,
    },
}

impl ModuleComponent {
    pub fn name(&self) -> String {
        match self {
            ModuleComponent::Struct { name, .. } => name.clone(),
            ModuleComponent::Enum { name, .. } => name.clone(),
            ModuleComponent::ImplBlock {
                target, trait_impl, ..
            } => {
                if let Some(trait_name) = trait_impl {
                    format!("{} for {}", trait_name, target)
                } else {
                    format!("{} impl", target)
                }
            }
            ModuleComponent::ModuleLevelFunction { name, .. } => name.clone(),
            ModuleComponent::NestedModule { name, .. } => format!("mod {}", name),
        }
    }

    pub fn method_count(&self) -> usize {
        match self {
            ModuleComponent::Struct { methods, .. } => *methods,
            ModuleComponent::Enum { methods, .. } => *methods,
            ModuleComponent::ImplBlock { methods, .. } => *methods,
            ModuleComponent::ModuleLevelFunction { .. } => 1,
            ModuleComponent::NestedModule { functions, .. } => *functions,
        }
    }

    pub fn line_count(&self) -> usize {
        match self {
            ModuleComponent::Struct { line_range, .. } => line_range.1.saturating_sub(line_range.0),
            ModuleComponent::Enum { line_range, .. } => line_range.1.saturating_sub(line_range.0),
            ModuleComponent::ImplBlock { line_range, .. } => {
                line_range.1.saturating_sub(line_range.0)
            }
            ModuleComponent::ModuleLevelFunction { lines, .. } => *lines,
            ModuleComponent::NestedModule { .. } => 0,
        }
    }
}

/// Module facade detection information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleFacadeInfo {
    /// Whether this file qualifies as a module facade
    pub is_facade: bool,
    /// Number of submodules (both #\[path\] and inline)
    pub submodule_count: usize,
    /// List of #\[path\] declarations
    pub path_declarations: Vec<PathDeclaration>,
    /// Facade quality score (0.0-1.0)
    pub facade_score: f64,
    /// Organization quality classification
    pub organization_quality: OrganizationQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathDeclaration {
    pub module_name: String,
    pub file_path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrganizationQuality {
    Excellent,  // ≥10 submodules, facade_score ≥0.8
    Good,       // ≥5 submodules, facade_score ≥0.6
    Poor,       // ≥3 submodules, facade_score ≥0.5
    Monolithic, // <3 submodules or facade_score <0.5
}

/// Complete structure analysis of a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStructure {
    pub total_lines: usize,
    pub components: Vec<ModuleComponent>,
    pub function_counts: FunctionCounts,
    pub responsibility_count: usize,
    pub public_api_surface: usize,
    pub dependencies: ComponentDependencyGraph,
    /// Facade detection results (Spec 170)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facade_info: Option<ModuleFacadeInfo>,
}

/// Dependency graph for coupling analysis
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComponentDependencyGraph {
    pub components: Vec<String>,
    pub edges: Vec<(String, String)>,
    pub coupling_scores: HashMap<String, f64>,
}

impl ComponentDependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Identify components that are good candidates for extraction
    pub fn identify_split_candidates(&self) -> Vec<SplitRecommendation> {
        let mut candidates: Vec<_> = self
            .coupling_scores
            .iter()
            .filter(|(_, score)| **score < 0.3)
            .map(|(component, score)| SplitRecommendation {
                component: component.clone(),
                coupling_score: *score,
                suggested_module_name: suggest_module_name(component),
                estimated_lines: 200, // Placeholder - would need actual calculation
                difficulty: difficulty_from_coupling(*score),
            })
            .collect();

        candidates.sort_by(|a, b| {
            a.coupling_score
                .partial_cmp(&b.coupling_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates
    }

    pub fn analyze_coupling(&self) -> ComponentCouplingAnalysis {
        let mut afferent: HashMap<String, usize> = HashMap::new();
        let mut efferent: HashMap<String, usize> = HashMap::new();

        for (from, to) in &self.edges {
            *efferent.entry(from.clone()).or_insert(0) += 1;
            *afferent.entry(to.clone()).or_insert(0) += 1;
        }

        ComponentCouplingAnalysis {
            afferent,
            efferent,
            total_edges: self.edges.len(),
        }
    }
}

/// Recommendation for splitting out a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitRecommendation {
    pub component: String,
    pub coupling_score: f64,
    pub suggested_module_name: String,
    pub estimated_lines: usize,
    pub difficulty: Difficulty,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Difficulty {
    Easy,   // Coupling < 0.2
    Medium, // Coupling 0.2-0.5
    Hard,   // Coupling > 0.5
}

/// Coupling analysis results
#[derive(Debug, Clone)]
pub struct ComponentCouplingAnalysis {
    pub afferent: HashMap<String, usize>, // Incoming dependencies
    pub efferent: HashMap<String, usize>, // Outgoing dependencies
    pub total_edges: usize,
}

/// Grouped functions by domain/responsibility
#[derive(Debug, Clone)]
pub struct FunctionGroup {
    pub prefix: String,
    pub functions: Vec<String>,
}

// Pure helper functions

pub fn difficulty_from_coupling(score: f64) -> Difficulty {
    if score < 0.2 {
        Difficulty::Easy
    } else if score < 0.5 {
        Difficulty::Medium
    } else {
        Difficulty::Hard
    }
}

pub fn suggest_module_name(component: &str) -> String {
    let lower = component.to_lowercase().replace(' ', "_");
    if lower.ends_with("_impl") {
        lower.trim_end_matches("_impl").to_string()
    } else {
        lower
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_counts_total() {
        let counts = FunctionCounts {
            module_level_functions: 10,
            impl_methods: 20,
            trait_methods: 5,
            nested_module_functions: 3,
            public_functions: 15,
            private_functions: 8,
        };
        assert_eq!(counts.total(), 38);
    }

    #[test]
    fn test_difficulty_from_coupling() {
        assert_eq!(difficulty_from_coupling(0.1), Difficulty::Easy);
        assert_eq!(difficulty_from_coupling(0.3), Difficulty::Medium);
        assert_eq!(difficulty_from_coupling(0.6), Difficulty::Hard);
    }

    #[test]
    fn test_module_component_name() {
        let comp = ModuleComponent::Struct {
            name: "TestStruct".to_string(),
            fields: 5,
            methods: 10,
            public: true,
            line_range: (1, 50),
        };
        assert_eq!(comp.name(), "TestStruct");
    }
}
