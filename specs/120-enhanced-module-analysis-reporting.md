---
number: 120
title: Enhanced Module Analysis Reporting for God Objects
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-25
---

# Specification 120: Enhanced Module Analysis Reporting for God Objects

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap correctly identifies god object modules (large files with many functions), but the reporting lacks clarity about what "functions" means and how to visualize the module structure for effective refactoring.

**Current Problem**:

From latest analysis output:
```
#1 SCORE: 68.3 [CRITICAL - FILE - GOD OBJECT]
├─ src/priority/formatter.rs (2881 lines, 112 functions)
├─ WHY: This module contains 112 module functions across 0 responsibilities
└─ DEPENDENCIES: 112 functions may have complex interdependencies
```

**Confusion Points**:
1. **"112 functions"** - What does this count?
   - Module-level functions only?
   - Impl block methods?
   - Functions from sub-modules?
   - Actual count via `grep '^fn '` shows 58 functions

2. **"0 responsibilities"** - Clearly incorrect
   - File has 5 `impl` blocks with distinct purposes
   - Multiple struct definitions with separate concerns
   - Sub-modules (e.g., `formatter_verbosity.rs`)

3. **No structure visualization**
   - Can't see which impl blocks are large
   - Don't know which structs have many methods
   - Missing dependency graph between components
   - No guidance on split boundaries

**Real-World Impact**:
- Developers don't know where to start refactoring
- Split recommendations are generic ("core, I/O, utils")
- Unclear if problem is many small functions or few large ones
- Missing information about coupling between components

## Objective

Enhance god object reporting to provide detailed module structure analysis with accurate function counts, component breakdown, and visual structure to guide refactoring decisions.

## Requirements

### Functional Requirements

1. **Accurate Function Counting**
   - Distinguish between:
     - Module-level functions
     - Methods in impl blocks
     - Functions in nested modules
     - Private vs public functions
   - Report all categories separately in output
   - Total should match what developers expect

2. **Responsibility Detection**
   - Detect impl blocks and count methods per block
   - Identify struct/enum definitions
   - Detect trait implementations
   - Group related types and implementations
   - Calculate actual responsibility count (not 0!)

3. **Structure Visualization**
   - Show module hierarchy
   - Display impl blocks with method counts
   - Show struct definitions with field counts
   - Indicate public vs private API surface
   - Highlight largest components (top 5)

4. **Dependency Analysis**
   - Map which functions call which
   - Identify tightly coupled components
   - Detect standalone vs interconnected functions
   - Calculate coupling metrics per component
   - Suggest natural split boundaries

5. **Refactoring Guidance**
   - Specific recommendations based on actual structure
   - Identify low-coupling components (easy to extract)
   - Suggest split order (least coupled first)
   - Estimate refactoring effort per split
   - Show before/after file sizes

### Non-Functional Requirements

- Analysis adds <5% overhead to god object detection
- Structure visualization renders clearly in terminal
- Dependency analysis scales to 500+ function modules
- Works for Rust, Python, JavaScript, TypeScript
- Configurable detail level (summary vs detailed)

## Acceptance Criteria

- [ ] formatter.rs shows accurate breakdown: "58 module functions, 54 impl methods (total: 112)"
- [ ] Responsibility count is non-zero and reflects actual impl blocks/structs
- [ ] Output includes structure tree showing impl blocks and method counts
- [ ] Dependency analysis identifies tightly vs loosely coupled components
- [ ] Refactoring recommendations specific to actual structure (not generic)
- [ ] Largest components highlighted in output (top 5 by methods/size)
- [ ] Public vs private API clearly distinguished
- [ ] Works correctly for all supported languages
- [ ] Detail level configurable via `--verbosity` flag
- [ ] Performance impact <5% on large module analysis

## Technical Details

### Implementation Approach

**Phase 1: Enhanced Module Analysis**

Create `src/analysis/module_structure.rs`:

```rust
pub struct ModuleStructureAnalyzer {
    language: Language,
}

pub struct ModuleStructure {
    pub total_lines: usize,
    pub components: Vec<ModuleComponent>,
    pub function_counts: FunctionCounts,
    pub responsibility_count: usize,
    pub public_api_surface: usize,
    pub dependencies: ComponentDependencyGraph,
}

#[derive(Debug, Clone)]
pub struct FunctionCounts {
    pub module_level_functions: usize,
    pub impl_methods: usize,
    pub trait_methods: usize,
    pub nested_module_functions: usize,
    pub public_functions: usize,
    pub private_functions: usize,
}

impl FunctionCounts {
    pub fn total(&self) -> usize {
        self.module_level_functions +
        self.impl_methods +
        self.trait_methods +
        self.nested_module_functions
    }
}

#[derive(Debug, Clone)]
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

impl ModuleStructureAnalyzer {
    pub fn analyze_file(&self, path: &Path, ast: &Ast) -> ModuleStructure {
        let components = self.extract_components(ast);
        let function_counts = self.count_functions(&components);
        let responsibility_count = self.detect_responsibilities(&components);
        let public_api_surface = self.count_public_api(&components);
        let dependencies = self.analyze_dependencies(&components, ast);

        ModuleStructure {
            total_lines: ast.line_count(),
            components,
            function_counts,
            responsibility_count,
            public_api_surface,
            dependencies,
        }
    }

    fn detect_responsibilities(&self, components: &[ModuleComponent]) -> usize {
        // Count distinct responsibility areas:
        // 1. Each impl block = 1 responsibility
        // 2. Each struct/enum with methods = 1 responsibility
        // 3. Module-level functions grouped by prefix/domain
        let mut responsibilities = 0;

        for component in components {
            match component {
                ModuleComponent::ImplBlock { .. } => responsibilities += 1,
                ModuleComponent::Struct { methods, .. } if *methods > 0 => {
                    responsibilities += 1
                }
                ModuleComponent::Enum { methods, .. } if *methods > 0 => {
                    responsibilities += 1
                }
                _ => {}
            }
        }

        // Group module-level functions by naming patterns
        let function_groups = self.group_functions_by_domain(components);
        responsibilities += function_groups.len();

        responsibilities.max(1) // At least 1 responsibility
    }

    fn group_functions_by_domain(&self, components: &[ModuleComponent]) -> Vec<FunctionGroup> {
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();

        for component in components {
            if let ModuleComponent::ModuleLevelFunction { name, .. } = component {
                let prefix = self.extract_function_prefix(name);
                groups.entry(prefix).or_default().push(name.clone());
            }
        }

        groups.into_iter()
            .map(|(prefix, functions)| FunctionGroup { prefix, functions })
            .collect()
    }
}
```

**Phase 2: Dependency Graph Analysis**

```rust
pub struct ComponentDependencyGraph {
    pub components: Vec<String>,
    pub edges: Vec<(String, String)>,
    pub coupling_scores: HashMap<String, f64>,
}

impl ComponentDependencyGraph {
    pub fn identify_split_candidates(&self) -> Vec<SplitRecommendation> {
        // Find components with low coupling (easy to extract)
        let low_coupling: Vec<_> = self.coupling_scores.iter()
            .filter(|(_, score)| **score < 0.3)
            .map(|(component, score)| (component.clone(), *score))
            .collect();

        low_coupling.iter()
            .map(|(component, coupling)| SplitRecommendation {
                component: component.clone(),
                coupling_score: *coupling,
                suggested_module_name: self.suggest_module_name(component),
                estimated_lines: self.estimate_component_size(component),
                difficulty: if *coupling < 0.2 { Difficulty::Easy } else { Difficulty::Medium },
            })
            .collect()
    }

    pub fn analyze_coupling(&self) -> ComponentCouplingAnalysis {
        // Calculate afferent (incoming) and efferent (outgoing) coupling
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

#[derive(Debug)]
pub struct SplitRecommendation {
    pub component: String,
    pub coupling_score: f64,
    pub suggested_module_name: String,
    pub estimated_lines: usize,
    pub difficulty: Difficulty,
}

#[derive(Debug, Clone, Copy)]
pub enum Difficulty {
    Easy,    // Can extract with minimal changes
    Medium,  // Requires some interface changes
    Hard,    // Tightly coupled, complex extraction
}
```

**Phase 3: Enhanced Output Formatting**

Modify `src/priority/formatter.rs`:

```rust
fn format_god_object_details(&self, structure: &ModuleStructure) -> String {
    let mut output = String::new();

    // Header with accurate counts
    writeln!(
        &mut output,
        "├─ 📊 STRUCTURE: {} lines, {} components, {} responsibilities",
        structure.total_lines,
        structure.components.len(),
        structure.responsibility_count
    )?;

    // Function breakdown
    writeln!(
        &mut output,
        "├─ 🔢 FUNCTIONS: {} total",
        structure.function_counts.total()
    )?;
    writeln!(
        &mut output,
        "│  ├─ Module-level: {} ({} public, {} private)",
        structure.function_counts.module_level_functions,
        structure.function_counts.public_functions,
        structure.function_counts.private_functions
    )?;
    writeln!(
        &mut output,
        "│  ├─ Impl methods: {}",
        structure.function_counts.impl_methods
    )?;
    if structure.function_counts.trait_methods > 0 {
        writeln!(
            &mut output,
            "│  ├─ Trait methods: {}",
            structure.function_counts.trait_methods
        )?;
    }

    // Top 5 largest components
    writeln!(&mut output, "│")?;
    writeln!(&mut output, "├─ 📦 LARGEST COMPONENTS:")?;
    let top_components = self.get_top_components(&structure.components, 5);
    for (i, component) in top_components.iter().enumerate() {
        writeln!(
            &mut output,
            "│  {}. {} ({} methods, {} lines)",
            i + 1,
            component.name(),
            component.method_count(),
            component.line_count()
        )?;
    }

    // Coupling analysis
    writeln!(&mut output, "│")?;
    writeln!(&mut output, "├─ 🔗 COUPLING ANALYSIS:")?;
    let coupling_analysis = structure.dependencies.analyze_coupling();
    let highly_coupled: Vec<_> = coupling_analysis.efferent.iter()
        .filter(|(_, count)| **count > 10)
        .take(5)
        .collect();

    if !highly_coupled.is_empty() {
        writeln!(&mut output, "│  ├─ Highly coupled components:")?;
        for (component, count) in highly_coupled {
            writeln!(&mut output, "│  │  • {} → {} dependencies", component, count)?;
        }
    }

    // Split recommendations
    let split_candidates = structure.dependencies.identify_split_candidates();
    if !split_candidates.is_empty() {
        writeln!(&mut output, "│")?;
        writeln!(&mut output, "├─ 🎯 RECOMMENDED SPLITS:")?;
        for (i, candidate) in split_candidates.iter().take(5).enumerate() {
            let difficulty_emoji = match candidate.difficulty {
                Difficulty::Easy => "🟢",
                Difficulty::Medium => "🟡",
                Difficulty::Hard => "🔴",
            };
            writeln!(
                &mut output,
                "│  {}. {} Extract {} → {} ({} lines, coupling: {:.2})",
                i + 1,
                difficulty_emoji,
                candidate.component,
                candidate.suggested_module_name,
                candidate.estimated_lines,
                candidate.coupling_score
            )?;
        }
    }

    output
}
```

**Example Enhanced Output**:

```
#1 SCORE: 68.3 [CRITICAL - FILE - GOD OBJECT]
├─ src/priority/formatter.rs
├─ 📊 STRUCTURE: 2881 lines, 12 components, 5 responsibilities
├─ 🔢 FUNCTIONS: 112 total
│  ├─ Module-level: 58 (15 public, 43 private)
│  ├─ Impl methods: 54
│  ├─ Trait methods: 0
│
├─ 📦 LARGEST COMPONENTS:
│  1. OutputFormatter impl (24 methods, 856 lines)
│  2. ColoredFormatter impl (18 methods, 534 lines)
│  3. SeverityInfo struct (8 methods, 312 lines)
│  4. format_* functions (12 functions, 478 lines)
│  5. verbosity module (8 functions, 245 lines)
│
├─ 🔗 COUPLING ANALYSIS:
│  ├─ Highly coupled components:
│  │  • OutputFormatter → 23 dependencies
│  │  • format_debt_section → 15 dependencies
│
├─ 🎯 RECOMMENDED SPLITS:
│  1. 🟢 Extract verbosity module → formatter/verbosity.rs (245 lines, coupling: 0.15)
│  2. 🟢 Extract severity helpers → formatter/severity.rs (312 lines, coupling: 0.18)
│  3. 🟡 Extract section formatters → formatter/sections/ (856 lines, coupling: 0.35)
│  4. 🟡 Extract color utilities → formatter/colors.rs (234 lines, coupling: 0.42)
│  5. 🔴 Refactor OutputFormatter core → formatter/core.rs (534 lines, coupling: 0.68)
```

### Architecture Changes

**New Modules**:
- `src/analysis/module_structure.rs` - Module structure analysis
- `src/analysis/component_coupling.rs` - Dependency graph and coupling metrics
- `src/analysis/split_recommendations.rs` - Refactoring guidance generation

**Modified Modules**:
- `src/priority/formatter.rs` - Enhanced god object output
- `src/debt/god_object.rs` - Use ModuleStructure for analysis
- `src/config.rs` - Add verbosity levels for structure output

### Data Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodObjectAnalysis {
    pub file_path: PathBuf,
    pub score: f64,
    pub structure: ModuleStructure,
    pub split_recommendations: Vec<SplitRecommendation>,
}

#[derive(Debug, Clone)]
pub struct FunctionGroup {
    pub prefix: String,
    pub functions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ComponentCouplingAnalysis {
    pub afferent: HashMap<String, usize>,  // Incoming dependencies
    pub efferent: HashMap<String, usize>,  // Outgoing dependencies
    pub total_edges: usize,
}
```

### Configuration

```toml
[god_object.structure_analysis]
enabled = true
show_top_components = 5
show_coupling_details = true
suggest_splits = true
max_split_recommendations = 5

[god_object.verbosity]
# summary: Basic counts only
# detailed: Full structure tree
# comprehensive: Include all components + dependency graph
level = "detailed"
```

## Dependencies

- **Prerequisites**: Existing god object detection
- **Affected Components**:
  - `src/debt/god_object.rs` - Detection logic
  - `src/priority/formatter.rs` - Output formatting
  - Tree-sitter parsers for component extraction
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn accurately_counts_functions_in_formatter() {
    let ast = parse_rust_file("src/priority/formatter.rs");
    let analyzer = ModuleStructureAnalyzer::new(Language::Rust);
    let structure = analyzer.analyze_file(Path::new("formatter.rs"), &ast);

    // Validate against actual counts
    assert_eq!(structure.function_counts.module_level_functions, 58);
    assert!(structure.function_counts.impl_methods > 50);
    assert_eq!(structure.function_counts.total(), 112);
}

#[test]
fn detects_non_zero_responsibilities() {
    let ast = parse_rust_file("src/priority/formatter.rs");
    let analyzer = ModuleStructureAnalyzer::new(Language::Rust);
    let structure = analyzer.analyze_file(Path::new("formatter.rs"), &ast);

    assert!(structure.responsibility_count > 0);
    assert!(structure.responsibility_count >= 5, "Should detect multiple impl blocks");
}

#[test]
fn identifies_low_coupling_components() {
    let structure = create_test_module_structure();
    let split_candidates = structure.dependencies.identify_split_candidates();

    assert!(!split_candidates.is_empty());
    assert!(split_candidates[0].coupling_score < 0.3);
    assert_eq!(split_candidates[0].difficulty, Difficulty::Easy);
}
```

### Integration Tests

```rust
#[test]
fn end_to_end_enhanced_output() {
    let config = DebtmapConfig::default();
    let analysis = analyze_file("src/priority/formatter.rs", &config);

    let god_object = analysis.god_objects.first().unwrap();

    // Verify enhanced output includes structure details
    let output = format_god_object(god_object);
    assert!(output.contains("STRUCTURE:"));
    assert!(output.contains("FUNCTIONS:"));
    assert!(output.contains("LARGEST COMPONENTS:"));
    assert!(output.contains("RECOMMENDED SPLITS:"));

    // Verify accurate counts
    assert!(output.contains("112 total"));
    assert!(output.contains("responsibilities"));
    assert!(!output.contains("0 responsibilities"));
}
```

### Language-Specific Tests

```rust
#[test]
fn works_for_rust_modules() {
    test_module_analysis("tests/fixtures/rust/god_object.rs", Language::Rust);
}

#[test]
fn works_for_typescript_modules() {
    test_module_analysis("tests/fixtures/ts/god_object.ts", Language::TypeScript);
}

#[test]
fn works_for_python_modules() {
    test_module_analysis("tests/fixtures/python/god_object.py", Language::Python);
}
```

## Documentation Requirements

### User Documentation

Update README with enhanced output explanation:

```markdown
## God Object Analysis

When debtmap detects god objects (large modules with many functions), it provides:

1. **Accurate Function Counts**: Distinguishes module-level functions, impl methods, and nested modules
2. **Structure Visualization**: Shows largest components and their relationships
3. **Coupling Analysis**: Identifies tightly coupled vs standalone components
4. **Refactoring Guidance**: Specific split recommendations with difficulty estimates

**Example Output**:
```
#1 GOD OBJECT: src/priority/formatter.rs
├─ 📊 STRUCTURE: 2881 lines, 12 components, 5 responsibilities
├─ 🔢 FUNCTIONS: 112 total (58 module-level, 54 impl methods)
├─ 📦 LARGEST COMPONENTS:
│  1. OutputFormatter impl (24 methods, 856 lines)
│  ...
└─ 🎯 RECOMMENDED SPLITS:
   1. 🟢 Easy: Extract verbosity module (low coupling)
```

The 🟢/🟡/🔴 indicators show refactoring difficulty based on coupling.
```

### Architecture Documentation

```markdown
## Module Structure Analysis

### Component Detection

For each large module, debtmap extracts:
- Struct/enum definitions with field/variant counts
- Impl blocks with method counts
- Module-level function groups
- Public vs private API surface
- Nested sub-modules

### Coupling Metrics

Calculates afferent (incoming) and efferent (outgoing) coupling per component:
- **Afferent coupling**: How many components depend on this
- **Efferent coupling**: How many components this depends on
- **Coupling score**: Normalized metric (0.0-1.0)

Components with low coupling (<0.3) are easiest to extract.

### Split Recommendations

Ranks components by extraction ease:
- **Easy** (🟢): Coupling <0.2, standalone functionality
- **Medium** (🟡): Coupling 0.2-0.5, some interface changes needed
- **Hard** (🔴): Coupling >0.5, deeply integrated
```

## Implementation Notes

### Function Counting Algorithm

Must distinguish:
1. **Module-level functions**: `fn foo() { }` at module root
2. **Impl methods**: `impl Foo { fn bar() { } }`
3. **Trait methods**: `impl Trait for Foo { fn baz() { } }`
4. **Nested modules**: `mod sub { fn qux() { } }`

Use AST node type checking:
```rust
match node.kind() {
    "function_item" if is_module_level(node) => count_module_fn,
    "function_item" if is_in_impl(node) => count_impl_method,
    "function_item" if is_in_trait_impl(node) => count_trait_method,
    // ...
}
```

### Responsibility Detection Heuristics

Count as separate responsibility if:
- Each `impl` block (different types/traits)
- Each group of 5+ related module functions (same prefix)
- Each struct/enum with 3+ methods
- Each nested module with 5+ functions

### Coupling Calculation

Simplified coupling score:
```rust
coupling_score = efferent_coupling / (afferent_coupling + efferent_coupling + 1)
```

- 0.0 = No dependencies (easy to extract)
- 0.5 = Balanced dependencies (medium difficulty)
- 1.0 = Only outgoing dependencies (hard to extract)

## Migration and Compatibility

### Breaking Changes
None - this enhances existing output without changing API.

### Output Format Changes
- God object sections will be longer (more details)
- Add `--verbosity` flag to control detail level:
  - `--verbosity=0`: Summary only (current behavior)
  - `--verbosity=1`: Include structure (default)
  - `--verbosity=2`: Full coupling graph

### Performance Impact
- Structure analysis adds ~50-100ms per god object
- Only runs for modules >1000 lines (god object threshold)
- Overall impact <5% for typical codebases

## Success Metrics

- All god object reports show non-zero responsibilities
- Function counts match developer manual counts (±5%)
- Split recommendations actionable (developers actually use them)
- Users report clearer understanding of refactoring path
- Reduced "where do I start?" questions in feedback

## Future Enhancements

- **Interactive mode**: CLI tool to explore module structure
- **Visualization**: Generate GraphViz/Mermaid diagrams
- **Historical tracking**: Show coupling trends over time
- **Automated splitting**: Generate refactoring PRs
- **Semantic clustering**: ML-based component grouping
