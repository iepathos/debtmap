# Spec 186: Codebase-Wide Type Organization Analysis

**Status**: Draft
**Priority**: High
**Dependencies**: [180, 181, 183]
**Created**: 2025-01-19

## Context

Current analysis (specs 181-185) focuses on **individual god objects**, but type-based organization issues exist at the **codebase level**:

### Problem 1: Type Scattering
Methods for the same type scattered across multiple files:

```
src/
├── priority_item.rs        # struct PriorityItem { ... }
├── utils.rs                # fn format_priority(item: &PriorityItem)
├── validation.rs           # fn validate_priority(item: &PriorityItem)
└── processing.rs           # fn process_priority(item: &PriorityItem)
```

**Issue**: PriorityItem's behavior is split across 4 files instead of being cohesive.

### Problem 2: Orphaned Functions
Standalone functions that should be methods:

```rust
// src/analysis.rs
pub struct FileMetrics { ... }

// src/utils.rs  ❌ Wrong location
pub fn calculate_complexity(metrics: &FileMetrics) -> u32 { }
pub fn is_high_complexity(metrics: &FileMetrics) -> bool { }

// Should be:
impl FileMetrics {
    pub fn complexity(&self) -> u32 { }
    pub fn is_high_complexity(&self) -> bool { }
}
```

### Problem 3: Utilities Sprawl
Multiple utility modules with mixed responsibilities:

```
src/
├── utils.rs                # 50 functions, 10 different types
├── helpers.rs              # 40 functions, 8 different types
├── common.rs               # 30 functions, 6 different types
└── shared.rs               # 25 functions, 5 different types
```

### Problem 4: Cross-File Technical Grouping
Verb-based organization across codebase:

```
src/
├── formatting/
│   ├── format_metrics.rs   # Formats FileMetrics
│   ├── format_debt.rs      # Formats DebtItem
│   └── format_god.rs       # Formats GodObject
├── validation/
│   ├── validate_metrics.rs # Validates FileMetrics
│   ├── validate_debt.rs    # Validates DebtItem
│   └── validate_god.rs     # Validates GodObject
└── calculation/
    ├── calc_metrics.rs     # Calculates FileMetrics
    ├── calc_debt.rs        # Calculates DebtItem
    └── calc_god.rs         # Calculates GodObject
```

**Issue**: Each type's behavior is split across 3+ directories by verb.

## Objective

Analyze entire codebase to detect type-based organization issues and recommend consolidation following idiomatic Rust principles.

## Requirements

### 1. Cross-File Type Analysis

```rust
// src/organization/codebase_type_analyzer.rs

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use crate::organization::type_based_clustering::MethodSignature;
use crate::organization::architecture_utils::*;

pub struct CodebaseTypeAnalyzer {
    config: CodebaseAnalysisConfig,
}

#[derive(Clone, Debug)]
pub struct CodebaseAnalysisConfig {
    /// Minimum methods scattered to report (default: 3)
    pub min_scattered_methods: usize,

    /// Minimum files a type appears in to report scattering (default: 2)
    pub min_file_scattering: usize,

    /// Minimum orphaned functions to report (default: 3)
    pub min_orphaned_functions: usize,

    /// Detect utilities sprawl (default: true)
    pub detect_utilities_sprawl: bool,
}

impl Default for CodebaseAnalysisConfig {
    fn default() -> Self {
        Self {
            min_scattered_methods: 3,
            min_file_scattering: 2,
            min_orphaned_functions: 3,
            detect_utilities_sprawl: true,
        }
    }
}

/// Analysis result for entire codebase
#[derive(Clone, Debug)]
pub struct CodebaseTypeAnalysis {
    /// Types with methods scattered across multiple files
    pub scattered_types: Vec<ScatteredType>,

    /// Orphaned functions that should be methods
    pub orphaned_functions: Vec<OrphanedFunctionGroup>,

    /// Utilities modules with mixed responsibilities
    pub utilities_sprawl: Vec<UtilitiesModule>,

    /// Cross-file technical groupings (verb-based)
    pub technical_groupings: Vec<TechnicalGrouping>,

    /// Recommended reorganization
    pub recommendations: Vec<CodebaseRecommendation>,
}

/// Type with methods scattered across multiple files
#[derive(Clone, Debug)]
pub struct ScatteredType {
    /// Type name (e.g., "PriorityItem", "FileMetrics")
    pub type_name: String,

    /// File where type is defined
    pub definition_file: PathBuf,

    /// Files containing methods for this type
    pub method_locations: HashMap<PathBuf, Vec<String>>,

    /// Total methods scattered
    pub total_methods: usize,

    /// Number of files with scattered methods
    pub file_count: usize,

    /// Severity (High if >5 files, Medium if >3, Low if 2)
    pub severity: ScatteringSeverity,
}

#[derive(Clone, Debug, PartialEq, Ord, PartialOrd, Eq)]
pub enum ScatteringSeverity {
    Low,      // 2 files
    Medium,   // 3-5 files
    High,     // 6+ files
}

/// Group of orphaned functions that should belong to a type
#[derive(Clone, Debug)]
pub struct OrphanedFunctionGroup {
    /// Type these functions operate on
    pub target_type: String,

    /// Orphaned function names
    pub functions: Vec<String>,

    /// Files containing orphaned functions
    pub source_files: HashSet<PathBuf>,

    /// Suggested home for these functions
    pub suggested_location: PathBuf,
}

/// Utilities module with mixed responsibilities
#[derive(Clone, Debug)]
pub struct UtilitiesModule {
    pub file_path: PathBuf,
    pub function_count: usize,
    pub distinct_types: HashSet<String>,
    pub type_distribution: HashMap<String, usize>,
}

/// Cross-file technical grouping
#[derive(Clone, Debug)]
pub struct TechnicalGrouping {
    /// Technical category (formatting, validation, calculation, etc.)
    pub category: String,

    /// Files involved in this grouping
    pub files: Vec<PathBuf>,

    /// Types affected
    pub types: HashSet<String>,

    /// Suggested reorganization
    pub suggestion: String,
}

/// Codebase-level recommendation
#[derive(Clone, Debug)]
pub struct CodebaseRecommendation {
    pub title: String,
    pub severity: RecommendationSeverity,
    pub description: String,
    pub actions: Vec<RefactoringAction>,
    pub estimated_effort: EffortEstimate,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RecommendationSeverity {
    Critical,  // Major architecture issue
    High,      // Significant improvement needed
    Medium,    // Moderate improvement
    Low,       // Nice to have
}

#[derive(Clone, Debug)]
pub struct RefactoringAction {
    pub action_type: ActionType,
    pub from_file: PathBuf,
    pub to_file: PathBuf,
    pub items: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum ActionType {
    MoveMethod,
    ExtractType,
    ConsolidateModule,
    CreateImplBlock,
}

#[derive(Clone, Debug)]
pub struct EffortEstimate {
    pub hours: f32,
    pub complexity: ComplexityLevel,
    pub risk: RiskLevel,
}

#[derive(Clone, Debug)]
pub enum ComplexityLevel {
    Simple,    // < 1 hour
    Moderate,  // 1-4 hours
    Complex,   // 4-8 hours
    VeryComplex, // > 8 hours
}

#[derive(Clone, Debug)]
pub enum RiskLevel {
    Low,       // Safe refactoring
    Medium,    // Some dependencies
    High,      // Major changes
}
```

### 2. Type Scattering Detection

```rust
impl CodebaseTypeAnalyzer {
    /// Analyze entire codebase for type scattering
    pub fn analyze_codebase(
        &self,
        codebase: &CodebaseSnapshot,
    ) -> CodebaseTypeAnalysis {
        // 1. Build type-to-methods mapping across all files
        let type_methods = self.build_type_method_map(codebase);

        // 2. Detect scattered types
        let scattered = self.detect_scattered_types(&type_methods, codebase);

        // 3. Detect orphaned functions
        let orphaned = self.detect_orphaned_functions(codebase);

        // 4. Detect utilities sprawl
        let utilities = if self.config.detect_utilities_sprawl {
            self.detect_utilities_sprawl(codebase)
        } else {
            vec![]
        };

        // 5. Detect technical groupings
        let technical = self.detect_technical_groupings(codebase);

        // 6. Generate recommendations
        let recommendations = self.generate_recommendations(
            &scattered,
            &orphaned,
            &utilities,
            &technical,
        );

        CodebaseTypeAnalysis {
            scattered_types: scattered,
            orphaned_functions: orphaned,
            utilities_sprawl: utilities,
            technical_groupings: technical,
            recommendations,
        }
    }

    /// Build mapping of types to methods across all files
    fn build_type_method_map(
        &self,
        codebase: &CodebaseSnapshot,
    ) -> HashMap<String, TypeMethodLocations> {
        let mut type_map: HashMap<String, TypeMethodLocations> = HashMap::new();

        for file in &codebase.files {
            // Extract all method signatures from this file
            let signatures = extract_method_signatures(&file.ast).unwrap_or_default();

            // Group by parameter types
            for sig in signatures {
                // Check if this is an impl method or standalone function
                if let Some(self_type) = &sig.self_type {
                    // impl method - belongs to self_type
                    type_map.entry(self_type.name.clone())
                        .or_insert_with(|| TypeMethodLocations::new(self_type.name.clone()))
                        .add_method(file.path.clone(), sig.name.clone(), true);
                } else {
                    // Standalone function - check parameter types
                    for param in &sig.param_types {
                        let base_type = extract_base_type(&param.name);
                        if is_domain_type(&base_type) {
                            type_map.entry(base_type.clone())
                                .or_insert_with(|| TypeMethodLocations::new(base_type))
                                .add_method(file.path.clone(), sig.name.clone(), false);
                        }
                    }
                }
            }
        }

        type_map
    }

    /// Detect types with methods scattered across multiple files
    fn detect_scattered_types(
        &self,
        type_map: &HashMap<String, TypeMethodLocations>,
        codebase: &CodebaseSnapshot,
    ) -> Vec<ScatteredType> {
        let mut scattered = Vec::new();

        for (type_name, locations) in type_map {
            let file_count = locations.files.len();

            // Only report if scattered across min_file_scattering+ files
            if file_count < self.config.min_file_scattering {
                continue;
            }

            // Only report if has min_scattered_methods+ methods
            let total_methods: usize = locations.files.values()
                .map(|methods| methods.len())
                .sum();

            if total_methods < self.config.min_scattered_methods {
                continue;
            }

            // Find where type is defined
            let definition_file = self.find_type_definition(type_name, codebase)
                .unwrap_or_else(|| PathBuf::from("unknown"));

            // Determine severity
            let severity = if file_count >= 6 {
                ScatteringSeverity::High
            } else if file_count >= 3 {
                ScatteringSeverity::Medium
            } else {
                ScatteringSeverity::Low
            };

            scattered.push(ScatteredType {
                type_name: type_name.clone(),
                definition_file,
                method_locations: locations.files.clone(),
                total_methods,
                file_count,
                severity,
            });
        }

        // Sort by severity, then by total methods
        scattered.sort_by(|a, b| {
            b.severity.cmp(&a.severity)
                .then(b.total_methods.cmp(&a.total_methods))
        });

        scattered
    }

    fn find_type_definition(
        &self,
        type_name: &str,
        codebase: &CodebaseSnapshot,
    ) -> Option<PathBuf> {
        // Search for struct/enum definition
        for file in &codebase.files {
            for item in &file.ast.items {
                match item {
                    syn::Item::Struct(s) if s.ident == type_name => {
                        return Some(file.path.clone());
                    }
                    syn::Item::Enum(e) if e.ident == type_name => {
                        return Some(file.path.clone());
                    }
                    _ => continue,
                }
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
struct TypeMethodLocations {
    type_name: String,
    /// Map of file path to methods in that file
    files: HashMap<PathBuf, Vec<String>>,
    /// Whether each method is impl (true) or standalone (false)
    method_impl_status: HashMap<String, bool>,
}

impl TypeMethodLocations {
    fn new(type_name: String) -> Self {
        Self {
            type_name,
            files: HashMap::new(),
            method_impl_status: HashMap::new(),
        }
    }

    fn add_method(&mut self, file: PathBuf, method: String, is_impl: bool) {
        self.files.entry(file).or_default().push(method.clone());
        self.method_impl_status.insert(method, is_impl);
    }
}
```

### 3. Orphaned Function Detection

```rust
impl CodebaseTypeAnalyzer {
    /// Detect standalone functions that should be methods
    fn detect_orphaned_functions(
        &self,
        codebase: &CodebaseSnapshot,
    ) -> Vec<OrphanedFunctionGroup> {
        let mut orphaned_map: HashMap<String, Vec<(PathBuf, String)>> = HashMap::new();

        for file in &codebase.files {
            let signatures = extract_method_signatures(&file.ast).unwrap_or_default();

            for sig in signatures {
                // Skip if already an impl method
                if sig.self_type.is_some() {
                    continue;
                }

                // Check if this operates on a single dominant type
                if let Some(dominant_type) = self.find_dominant_parameter_type(&sig) {
                    orphaned_map
                        .entry(dominant_type.clone())
                        .or_default()
                        .push((file.path.clone(), sig.name.clone()));
                }
            }
        }

        // Convert to OrphanedFunctionGroup
        let mut groups = Vec::new();

        for (type_name, functions) in orphaned_map {
            // Only report if min_orphaned_functions+ functions
            if functions.len() < self.config.min_orphaned_functions {
                continue;
            }

            let source_files: HashSet<_> = functions.iter()
                .map(|(path, _)| path.clone())
                .collect();

            let function_names: Vec<_> = functions.iter()
                .map(|(_, name)| name.clone())
                .collect();

            // Suggest moving to type's definition file
            let suggested_location = self.find_type_definition(&type_name, codebase)
                .unwrap_or_else(|| PathBuf::from(format!("src/{}.rs", to_snake_case(&type_name))));

            groups.push(OrphanedFunctionGroup {
                target_type: type_name,
                functions: function_names,
                source_files,
                suggested_location,
            });
        }

        groups.sort_by_key(|g| std::cmp::Reverse(g.functions.len()));
        groups
    }

    /// Find dominant type in function parameters
    fn find_dominant_parameter_type(&self, sig: &MethodSignature) -> Option<String> {
        // Count domain types in parameters
        let mut type_counts: HashMap<String, usize> = HashMap::new();

        for param in &sig.param_types {
            let base_type = extract_base_type(&param.name);
            if is_domain_type(&base_type) {
                *type_counts.entry(base_type).or_insert(0) += 1;
            }
        }

        // Check return type
        if let Some(ret) = &sig.return_type {
            let base_type = extract_base_type(&ret.name);
            if is_domain_type(&base_type) {
                *type_counts.entry(base_type).or_insert(0) += 1;
            }
        }

        // Return type with highest count (must be dominant, not tied)
        let total_types: usize = type_counts.values().sum();

        type_counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .filter(|(_, count)| *count >= total_types / 2) // At least 50% of types
            .map(|(name, _)| name)
    }
}
```

### 4. Utilities Sprawl Detection

```rust
impl CodebaseTypeAnalyzer {
    /// Detect utilities modules with mixed responsibilities
    fn detect_utilities_sprawl(
        &self,
        codebase: &CodebaseSnapshot,
    ) -> Vec<UtilitiesModule> {
        let mut utilities = Vec::new();

        for file in &codebase.files {
            // Check if file is utilities-like
            let file_name = file.path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            if !self.is_utilities_file(file_name) {
                continue;
            }

            // Analyze type distribution in this file
            let signatures = extract_method_signatures(&file.ast).unwrap_or_default();
            let mut type_distribution: HashMap<String, usize> = HashMap::new();

            for sig in &signatures {
                for param in &sig.param_types {
                    let base_type = extract_base_type(&param.name);
                    if is_domain_type(&base_type) {
                        *type_distribution.entry(base_type).or_insert(0) += 1;
                    }
                }
            }

            // Only report if mixed (3+ distinct types)
            if type_distribution.len() >= 3 {
                utilities.push(UtilitiesModule {
                    file_path: file.path.clone(),
                    function_count: signatures.len(),
                    distinct_types: type_distribution.keys().cloned().collect(),
                    type_distribution,
                });
            }
        }

        utilities.sort_by_key(|u| std::cmp::Reverse(u.distinct_types.len()));
        utilities
    }

    fn is_utilities_file(&self, file_name: &str) -> bool {
        matches!(
            file_name,
            "utils" | "util" | "utilities" | "helpers" | "helper" |
            "common" | "shared" | "misc" | "miscellaneous"
        )
    }
}
```

### 5. Recommendation Generation

```rust
impl CodebaseTypeAnalyzer {
    fn generate_recommendations(
        &self,
        scattered: &[ScatteredType],
        orphaned: &[OrphanedFunctionGroup],
        utilities: &[UtilitiesModule],
        technical: &[TechnicalGrouping],
    ) -> Vec<CodebaseRecommendation> {
        let mut recommendations = Vec::new();

        // Recommendation 1: Consolidate scattered types
        for scattered_type in scattered {
            recommendations.push(CodebaseRecommendation {
                title: format!("Consolidate {} methods", scattered_type.type_name),
                severity: match scattered_type.severity {
                    ScatteringSeverity::High => RecommendationSeverity::Critical,
                    ScatteringSeverity::Medium => RecommendationSeverity::High,
                    ScatteringSeverity::Low => RecommendationSeverity::Medium,
                },
                description: format!(
                    "{} has {} methods scattered across {} files. Consolidate into {}.",
                    scattered_type.type_name,
                    scattered_type.total_methods,
                    scattered_type.file_count,
                    scattered_type.definition_file.display()
                ),
                actions: self.generate_consolidation_actions(scattered_type),
                estimated_effort: EffortEstimate {
                    hours: (scattered_type.file_count as f32 * 0.5),
                    complexity: if scattered_type.file_count > 5 {
                        ComplexityLevel::Complex
                    } else {
                        ComplexityLevel::Moderate
                    },
                    risk: RiskLevel::Medium,
                },
            });
        }

        // Recommendation 2: Convert orphaned functions to methods
        for orphaned_group in orphaned {
            recommendations.push(CodebaseRecommendation {
                title: format!("Convert {} functions to methods", orphaned_group.target_type),
                severity: RecommendationSeverity::High,
                description: format!(
                    "{} standalone functions operate on {}. Convert to impl methods.",
                    orphaned_group.functions.len(),
                    orphaned_group.target_type
                ),
                actions: vec![RefactoringAction {
                    action_type: ActionType::CreateImplBlock,
                    from_file: orphaned_group.source_files.iter().next().unwrap().clone(),
                    to_file: orphaned_group.suggested_location.clone(),
                    items: orphaned_group.functions.clone(),
                }],
                estimated_effort: EffortEstimate {
                    hours: orphaned_group.functions.len() as f32 * 0.25,
                    complexity: ComplexityLevel::Simple,
                    risk: RiskLevel::Low,
                },
            });
        }

        // Recommendation 3: Break up utilities modules
        for util in utilities {
            recommendations.push(CodebaseRecommendation {
                title: format!("Break up {}", util.file_path.display()),
                severity: RecommendationSeverity::High,
                description: format!(
                    "Utilities module has {} functions operating on {} distinct types. \
                     Move functions to appropriate type modules.",
                    util.function_count,
                    util.distinct_types.len()
                ),
                actions: self.generate_utilities_breakup_actions(util),
                estimated_effort: EffortEstimate {
                    hours: (util.function_count as f32 * 0.2),
                    complexity: ComplexityLevel::Moderate,
                    risk: RiskLevel::Medium,
                },
            });
        }

        recommendations.sort_by_key(|r| std::cmp::Reverse(r.severity.clone()));
        recommendations
    }

    fn generate_consolidation_actions(&self, scattered: &ScatteredType) -> Vec<RefactoringAction> {
        let mut actions = Vec::new();

        for (source_file, methods) in &scattered.method_locations {
            if source_file == &scattered.definition_file {
                continue; // Already in correct location
            }

            actions.push(RefactoringAction {
                action_type: ActionType::MoveMethod,
                from_file: source_file.clone(),
                to_file: scattered.definition_file.clone(),
                items: methods.clone(),
            });
        }

        actions
    }

    fn generate_utilities_breakup_actions(&self, util: &UtilitiesModule) -> Vec<RefactoringAction> {
        let mut actions = Vec::new();

        for type_name in &util.distinct_types {
            actions.push(RefactoringAction {
                action_type: ActionType::ConsolidateModule,
                from_file: util.file_path.clone(),
                to_file: PathBuf::from(format!("src/{}.rs", to_snake_case(type_name))),
                items: vec![format!("Functions operating on {}", type_name)],
            });
        }

        actions
    }
}
```

## Output Format

```
Codebase Type Organization Analysis
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Summary:
  ⚠ 5 scattered types detected
  ⚠ 3 orphaned function groups detected
  ⚠ 2 utilities modules with mixed responsibilities
  ✓ 0 cross-file technical groupings

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[CRITICAL] Scattered Type: FileMetrics
  Definition: src/analysis/file_metrics.rs
  Methods scattered across 6 files (23 total methods):

  src/utils.rs (8 methods):
    - calculate_complexity, calculate_coverage, calculate_debt_score
    - format_metrics, validate_metrics, merge_metrics
    - normalize_metrics, aggregate_metrics

  src/helpers.rs (6 methods):
    - get_metric_value, set_metric_value, update_metric
    - is_high_complexity, is_low_coverage, has_debt

  src/processing.rs (4 methods):
    - process_metrics, batch_process, filter_metrics, sort_metrics

  src/formatting/metrics.rs (3 methods):
    - format_detailed, format_summary, format_json

  src/validation/metrics.rs (2 methods):
    - validate_ranges, validate_consistency

  Recommendation:
    Move all 23 methods to src/analysis/file_metrics.rs as impl methods

    impl FileMetrics {
        // Calculation methods
        pub fn complexity(&self) -> u32 { }
        pub fn coverage(&self) -> f64 { }
        pub fn debt_score(&self) -> f64 { }

        // Validation methods
        pub fn validate(&self) -> Result<()> { }
        pub fn is_high_complexity(&self) -> bool { }

        // Formatting methods
        pub fn format_detailed(&self) -> String { }
        pub fn format_summary(&self) -> String { }
    }

  Estimated Effort: 3 hours (Moderate complexity, Medium risk)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[HIGH] Orphaned Functions: PriorityItem
  12 standalone functions should be methods:

  src/utils.rs:
    - format_priority, validate_priority, calculate_priority_score
    - normalize_priority, is_high_priority

  src/helpers.rs:
    - get_priority_location, get_priority_metrics, update_priority

  src/processing.rs:
    - process_priority, batch_priorities, filter_priorities, sort_priorities

  Recommendation:
    Convert to impl methods in src/priority/priority_item.rs:

    impl PriorityItem {
        pub fn format(&self) -> String { }
        pub fn validate(&self) -> Result<()> { }
        pub fn score(&self) -> f64 { }
        pub fn is_high_priority(&self) -> bool { }
    }

  Estimated Effort: 3 hours (Simple complexity, Low risk)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[HIGH] Utilities Sprawl: src/utils.rs
  50 functions operating on 10 distinct types:

  Type Distribution:
    FileMetrics: 8 functions
    PriorityItem: 5 functions
    DebtItem: 6 functions
    GodObjectAnalysis: 4 functions
    ComplexityMetrics: 3 functions
    CoverageMetrics: 3 functions
    ... 4 more types

  Recommendation:
    Break up utils.rs - move functions to appropriate type modules:
    - FileMetrics functions → src/analysis/file_metrics.rs
    - PriorityItem functions → src/priority/priority_item.rs
    - DebtItem functions → src/debt/debt_item.rs
    etc.

  Estimated Effort: 10 hours (Moderate complexity, Medium risk)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Total Recommendations: 10
  Critical: 2
  High: 5
  Medium: 3
  Low: 0

Estimated Total Effort: 28 hours

Priority Order:
  1. Consolidate FileMetrics (Critical, 3h)
  2. Consolidate DebtItem (Critical, 2.5h)
  3. Convert PriorityItem orphans to methods (High, 3h)
  4. Break up utils.rs (High, 10h)
  5. Consolidate GodObjectAnalysis (High, 2h)
```

## Integration with Existing Analysis

### Codebase Snapshot Structure

```rust
pub struct CodebaseSnapshot {
    pub files: Vec<FileSnapshot>,
    pub root_path: PathBuf,
}

pub struct FileSnapshot {
    pub path: PathBuf,
    pub ast: syn::File,
    pub content: String,
}

impl CodebaseSnapshot {
    /// Create snapshot of entire codebase
    pub fn from_directory(root: &Path) -> Result<Self, String> {
        let mut files = Vec::new();

        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
        {
            let content = std::fs::read_to_string(entry.path())
                .map_err(|e| format!("Failed to read {}: {}", entry.path().display(), e))?;

            let ast = syn::parse_file(&content)
                .map_err(|e| format!("Failed to parse {}: {}", entry.path().display(), e))?;

            files.push(FileSnapshot {
                path: entry.path().to_path_buf(),
                ast,
                content,
            });
        }

        Ok(Self {
            files,
            root_path: root.to_path_buf(),
        })
    }
}
```

### CLI Integration

```bash
# Analyze entire codebase for type organization issues
debtmap analyze-types --codebase ./src

# With filters
debtmap analyze-types --codebase ./src \
    --min-scattered 3 \
    --min-files 2 \
    --detect-utilities

# Generate report
debtmap analyze-types --codebase ./src --output type-analysis.json

# Auto-fix (with confirmation)
debtmap analyze-types --codebase ./src --auto-fix --dry-run
```

## Dependencies

- **Spec 180**: Shared utilities (case conversion, type analysis)
- **Spec 181**: Type-based clustering (method signature extraction)
- **Spec 183**: Anti-pattern detection (utilities detection, verb analysis)
- `walkdir` crate for directory traversal
- `syn` crate for AST parsing

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scattered_type_detection() {
        let snapshot = create_test_snapshot(&[
            ("src/metrics.rs", "struct FileMetrics { }"),
            ("src/utils.rs", "fn calculate(m: &FileMetrics) { }"),
            ("src/helpers.rs", "fn validate(m: &FileMetrics) { }"),
        ]);

        let analyzer = CodebaseTypeAnalyzer::new();
        let analysis = analyzer.analyze_codebase(&snapshot);

        assert_eq!(analysis.scattered_types.len(), 1);
        assert_eq!(analysis.scattered_types[0].type_name, "FileMetrics");
        assert_eq!(analysis.scattered_types[0].file_count, 3);
    }

    #[test]
    fn test_orphaned_function_detection() {
        // Test detecting functions that should be methods
    }

    #[test]
    fn test_utilities_sprawl_detection() {
        // Test detecting utilities modules
    }
}
```

### Integration Tests

```rust
// Test on real codebases
#[test]
fn test_analyze_debtmap_codebase() {
    let snapshot = CodebaseSnapshot::from_directory(Path::new("src")).unwrap();
    let analyzer = CodebaseTypeAnalyzer::new();
    let analysis = analyzer.analyze_codebase(&snapshot);

    // Should detect actual scattering in debtmap
    assert!(analysis.scattered_types.len() > 0);
}
```

## Success Metrics

- Detect 90%+ of scattered types (manual validation)
- Identify orphaned functions with 85%+ accuracy
- Generate actionable recommendations (user feedback)
- Analysis completes in < 5 seconds for 100-file codebase
- Zero false positives for well-organized codebases

## Future Enhancements

1. **Auto-refactoring**: Generate pull requests with actual code changes
2. **Incremental analysis**: Only analyze changed files
3. **CI integration**: Block PRs that increase scattering
4. **Visualization**: Generate dependency graphs showing type scattering
5. **IDE integration**: Real-time warnings for misplaced methods

## Migration Path

1. **Week 1**: Implement scattered type detection
2. **Week 2**: Add orphaned function detection
3. **Week 3**: Add utilities sprawl detection
4. **Week 4**: Add recommendation generation and output formatting
5. **Week 5**: CLI integration and testing
6. **Week 6**: Documentation and polish
