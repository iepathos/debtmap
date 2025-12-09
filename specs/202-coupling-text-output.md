---
number: 202
title: Coupling Metrics in Text Output
category: foundation
priority: high
status: draft
dependencies: [201]
created: 2025-12-09
---

# Specification 202: Coupling Metrics in Text Output

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: [201 - File-Level Dependency Metrics]

## Context

Spec 201 defines the data structures for file-level dependency metrics. This spec focuses on **displaying those metrics in text/markdown output** in a compact, scannable format.

**Current text output** for files shows:
```
#1 SCORE: 89.8 [HIGH]
   src/builders/unified_analysis.rs
   └─ METRICS: Methods: 44, Lines: 2061, Responsibilities: 9
   └─ RECOMMENDATION: Split by data flow...
```

**Missing**: No coupling information at all.

**Desired output**:
```
#1 SCORE: 89.8 [HIGH]
   src/builders/unified_analysis.rs
   └─ METRICS: Methods: 44, Lines: 2061, Responsibilities: 9
   └─ COUPLING: Ca=12 (stable core), Ce=8, I=0.40
      ← main.rs, lib.rs, commands/analyze.rs (+9 more)
      → std::collections, serde, crate::core (+5 more)
   └─ RECOMMENDATION: Split by data flow...
```

## Objective

Add a COUPLING section to text/markdown file output that displays:
1. Afferent coupling (Ca) with classification label
2. Efferent coupling (Ce)
3. Instability metric (I)
4. Top dependents (← arrows, incoming)
5. Top dependencies (→ arrows, outgoing)

## Requirements

### Functional Requirements

1. **Coupling Summary Line**
   - Format: `COUPLING: Ca={N} ({classification}), Ce={N}, I={0.XX}`
   - Classification labels: "stable core", "utility", "leaf", "isolated", "highly coupled"
   - Only show for files with non-zero coupling

2. **Dependents List (Incoming)**
   - Format: `← file1.rs, file2.rs, file3.rs (+N more)`
   - Show top 3 dependents by default
   - Use `←` arrow to indicate "these depend on me"
   - Truncate long paths to filename only

3. **Dependencies List (Outgoing)**
   - Format: `→ module1, module2, module3 (+N more)`
   - Show top 3 dependencies by default
   - Use `→` arrow to indicate "I depend on these"
   - Group external crates vs internal modules

4. **Conditional Display**
   - Skip coupling section if total coupling (Ca + Ce) < 2
   - Skip dependents line if Ca = 0
   - Skip dependencies line if Ce = 0

5. **Integration with Existing Output**
   - Insert after METRICS line
   - Before RECOMMENDATION line
   - Use same indentation and formatting style

### Non-Functional Requirements

1. **Readability**: Coupling info should be scannable at a glance
2. **Compactness**: Keep to 2-3 lines maximum
3. **Consistency**: Match existing text output formatting conventions

## Acceptance Criteria

- [ ] File items with coupling > 1 show COUPLING line in text output
- [ ] Classification label matches coupling profile (stable/utility/leaf/etc.)
- [ ] Dependents list uses ← arrow prefix
- [ ] Dependencies list uses → arrow prefix
- [ ] Lists truncate with (+N more) when exceeding 3 items
- [ ] Coupling section appears between METRICS and RECOMMENDATION
- [ ] JSON output (unchanged) still works as before
- [ ] Markdown output includes same coupling information

## Technical Details

### Implementation Approach

Update `src/io/writers/` text formatters:

```rust
// In text/markdown formatter
fn format_file_coupling(deps: &FileDependencies) -> Vec<String> {
    let mut lines = Vec::new();

    // Main coupling line
    let classification = classify_coupling(deps.afferent_coupling, deps.efferent_coupling);
    lines.push(format!(
        "└─ COUPLING: Ca={} ({}), Ce={}, I={:.2}",
        deps.afferent_coupling,
        classification,
        deps.efferent_coupling,
        deps.instability
    ));

    // Dependents (incoming)
    if !deps.top_dependents.is_empty() {
        let display = format_truncated_list(&deps.top_dependents, 3);
        lines.push(format!("   ← {}", display));
    }

    // Dependencies (outgoing)
    if !deps.top_dependencies.is_empty() {
        let display = format_truncated_list(&deps.top_dependencies, 3);
        lines.push(format!("   → {}", display));
    }

    lines
}

fn format_truncated_list(items: &[String], max: usize) -> String {
    if items.len() <= max {
        items.join(", ")
    } else {
        let shown: Vec<_> = items.iter().take(max).collect();
        format!("{} (+{} more)", shown.join(", "), items.len() - max)
    }
}

fn classify_coupling(ca: usize, ce: usize) -> &'static str {
    let total = ca + ce;
    let instability = if total > 0 { ce as f64 / total as f64 } else { 0.0 };

    match (ca, ce, instability) {
        (_, _, _) if total < 3 => "isolated",
        (ca, _, i) if i < 0.3 && ca > 5 => "stable core",
        (_, _, i) if i > 0.7 => "leaf",
        (ca, ce, _) if ca > 15 || ce > 15 => "highly coupled",
        _ => "utility",
    }
}
```

### Affected Components

- `src/io/writers/enhanced_markdown/debt_writer.rs`
- `src/io/writers/markdown/enhanced.rs`
- `src/priority/formatter/mod.rs` (text output)
- `src/priority/formatter_verbosity/` (verbosity levels)

### Output Examples

**Stable Core Module** (high Ca, low Ce):
```
└─ COUPLING: Ca=25 (stable core), Ce=3, I=0.11
   ← main.rs, lib.rs, commands/analyze.rs (+22 more)
   → std::path, anyhow
```

**Leaf Module** (low Ca, high Ce):
```
└─ COUPLING: Ca=2 (leaf), Ce=12, I=0.86
   ← tests/integration.rs, benches/perf.rs
   → serde, tokio, reqwest (+9 more)
```

**Highly Coupled** (warning case):
```
└─ COUPLING: Ca=18 (highly coupled), Ce=15, I=0.45
   ← [18 files depend on this]
   → [15 external dependencies]
```

## Dependencies

- **Prerequisites**: Spec 201 (File-Level Dependency Metrics) must be implemented first
- **Affected Components**: Text/markdown formatters
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Test `format_truncated_list`, `classify_coupling` functions
- **Integration Tests**: Verify full text output includes coupling section
- **Snapshot Tests**: Compare output format against expected samples

## Documentation Requirements

- **User Documentation**: Update output format documentation
- **Examples**: Show coupling section in sample outputs

## Implementation Notes

1. Start with enhanced markdown writer, then backport to plain text
2. Consider verbosity levels: brief (just Ca/Ce), normal (with lists), verbose (full paths)
3. Use consistent arrow symbols across platforms (← →)

## Migration and Compatibility

- Additive change to text output
- No breaking changes to existing parsers
- JSON format unchanged (covered by Spec 201)
