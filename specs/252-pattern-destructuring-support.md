---
number: 252
title: Pattern Destructuring Support for CFG
category: foundation
priority: high
status: draft
dependencies: [248]
created: 2025-12-12
---

# Specification 252: Pattern Destructuring Support for CFG

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 248 (Enhanced Expression Variable Extraction)

## Context

### Current Problem

The `process_local` function in CFG construction (`src/analysis/data_flow.rs:1156-1167`) only handles simple identifier patterns:

```rust
fn process_local(&mut self, local: &Local) {
    if let Pat::Ident(pat_ident) = &local.pat {
        let var = self.get_or_create_var(&pat_ident.ident.to_string());
        let init = local.init.as_ref().map(|_init| Rvalue::Constant);
        // ...
    }
    // All other patterns silently ignored!
}
```

This means the following common Rust patterns are **completely ignored**:

```rust
// Tuple destructuring - NOT TRACKED
let (a, b) = get_pair();

// Struct destructuring - NOT TRACKED
let Point { x, y } = get_point();

// Slice destructuring - NOT TRACKED
let [first, second, rest @ ..] = get_array();

// Nested patterns - NOT TRACKED
let Some((key, value)) = map.iter().next();

// Reference patterns - NOT TRACKED
let &x = some_ref;
```

### Impact

Without pattern destructuring support:
1. **Variable coverage drops**: ~40% of variables in typical Rust code use patterns
2. **Data flow breaks**: Can't track data from source to destructured variables
3. **Def-use chains incomplete**: Uses of pattern-bound variables have no definitions
4. **Taint analysis gaps**: Taint doesn't propagate to destructured bindings

### Common Patterns Affected

| Pattern | Usage | Currently Tracked |
|---------|-------|-------------------|
| `let x = value` | 60% | Yes |
| `let (a, b) = tuple` | 15% | No |
| `let Point { x, y } = struct` | 10% | No |
| `let Some(x) = option` | 8% | No |
| `let [a, b] = array` | 5% | No |
| `let &x = ref` | 2% | No |

## Objective

Extend CFG construction to handle all Rust pattern forms, creating appropriate variable bindings for each pattern element.

## Requirements

### Functional Requirements

1. **Tuple Pattern Support**
   - `let (a, b, c) = tuple`
   - Track each element as separate VarId
   - Link to tuple source for data flow

2. **Struct Pattern Support**
   - `let Point { x, y } = point`
   - `let Point { x: my_x, y: my_y } = point` (renaming)
   - `let Point { x, .. } = point` (partial)
   - Track each field binding as VarId

3. **TupleStruct Pattern Support**
   - `let Some(value) = option`
   - `let Ok(data) = result`
   - Track inner bindings

4. **Slice Pattern Support**
   - `let [first, second] = array`
   - `let [first, rest @ ..] = array`
   - Track each element binding

5. **Reference Pattern Support**
   - `let &x = some_ref`
   - `let &mut x = some_mut_ref`
   - Track dereferenced binding

6. **Nested Pattern Support**
   - `let (a, (b, c)) = nested`
   - `let Some(Point { x, y }) = opt_point`
   - Recursively process nested patterns

7. **Or Pattern Support**
   - `let Ok(v) | Err(v) = result` (same binding in each arm)
   - Take bindings from first arm (all arms bind same names)

### Non-Functional Requirements

- **Performance**: <0.1ms additional overhead per pattern
- **Completeness**: Support all syn::Pat variants
- **Accuracy**: Each binding tracked as separate VarId

## Acceptance Criteria

- [ ] Tuple patterns create VarId for each element
- [ ] Struct patterns create VarId for each field binding
- [ ] TupleStruct patterns (Some, Ok, etc.) handled
- [ ] Slice patterns create VarId for each element
- [ ] Reference patterns handled correctly
- [ ] Nested patterns recursively processed
- [ ] Or patterns extract common bindings
- [ ] Each binding linked to source for data flow
- [ ] Def-use chains correct for pattern bindings
- [ ] All existing tests pass
- [ ] Pattern coverage >95%

## Technical Details

### Implementation Approach

#### Phase 1: Pattern Binding Extractor

```rust
/// A binding extracted from a pattern.
#[derive(Debug, Clone)]
pub struct PatternBinding {
    /// The bound variable name
    pub name: String,
    /// Optional field path from source (for struct/tuple access)
    pub access_path: Option<AccessPath>,
    /// Whether binding is mutable
    pub is_mut: bool,
    /// Whether binding is by reference
    pub by_ref: bool,
}

/// Path to access a field in the source expression.
#[derive(Debug, Clone)]
pub enum AccessPath {
    /// Tuple index: .0, .1, .2
    TupleIndex(usize),
    /// Named field: .field_name
    NamedField(String),
    /// Array index: [0], [1]
    ArrayIndex(usize),
    /// Nested access: .0.field.1
    Nested(Vec<AccessPath>),
    /// Slice rest: [..]
    SliceRest(usize), // start index
}

impl CfgBuilder {
    /// Extract all variable bindings from a pattern.
    fn extract_pattern_bindings(&self, pat: &Pat) -> Vec<PatternBinding> {
        self.extract_pattern_bindings_with_path(pat, None)
    }

    fn extract_pattern_bindings_with_path(
        &self,
        pat: &Pat,
        parent_path: Option<AccessPath>,
    ) -> Vec<PatternBinding> {
        match pat {
            // Simple identifier: let x = ...
            Pat::Ident(pat_ident) => {
                vec![PatternBinding {
                    name: pat_ident.ident.to_string(),
                    access_path: parent_path,
                    is_mut: pat_ident.mutability.is_some(),
                    by_ref: pat_ident.by_ref.is_some(),
                }]
            }

            // Tuple: let (a, b, c) = ...
            Pat::Tuple(tuple) => {
                tuple.elems.iter().enumerate()
                    .flat_map(|(i, elem)| {
                        let path = Self::append_path(parent_path.clone(), AccessPath::TupleIndex(i));
                        self.extract_pattern_bindings_with_path(elem, Some(path))
                    })
                    .collect()
            }

            // Struct: let Point { x, y } = ...
            Pat::Struct(pat_struct) => {
                pat_struct.fields.iter()
                    .flat_map(|field| {
                        let field_name = match &field.member {
                            syn::Member::Named(ident) => ident.to_string(),
                            syn::Member::Unnamed(idx) => idx.index.to_string(),
                        };
                        let path = Self::append_path(
                            parent_path.clone(),
                            AccessPath::NamedField(field_name),
                        );
                        self.extract_pattern_bindings_with_path(&field.pat, Some(path))
                    })
                    .collect()
            }

            // TupleStruct: let Some(x) = ..., let Ok(v) = ...
            Pat::TupleStruct(tuple_struct) => {
                tuple_struct.elems.iter().enumerate()
                    .flat_map(|(i, elem)| {
                        let path = Self::append_path(parent_path.clone(), AccessPath::TupleIndex(i));
                        self.extract_pattern_bindings_with_path(elem, Some(path))
                    })
                    .collect()
            }

            // Slice: let [first, second, rest @ ..] = ...
            Pat::Slice(slice) => {
                slice.elems.iter().enumerate()
                    .flat_map(|(i, elem)| {
                        let path = Self::append_path(parent_path.clone(), AccessPath::ArrayIndex(i));
                        self.extract_pattern_bindings_with_path(elem, Some(path))
                    })
                    .collect()
            }

            // Reference: let &x = ... or let &mut x = ...
            Pat::Reference(reference) => {
                // The inner pattern binds to the dereferenced value
                self.extract_pattern_bindings_with_path(&reference.pat, parent_path)
            }

            // Box: let box x = ...
            Pat::Box(pat_box) => {
                self.extract_pattern_bindings_with_path(&pat_box.pat, parent_path)
            }

            // Or: let Ok(x) | Err(x) = ... (all branches bind same names)
            Pat::Or(or) => {
                // Take bindings from first case (all cases should bind same vars)
                or.cases.first()
                    .map(|p| self.extract_pattern_bindings_with_path(p, parent_path))
                    .unwrap_or_default()
            }

            // Type annotation: let x: T = ...
            Pat::Type(pat_type) => {
                self.extract_pattern_bindings_with_path(&pat_type.pat, parent_path)
            }

            // Paren: let (x) = ... (just wrapping)
            Pat::Paren(paren) => {
                self.extract_pattern_bindings_with_path(&paren.pat, parent_path)
            }

            // Rest: .. (in slices, doesn't bind a variable unless named)
            Pat::Rest(_) => vec![],

            // Wildcard: let _ = ... (no binding)
            Pat::Wild(_) => vec![],

            // Literal: match arm literal, no binding
            Pat::Lit(_) => vec![],

            // Range: match arm range, no binding
            Pat::Range(_) => vec![],

            // Path: match arm path (enum variant without data), no binding
            Pat::Path(_) => vec![],

            // Macro: can't analyze, skip
            Pat::Macro(_) => vec![],

            // Verbatim: raw tokens, skip
            Pat::Verbatim(_) => vec![],

            _ => vec![],
        }
    }

    fn append_path(parent: Option<AccessPath>, child: AccessPath) -> AccessPath {
        match parent {
            Some(AccessPath::Nested(mut vec)) => {
                vec.push(child);
                AccessPath::Nested(vec)
            }
            Some(other) => AccessPath::Nested(vec![other, child]),
            None => child,
        }
    }
}
```

#### Phase 2: Updated process_local

```rust
impl CfgBuilder {
    fn process_local(&mut self, local: &Local) {
        // Extract all bindings from the pattern
        let bindings = self.extract_pattern_bindings(&local.pat);

        if bindings.is_empty() {
            return; // Wildcard or unsupported pattern
        }

        // Get source Rvalue from initializer
        let source_rvalue = local.init.as_ref()
            .map(|init| self.expr_to_rvalue(&init.expr));

        // Get source variables for data flow tracking
        let source_vars = local.init.as_ref()
            .map(|init| self.extract_vars_from_expr(&init.expr))
            .unwrap_or_default();

        // Create declaration for each binding
        for binding in bindings {
            let var = self.get_or_create_var(&binding.name);

            // Create appropriate Rvalue based on access path
            let init_rvalue = match (&source_rvalue, &binding.access_path) {
                (Some(src), Some(path)) => {
                    // Field/element access from source
                    Some(self.create_access_rvalue(src, path, &source_vars))
                }
                (Some(src), None) => {
                    // Simple binding, use source directly
                    Some(src.clone())
                }
                (None, _) => None,
            };

            self.current_block.push(Statement::Declare {
                var,
                init: init_rvalue,
                line: self.get_span_line(local),
            });
        }
    }

    /// Create an Rvalue representing field/element access.
    fn create_access_rvalue(
        &self,
        source: &Rvalue,
        path: &AccessPath,
        source_vars: &[VarId],
    ) -> Rvalue {
        // Get base variable from source
        let base_var = match source {
            Rvalue::Use(var) => Some(*var),
            _ => source_vars.first().copied(),
        };

        match (base_var, path) {
            (Some(base), AccessPath::TupleIndex(idx)) => {
                Rvalue::FieldAccess {
                    base,
                    field: idx.to_string(),
                }
            }
            (Some(base), AccessPath::NamedField(name)) => {
                Rvalue::FieldAccess {
                    base,
                    field: name.clone(),
                }
            }
            (Some(base), AccessPath::ArrayIndex(idx)) => {
                Rvalue::FieldAccess {
                    base,
                    field: format!("[{}]", idx),
                }
            }
            (Some(base), AccessPath::Nested(paths)) => {
                // For nested access, use the first path element
                // Full path tracking would require enhanced Rvalue
                if let Some(first) = paths.first() {
                    self.create_access_rvalue(source, first, source_vars)
                } else {
                    Rvalue::Use(base)
                }
            }
            _ => source.clone(),
        }
    }
}
```

#### Phase 3: Match Pattern Integration

For match arms, we also need pattern support:

```rust
impl CfgBuilder {
    /// Process a match expression pattern, binding variables from scrutinee.
    fn bind_pattern_vars(&mut self, pat: &Pat, scrutinee: VarId) {
        let bindings = self.extract_pattern_bindings(pat);

        for binding in bindings {
            let var = self.get_or_create_var(&binding.name);

            let init = match &binding.access_path {
                Some(AccessPath::TupleIndex(idx)) => Rvalue::FieldAccess {
                    base: scrutinee,
                    field: idx.to_string(),
                },
                Some(AccessPath::NamedField(name)) => Rvalue::FieldAccess {
                    base: scrutinee,
                    field: name.clone(),
                },
                Some(AccessPath::ArrayIndex(idx)) => Rvalue::FieldAccess {
                    base: scrutinee,
                    field: format!("[{}]", idx),
                },
                _ => Rvalue::Use(scrutinee),
            };

            self.current_block.push(Statement::Declare {
                var,
                init: Some(init),
                line: None, // Pattern doesn't have its own span easily
            });
        }
    }
}
```

### Architecture Changes

1. **New types**: `PatternBinding`, `AccessPath`
2. **New methods**: `extract_pattern_bindings`, `create_access_rvalue`, `bind_pattern_vars`
3. **Modified methods**: `process_local` expanded significantly

### Data Structures

```rust
pub struct PatternBinding {
    pub name: String,
    pub access_path: Option<AccessPath>,
    pub is_mut: bool,
    pub by_ref: bool,
}

pub enum AccessPath {
    TupleIndex(usize),
    NamedField(String),
    ArrayIndex(usize),
    Nested(Vec<AccessPath>),
    SliceRest(usize),
}
```

### APIs and Interfaces

Internal changes only. Public API unchanged.

## Dependencies

- **Prerequisites**: Spec 248 (Enhanced Expression Variable Extraction)
- **Related**: Spec 253 (Match Expression CFG) uses pattern binding
- **Affected Components**: `src/analysis/data_flow.rs`
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod pattern_tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_tuple_destructuring() {
        let block: Block = parse_quote!({
            let (a, b) = get_pair();
            a + b
        });

        let cfg = ControlFlowGraph::from_block(&block);

        // Both a and b should be tracked
        assert!(cfg.var_names.contains(&"a".to_string()));
        assert!(cfg.var_names.contains(&"b".to_string()));
    }

    #[test]
    fn test_struct_destructuring() {
        let block: Block = parse_quote!({
            let Point { x, y } = get_point();
            x * y
        });

        let cfg = ControlFlowGraph::from_block(&block);

        assert!(cfg.var_names.contains(&"x".to_string()));
        assert!(cfg.var_names.contains(&"y".to_string()));
    }

    #[test]
    fn test_struct_destructuring_with_rename() {
        let block: Block = parse_quote!({
            let Point { x: my_x, y: my_y } = get_point();
            my_x + my_y
        });

        let cfg = ControlFlowGraph::from_block(&block);

        assert!(cfg.var_names.contains(&"my_x".to_string()));
        assert!(cfg.var_names.contains(&"my_y".to_string()));
    }

    #[test]
    fn test_option_destructuring() {
        let block: Block = parse_quote!({
            let Some(value) = maybe_value else { return };
            value
        });

        let cfg = ControlFlowGraph::from_block(&block);

        assert!(cfg.var_names.contains(&"value".to_string()));
    }

    #[test]
    fn test_nested_destructuring() {
        let block: Block = parse_quote!({
            let (a, (b, c)) = get_nested();
            a + b + c
        });

        let cfg = ControlFlowGraph::from_block(&block);

        assert!(cfg.var_names.contains(&"a".to_string()));
        assert!(cfg.var_names.contains(&"b".to_string()));
        assert!(cfg.var_names.contains(&"c".to_string()));
    }

    #[test]
    fn test_slice_destructuring() {
        let block: Block = parse_quote!({
            let [first, second] = arr;
            first + second
        });

        let cfg = ControlFlowGraph::from_block(&block);

        assert!(cfg.var_names.contains(&"first".to_string()));
        assert!(cfg.var_names.contains(&"second".to_string()));
    }

    #[test]
    fn test_reference_pattern() {
        let block: Block = parse_quote!({
            let &x = some_ref;
            x
        });

        let cfg = ControlFlowGraph::from_block(&block);

        assert!(cfg.var_names.contains(&"x".to_string()));
    }

    #[test]
    fn test_or_pattern() {
        let block: Block = parse_quote!({
            let Ok(v) | Err(v) = result;
            v
        });

        let cfg = ControlFlowGraph::from_block(&block);

        // v should be tracked (same name in both arms)
        assert!(cfg.var_names.contains(&"v".to_string()));
    }

    #[test]
    fn test_wildcard_pattern_no_binding() {
        let block: Block = parse_quote!({
            let _ = compute_and_discard();
        });

        let cfg = ControlFlowGraph::from_block(&block);

        // No new variables should be tracked
        let user_vars: Vec<_> = cfg.var_names.iter()
            .filter(|n| !n.starts_with("_temp"))
            .collect();
        assert!(user_vars.is_empty());
    }

    #[test]
    fn test_data_flow_through_pattern() {
        let block: Block = parse_quote!({
            let source = get_data();
            let (a, b) = source;
            a + b
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let escape = EscapeAnalysis::analyze(&cfg);

        // a and b should depend on source (and thus be return dependencies)
        // This tests that data flow is properly connected
        assert!(!escape.return_dependencies.is_empty());
    }

    #[test]
    fn test_pattern_binding_extraction() {
        let pat: Pat = parse_quote!((a, (b, c), Point { x, y }));
        let builder = CfgBuilder::new();
        let bindings = builder.extract_pattern_bindings(&pat);

        let names: Vec<_> = bindings.iter().map(|b| &b.name).collect();
        assert!(names.contains(&&"a".to_string()));
        assert!(names.contains(&&"b".to_string()));
        assert!(names.contains(&&"c".to_string()));
        assert!(names.contains(&&"x".to_string()));
        assert!(names.contains(&&"y".to_string()));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_pattern_with_def_use() {
    let block: Block = parse_quote!({
        let source = compute();
        let (a, b) = source;
        let result = a + b;
        result
    });

    let cfg = ControlFlowGraph::from_block(&block);
    let reaching = ReachingDefinitions::analyze(&cfg);

    // a and b should have definitions
    let a_defs: Vec<_> = reaching.all_definitions.iter()
        .filter(|d| cfg.var_names[d.var.name_id as usize] == "a")
        .collect();
    assert!(!a_defs.is_empty(), "a should have a definition");

    let b_defs: Vec<_> = reaching.all_definitions.iter()
        .filter(|d| cfg.var_names[d.var.name_id as usize] == "b")
        .collect();
    assert!(!b_defs.is_empty(), "b should have a definition");
}

#[test]
fn test_taint_through_pattern() {
    let block: Block = parse_quote!({
        let mut source = get_tainted();
        source.push(1);  // Taint source
        let (a, b) = source.split();  // a, b derived from tainted
        a + b
    });

    let cfg = ControlFlowGraph::from_block(&block);
    let liveness = LivenessInfo::analyze(&cfg);
    let escape = EscapeAnalysis::analyze(&cfg);
    let taint = TaintAnalysis::analyze(&cfg, &liveness, &escape);

    // Return should be tainted (through pattern)
    assert!(taint.return_tainted);
}
```

### Performance Tests

```rust
#[test]
fn test_pattern_extraction_performance() {
    use std::time::Instant;

    // Complex nested pattern
    let pat: Pat = parse_quote!(
        (a, (b, c), Point { x, y: (y1, y2) }, [first, second, rest @ ..])
    );

    let builder = CfgBuilder::new();

    let start = Instant::now();
    for _ in 0..10000 {
        let _ = builder.extract_pattern_bindings(&pat);
    }
    let elapsed = start.elapsed();

    // 10000 extractions should complete in <50ms
    assert!(elapsed.as_millis() < 50, "Took {:?}", elapsed);
}
```

## Documentation Requirements

- **Code Documentation**: Document all new types and extraction methods
- **User Documentation**: No changes (internal improvement)
- **Architecture Updates**: Document pattern handling in module docs

## Implementation Notes

### Handling Partial Patterns

For `let Point { x, .. } = p`, only `x` is bound. The `..` rest pattern doesn't create bindings.

### Reference Patterns

`let &x = r` binds `x` to the dereferenced value. The `by_ref` field in `PatternBinding` tracks this but doesn't change VarId creation.

### Mutable Bindings

`let (mut a, b) = t` makes `a` mutable. The `is_mut` field tracks this for potential future use (e.g., mutation analysis).

### Match Arm Patterns

Match arm patterns use the same extraction logic but bind from the scrutinee variable. This is handled by `bind_pattern_vars` which will be expanded in Spec 253.

## Migration and Compatibility

- **No migration needed**: Additive improvement
- **Backward compatible**: Simple patterns still work
- **Gradual benefit**: More patterns = better coverage
