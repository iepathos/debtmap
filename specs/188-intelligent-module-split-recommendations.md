---
number: 180
title: Intelligent Module Split Recommendations
category: optimization
priority: high
status: draft
dependencies: [178, 179]
created: 2025-11-18
---

# Specification 180: Intelligent Module Split Recommendations

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [178 - Behavioral Decomposition, 179 - Coupling Analysis]

## Context

Current module split recommendations for god objects suffer from several critical issues:

### Issue 1: Generic "Misc" Categories

**Example from Zed analysis**:
```
- config/misc.rs - misc (0 methods, ~423 lines) [Medium]
   -> Structs: ClipboardSelection, CompletionEdit, ... (45 structs)
```

**Problems**:
- "misc" is an anti-pattern - catch-all for things that don't fit elsewhere
- Recommended module still contains 45 structs - remains a god object
- No clear responsibility or purpose
- Developers won't know what belongs in "misc" going forward

### Issue 2: Insufficient Granularity

**Example**: Recommending 2-3 modules for a file with 675 methods:
```
RECOMMENDED SPLITS (2 modules):
  - config/misc.rs (423 lines, 45 structs)
  - config/core_config.rs (12 lines, 3 structs)
```

**Problems**:
- Moving from 1 god object to 1 large module + 1 tiny module
- Doesn't solve the fundamental complexity problem
- Imbalanced module sizes (423 lines vs 12 lines)
- No clear path to further decomposition

### Issue 3: Struct-Focused Instead of Behavior-Focused

Recommendations group structs rather than methods, ignoring that the real complexity is in the 675-method impl block, not the supporting data structures.

### Issue 4: No Validation of Module Quality

Recommendations don't verify that suggested modules:
- Have single, clear responsibility
- Maintain reasonable size (target: 200-400 lines, 10-30 methods)
- Have well-defined interfaces
- Follow naming conventions

## Objective

Generate high-quality module split recommendations that:
1. **Eliminate "misc" categories** - every module has a clear, specific responsibility
2. **Right-size modules** - target 10-30 methods, 200-400 lines each
3. **Behavior-first** - organize by method groups, not just struct groups
4. **Validate quality** - ensure recommended modules meet quality standards
5. **Actionable** - provide clear guidance on what belongs in each module

## Requirements

### Functional Requirements

1. **Anti-Pattern Detection**
   - Flag "misc", "utilities", "helpers", "common" as anti-pattern names
   - Require specific behavioral or domain names instead
   - Validate module names against quality criteria
   - Reject recommendations with generic names

2. **Module Sizing Guidelines**
   - Target: 10-30 methods per module (sweet spot: 15-20)
   - Target: 200-400 lines per module
   - Maximum: 50 methods or 600 lines per module
   - Minimum: 5 methods or 100 lines (below this, keep in parent)
   - Flag modules outside ranges as needing further decomposition

3. **Responsibility Clarity**
   - Each module must have single, clear responsibility
   - Derive module name from responsibility
   - Include responsibility statement in recommendation
   - Use domain language, not technical jargon when possible

4. **Balanced Decomposition**
   - Aim for relatively equal module sizes (no 423-line + 12-line splits)
   - If imbalance detected (>10x size difference), suggest further splits
   - Consider both method count and line count balance
   - Warn if one module is disproportionately large

5. **Module Naming Quality**
   - Use domain-driven names when possible
   - Use behavioral names (e.g., "rendering", "persistence")
   - Avoid technical jargon unless necessary
   - Follow language conventions (Rust: snake_case, etc.)
   - Names should be self-documenting

6. **Interface Definition**
   - For each recommended module, identify:
     - Public API methods (what other modules call)
     - Dependencies on other modules
     - Data that must be shared (fields, types)
   - Suggest trait extraction for well-defined interfaces
   - Flag tight coupling between proposed modules

7. **Incremental Decomposition Path**
   - If god object is extremely large (>500 methods):
     - Provide multi-level decomposition
     - First split: 5-8 coarse-grained modules
     - Then: Further split each into 3-5 fine-grained modules
   - Show decomposition tree with size at each level

### Non-Functional Requirements

1. **Quality**: No generic "misc" or "utilities" modules in output
2. **Clarity**: Module purpose obvious from name and description
3. **Practicality**: Recommendations implementable by human developers
4. **Consistency**: Follow project/language conventions
5. **Completeness**: Account for all methods/structs in original file

## Acceptance Criteria

- [ ] "misc", "utilities", "helpers", "common" never appear in module names
- [ ] Each recommended module has 10-50 methods (strict: reject <5 or >50)
- [ ] Module sizes are balanced (max/min ratio <5:1)
- [ ] Every module has clear responsibility statement
- [ ] Module names use domain or behavioral terminology
- [ ] Multi-level decomposition shown for very large god objects (>500 methods)
- [ ] For each module:
  - Method count estimate
  - Line count estimate
  - Representative method names (top 5-8)
  - Responsibility statement
  - Public interface description
- [ ] Validation warnings shown for:
  - Imbalanced splits (10x size difference)
  - Modules outside target ranges
  - Potential coupling issues
- [ ] When run on Zed editor.rs (675 methods):
  - Recommends 8-12 modules (not 2-3)
  - Each module 20-60 methods
  - No "misc" category
  - Clear behavioral or domain names
  - Balanced sizes

## Technical Details

### Implementation Approach

#### 1. Module Quality Validation

```rust
struct ModuleRecommendation {
    name: String,
    responsibility: String,
    methods: Vec<String>,
    line_count_estimate: usize,
    method_count: usize,
    public_interface: Vec<String>,
    quality_score: f64,
    warnings: Vec<String>,
}

impl ModuleRecommendation {
    fn validate(&mut self) {
        // Check for anti-pattern names
        if is_generic_name(&self.name) {
            self.warnings.push(format!(
                "Generic module name '{}' - specify concrete responsibility",
                self.name
            ));
            self.quality_score -= 0.3;
        }

        // Check size
        if self.method_count < 5 {
            self.warnings.push(format!(
                "Too small ({} methods) - consider keeping in parent module",
                self.method_count
            ));
            self.quality_score -= 0.2;
        } else if self.method_count > 50 {
            self.warnings.push(format!(
                "Too large ({} methods) - consider further decomposition",
                self.method_count
            ));
            self.quality_score -= 0.2;
        }

        // Check if name matches responsibility
        if !name_matches_responsibility(&self.name, &self.responsibility) {
            self.warnings.push("Module name doesn't reflect responsibility".to_string());
            self.quality_score -= 0.1;
        }
    }
}

fn is_generic_name(name: &str) -> bool {
    let generic_patterns = [
        "misc", "miscellaneous",
        "util", "utils", "utilities",
        "helper", "helpers",
        "common", "shared",
        "stuff", "things",
        "other", "extra",
        "base", "core" // unless very justified
    ];

    let normalized = name.to_lowercase();
    generic_patterns.iter().any(|pattern| {
        normalized.contains(pattern)
    })
}

fn name_matches_responsibility(name: &str, responsibility: &str) -> bool {
    // Extract key terms from responsibility
    let resp_terms: Vec<_> = responsibility
        .to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 3)  // Skip short words
        .collect();

    // Check if module name contains key terms
    let name_lower = name.to_lowercase();
    resp_terms.iter().any(|term| name_lower.contains(term))
}
```

#### 2. Balanced Decomposition

```rust
struct DecompositionPlan {
    levels: Vec<DecompositionLevel>,
    total_methods: usize,
    total_lines: usize,
}

struct DecompositionLevel {
    level: usize,
    modules: Vec<ModuleRecommendation>,
}

impl DecompositionPlan {
    fn validate_balance(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        for level in &self.levels {
            let sizes: Vec<_> = level.modules.iter()
                .map(|m| m.method_count)
                .collect();

            if let (Some(&max), Some(&min)) = (sizes.iter().max(), sizes.iter().min()) {
                let ratio = max as f64 / min.max(1) as f64;

                if ratio > 5.0 {
                    warnings.push(format!(
                        "Level {}: Imbalanced module sizes ({}:1 ratio) - largest: {} methods, smallest: {} methods",
                        level.level, ratio, max, min
                    ));
                }
            }
        }

        warnings
    }

    fn suggest_refinement(&self) -> Option<String> {
        // Find modules that are still too large
        let oversized: Vec<_> = self.levels.last()
            .unwrap()
            .modules.iter()
            .filter(|m| m.method_count > 50)
            .collect();

        if !oversized.is_empty() {
            Some(format!(
                "Consider further splitting: {} (still has {} methods)",
                oversized[0].name,
                oversized[0].method_count
            ))
        } else {
            None
        }
    }
}
```

#### 3. Multi-Level Decomposition

```rust
impl GodObjectAnalyzer {
    fn generate_decomposition_plan(
        &self,
        god_object: &GodObjectDetection,
        clusters: &[MethodCluster],  // From Spec 178
        coupling: &CouplingAnalysis,  // From Spec 179
    ) -> DecompositionPlan {
        let total_methods = god_object.method_count;

        if total_methods < 100 {
            // Single-level decomposition
            self.single_level_decomposition(clusters, coupling)
        } else if total_methods < 300 {
            // Two-level decomposition
            self.two_level_decomposition(clusters, coupling)
        } else {
            // Three-level decomposition for massive objects
            self.three_level_decomposition(clusters, coupling)
        }
    }

    fn single_level_decomposition(
        &self,
        clusters: &[MethodCluster],
        coupling: &CouplingAnalysis,
    ) -> DecompositionPlan {
        // Target: 5-8 modules, each with 10-30 methods
        let target_modules = 6;
        let modules = self.create_balanced_modules(clusters, target_modules);

        DecompositionPlan {
            levels: vec![DecompositionLevel { level: 1, modules }],
            total_methods: clusters.iter().map(|c| c.methods.len()).sum(),
            total_lines: 0,  // Estimated elsewhere
        }
    }

    fn two_level_decomposition(
        &self,
        clusters: &[MethodCluster],
        coupling: &CouplingAnalysis,
    ) -> DecompositionPlan {
        // Level 1: 6-10 coarse modules
        let coarse_modules = self.create_coarse_modules(clusters, 8);

        // Level 2: Each coarse module split into 2-4 fine modules
        let fine_modules = coarse_modules.iter()
            .flat_map(|m| self.further_split_module(m))
            .collect();

        DecompositionPlan {
            levels: vec![
                DecompositionLevel { level: 1, modules: coarse_modules },
                DecompositionLevel { level: 2, modules: fine_modules },
            ],
            total_methods: clusters.iter().map(|c| c.methods.len()).sum(),
            total_lines: 0,
        }
    }
}
```

#### 4. Smart Module Naming

```rust
struct ModuleNamer {
    domain_terms: HashMap<String, Vec<String>>,
    behavioral_patterns: Vec<(Regex, String)>,
}

impl ModuleNamer {
    fn generate_name(
        &self,
        cluster: &MethodCluster,
        context: &FileContext,
    ) -> (String, String) {  // (name, responsibility)
        // Try domain-driven naming first
        if let Some(domain_name) = self.infer_domain_name(cluster, context) {
            let responsibility = format!(
                "Handles {} operations",
                domain_name.replace('_', " ")
            );
            return (domain_name, responsibility);
        }

        // Fall back to behavioral naming
        let behavioral_name = self.infer_behavioral_name(cluster);
        let responsibility = self.generate_responsibility(cluster);

        (behavioral_name, responsibility)
    }

    fn infer_domain_name(
        &self,
        cluster: &MethodCluster,
        context: &FileContext,
    ) -> Option<String> {
        // Analyze method names for domain terms
        let terms = self.extract_domain_terms(&cluster.methods);

        // Check if domain terms are consistent
        if let Some(dominant_term) = self.find_dominant_term(&terms, 0.6) {
            return Some(dominant_term);
        }

        None
    }

    fn infer_behavioral_name(&self, cluster: &MethodCluster) -> String {
        // Use behavioral category from Spec 178
        match cluster.category {
            BehaviorCategory::Rendering => "rendering",
            BehaviorCategory::EventHandling => "event_handling",
            BehaviorCategory::Persistence => "persistence",
            BehaviorCategory::Validation => "validation",
            BehaviorCategory::Computation => "computation",
            BehaviorCategory::StateManagement => "state",
            BehaviorCategory::Lifecycle => "lifecycle",
            BehaviorCategory::Domain(ref name) => name,
        }.to_string()
    }

    fn generate_responsibility(&self, cluster: &MethodCluster) -> String {
        match cluster.category {
            BehaviorCategory::Rendering => {
                format!("Responsible for rendering and visual display ({} methods)", cluster.methods.len())
            },
            BehaviorCategory::EventHandling => {
                format!("Handles user input and events ({} methods)", cluster.methods.len())
            },
            BehaviorCategory::Persistence => {
                format!("Manages data persistence and serialization ({} methods)", cluster.methods.len())
            },
            BehaviorCategory::Validation => {
                format!("Validates data and business rules ({} methods)", cluster.methods.len())
            },
            BehaviorCategory::Computation => {
                format!("Performs calculations and transformations ({} methods)", cluster.methods.len())
            },
            BehaviorCategory::StateManagement => {
                format!("Manages internal state and data access ({} methods)", cluster.methods.len())
            },
            BehaviorCategory::Lifecycle => {
                format!("Handles object initialization and cleanup ({} methods)", cluster.methods.len())
            },
            BehaviorCategory::Domain(ref name) => {
                format!("Handles {} domain operations ({} methods)", name, cluster.methods.len())
            },
        }
    }

    fn extract_domain_terms(&self, methods: &[String]) -> HashMap<String, usize> {
        let mut terms = HashMap::new();

        for method in methods {
            // Extract nouns from method names (snake_case)
            let parts: Vec<_> = method.split('_').collect();

            for part in parts {
                // Skip common verbs and noise words
                if !self.is_noise_word(part) {
                    *terms.entry(part.to_string()).or_insert(0) += 1;
                }
            }
        }

        terms
    }

    fn find_dominant_term(&self, terms: &HashMap<String, usize>, threshold: f64) -> Option<String> {
        let total: usize = terms.values().sum();

        terms.iter()
            .filter(|(_, &count)| count as f64 / total as f64 >= threshold)
            .max_by_key(|(_, &count)| count)
            .map(|(term, _)| term.clone())
    }
}
```

### Enhanced Output Format

```
#1 SCORE: 25904 [CRITICAL]
└─ ./crates/editor/src/editor.rs (24902 lines, 1614 functions)
└─ WHY THIS MATTERS: This struct violates single responsibility principle...

└─ RECOMMENDED DECOMPOSITION (2 levels):

   LEVEL 1: Coarse-grained split (8 modules)
   ├─ editor_rendering.rs - Rendering & Display (85 methods, ~1700 lines)
   ├─ editor_events.rs - Event Handling (72 methods, ~1440 lines)
   ├─ editor_selection.rs - Selection Management (58 methods, ~1160 lines)
   ├─ editor_editing.rs - Text Editing Operations (64 methods, ~1280 lines)
   ├─ editor_navigation.rs - Navigation & Movement (43 methods, ~860 lines)
   ├─ editor_persistence.rs - Save/Load Operations (28 methods, ~560 lines)
   ├─ editor_diagnostics.rs - Diagnostics & Linting (35 methods, ~700 lines)
   └─ editor_core.rs - Core State & Lifecycle (290 methods, ~5800 lines)
                        ⚠️ Still large - see Level 2 refinement

   LEVEL 2: Fine-grained split (editor_core.rs → 5 modules)
   ├─ editor_state.rs - State Management (62 methods, ~1240 lines)
   ├─ editor_buffer.rs - Buffer Management (48 methods, ~960 lines)
   ├─ editor_display_map.rs - Display Mapping (54 methods, ~1080 lines)
   ├─ editor_configuration.rs - Settings & Config (38 methods, ~760 lines)
   └─ editor_lifecycle.rs - Init & Cleanup (18 methods, ~360 lines)

└─ DETAILED RECOMMENDATIONS:

   [1] editor_rendering.rs - Rendering & Display
   ├─ Responsibility: Render editor content, highlights, cursor, and UI elements
   ├─ Methods (85 total):
   │  ├─ render() - Main rendering entry point
   │  ├─ paint_text() - Text rendering
   │  ├─ paint_highlighted_ranges() - Syntax highlighting
   │  ├─ draw_cursor() - Cursor rendering
   │  ├─ render_gutter() - Line numbers and gutter
   │  ├─ paint_selections() - Selection highlights
   │  ├─ render_diagnostics() - Inline diagnostics
   │  └─ ... +78 more rendering methods
   ├─ Fields needed (8):
   │  ├─ display_map: Entity<DisplayMap>
   │  ├─ style: EditorStyle
   │  ├─ scroll_manager: ScrollAnchor
   │  ├─ gutter_dimensions: GutterDimensions
   │  └─ ... +4 more fields
   ├─ Public interface (12 methods):
   │  ├─ render(&mut self, cx: &mut ViewContext) -> impl Component
   │  ├─ invalidate_display(&mut self)
   │  └─ ... +10 more public methods
   ├─ Suggested extraction:
   │  └─ trait Render { fn render(...) -> impl Component; }
   │  └─ impl Render for Editor { /* 85 methods */ }
   └─ Quality: ✓ Clear responsibility, ✓ Good size, ✓ Well-defined interface

   [2] editor_events.rs - Event Handling
   ├─ Responsibility: Handle user input events and dispatch actions
   ├─ Methods (72 total):
   │  ├─ handle_keypress() - Keyboard input
   │  ├─ on_mouse_down() - Mouse events
   │  ├─ on_scroll() - Scroll events
   │  ├─ handle_input_event() - Generic input
   │  ├─ dispatch_action() - Action dispatch
   │  └─ ... +67 more event methods
   ├─ Fields needed (5):
   │  ├─ focus_handle: FocusHandle
   │  ├─ buffer: Entity<MultiBuffer>
   │  └─ ... +3 more fields
   ├─ Suggested extraction:
   │  └─ trait EventHandler { fn handle_input(...); }
   │  └─ impl EventHandler for Editor { /* 72 methods */ }
   └─ Quality: ✓ Clear responsibility, ✓ Good size, ✓ Cohesive behavior

   [3] editor_selection.rs - Selection Management
   ├─ Responsibility: Manage text selections and multi-cursor operations
   ├─ Methods (58 total):
   │  ├─ update_selections() - Update selection state
   │  ├─ move_cursor() - Cursor movement
   │  ├─ select_range() - Create selection
   │  ├─ add_selection() - Multi-cursor
   │  └─ ... +54 more selection methods
   ├─ Coupling analysis:
   │  ├─ High cohesion (0.82) - methods frequently call each other
   │  ├─ Medium external coupling - 15 callers from other modules
   │  └─ Contains 1 dependency cycle - requires careful extraction
   ├─ Suggested extraction:
   │  └─ struct SelectionManager { /* fields */ }
   │  └─ impl SelectionManager { /* 58 methods */ }
   │  └─ Editor stores: selection_manager: SelectionManager
   └─ Quality: ✓ Clear responsibility, ✓ Good size, ⚠️ Has dependency cycle

   ... [Remaining modules with similar detail]

└─ EXTRACTION ROADMAP:

   Phase 1: Extract low-coupling modules first (Weeks 1-2)
   ├─ [1] editor_persistence.rs (28 methods, coupling: LOW)
   ├─ [2] editor_diagnostics.rs (35 methods, coupling: LOW)
   └─ [3] editor_navigation.rs (43 methods, coupling: MEDIUM)

   Phase 2: Extract behavioral modules (Weeks 3-4)
   ├─ [4] editor_rendering.rs (85 methods, coupling: MEDIUM)
   └─ [5] editor_events.rs (72 methods, coupling: MEDIUM)

   Phase 3: Refactor core state (Weeks 5-6)
   ├─ [6] Break dependency cycles in selection management
   ├─ [7] Extract editor_selection.rs (58 methods, coupling: HIGH)
   └─ [8] Extract editor_editing.rs (64 methods, coupling: HIGH)

   Phase 4: Final decomposition (Week 7)
   ├─ [9] Split editor_core.rs into 5 modules (Level 2 decomposition)
   └─ [10] Integration testing and refinement

└─ VALIDATION:

   ✓ No "misc" or "utilities" modules
   ✓ All modules 18-85 methods (target: 10-50)
   ✓ Balanced sizes (max/min ratio: 4.7:1 - acceptable)
   ✓ Clear behavioral or domain names
   ✓ All modules have specific responsibilities
   ⚠️ Warning: editor_rendering.rs is 85 methods (above target 50)
   ⚠️ Warning: 1 dependency cycle detected in selection module
   ✓ All methods accounted for (675 total)
```

## Dependencies

- **Prerequisites**:
  - [178] Behavioral decomposition recommendations
  - [179] Coupling and dependency analysis
- **Affected Components**:
  - `recommendations/god_object.rs` - Module recommendation generation
  - Module quality validation logic
  - Output formatting
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

1. **Anti-pattern detection**:
   - Test `is_generic_name()` with "misc", "utils", "helpers"
   - Verify rejection of generic names
   - Test with acceptable names

2. **Module sizing**:
   - Test validation of too-small modules (<5 methods)
   - Test validation of too-large modules (>50 methods)
   - Test balanced decomposition algorithm

3. **Naming quality**:
   - Test domain term extraction from method names
   - Test behavioral name inference
   - Test responsibility generation

4. **Multi-level decomposition**:
   - Test single-level for small god objects
   - Test two-level for medium god objects
   - Test three-level for massive god objects

### Integration Tests

1. **Real codebases**:
   - Analyze Zed editor.rs - verify no "misc" modules
   - Verify module counts (8-12 recommended)
   - Verify balanced sizes
   - Verify clear responsibilities

2. **Quality validation**:
   - Run on multiple god objects
   - Verify quality scores >0.7 for all modules
   - Verify no warnings for well-balanced decompositions
   - Verify multi-level shown for very large files

3. **Output format**:
   - Verify detailed recommendations match template
   - Verify validation warnings shown
   - Verify extraction roadmap generated

### Manual Review

- Human expert reviews recommendations for 5 real god objects
- Verify module names are intuitive
- Verify responsibilities are clear
- Verify recommendations are actionable

## Documentation Requirements

### Code Documentation

1. Document module quality criteria and thresholds
2. Document anti-pattern detection logic
3. Document balanced decomposition algorithm
4. Add examples of good vs bad module recommendations

### User Documentation

1. Update README with examples of improved recommendations
2. Add guide on interpreting multi-level decomposition
3. Explain module quality validation
4. Show before/after of enhanced output

### Architecture Updates

1. Document module recommendation subsystem
2. Explain integration with behavioral decomposition and coupling
3. Document quality scoring algorithm

## Implementation Notes

### Module Size Guidelines

**Optimal ranges** (empirically derived):
- **Methods**: 10-30 per module (sweet spot: 15-20)
- **Lines**: 200-400 per module (sweet spot: 250-350)
- **Acceptable**: 5-50 methods, 100-600 lines
- **Reject**: <5 methods (too small), >50 methods (still too large)

**Balance guidelines**:
- Max/min ratio should be <5:1 (ideally <3:1)
- Warn if any module is >2x average size
- Suggest further splitting if module >50 methods

### Anti-Pattern Names to Reject

```rust
const ANTI_PATTERN_NAMES: &[&str] = &[
    "misc", "miscellaneous",
    "util", "utils", "utilities", "utility",
    "helper", "helpers",
    "common", "shared",
    "stuff", "things", "other", "extra",
    "base",  // unless very justified
    "core",  // unless very justified
    "lib",
    "functions",
    "methods",
];
```

### Quality Scoring Formula

```rust
fn calculate_quality_score(module: &ModuleRecommendation) -> f64 {
    let mut score = 1.0;

    // Penalize generic names
    if is_generic_name(&module.name) {
        score -= 0.3;
    }

    // Penalize size issues
    if module.method_count < 5 || module.method_count > 50 {
        score -= 0.2;
    }

    // Reward clear responsibility
    if has_specific_responsibility(&module.responsibility) {
        score += 0.1;
    }

    // Penalize if name doesn't match responsibility
    if !name_matches_responsibility(&module.name, &module.responsibility) {
        score -= 0.1;
    }

    // Reward well-defined interface
    if module.public_interface.len() >= 3 && module.public_interface.len() <= 10 {
        score += 0.1;
    }

    score.max(0.0).min(1.0)
}
```

### Integration with Previous Specs

This spec builds on:
- **Spec 178**: Uses behavioral clusters for module grouping
- **Spec 179**: Uses coupling analysis for extraction order

Combined workflow:
1. Detect god object
2. Perform behavioral clustering (Spec 178)
3. Analyze coupling (Spec 179)
4. Generate module recommendations (This spec)
5. Validate module quality (This spec)
6. Output enhanced recommendations

## Migration and Compatibility

### Breaking Changes

- None - enhances existing recommendations

### Backwards Compatibility

- Existing detection unchanged
- Can add flag `--legacy-recommendations` to use old format
- New format is default

### Configuration

```toml
[recommendations.module_splits]
reject_generic_names = true
min_methods_per_module = 5
max_methods_per_module = 50
target_methods_per_module = 20
max_size_ratio = 5.0  # Max/min size ratio
multi_level_threshold = 200  # Methods threshold for multi-level
```

## Success Metrics

- Zero "misc" or "utilities" modules in recommendations
- Average module quality score >0.75
- Module count increases: was 2-3, now 8-12 for large god objects
- User feedback: recommendations are more actionable
- Module size variance decreases (more balanced)
- All modules have specific, clear responsibilities

## Future Enhancements

1. **Machine learning**: Learn optimal module groupings from successful refactorings
2. **Interactive refinement**: Allow user to tweak module boundaries
3. **Code generation**: Generate skeleton code for extracted modules
4. **Progress tracking**: Track partial refactorings over time
5. **Best practices**: Suggest design patterns for extracted modules
