---
number: 212
title: Unified File Extractor
category: optimization
priority: critical
status: draft
dependencies: [211]
created: 2025-01-14
---

# Specification 212: Unified File Extractor

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: 211 (Unified Extraction Types)

## Context

With the extraction types defined in spec 211, we need a single-pass extractor that traverses a parsed AST exactly once and populates all the extraction data structures. This extractor replaces multiple separate traversals:

- Complexity calculation (cyclomatic, cognitive, nesting)
- Purity detection
- I/O operation detection
- Parameter extraction
- Transformation pattern detection
- Call site extraction
- Struct/impl extraction

Currently these are done by separate analyzers that each parse the file independently.

## Objective

Implement a `UnifiedFileExtractor` that performs a single AST traversal to extract all data needed by downstream analysis phases, producing an `ExtractedFileData` structure.

## Requirements

### Functional Requirements

1. **Single-Pass Extraction**: Parse file once with `syn::parse_file`, traverse AST once to extract everything

2. **Function Extraction**: For each function/method found:
   - Extract name, qualified name, line numbers
   - Calculate cyclomatic, cognitive, nesting complexity
   - Run purity detector to get PurityAnalysisData
   - Detect I/O operations in function body
   - Extract parameter names from signature
   - Detect transformation patterns (map/filter/fold)
   - Extract all call sites

3. **Struct Extraction**: For each struct:
   - Extract name, line number
   - Extract all fields with types and visibility

4. **Impl Block Extraction**: For each impl block:
   - Extract type name and trait name
   - Extract method summaries (name, line, visibility)

5. **Import Extraction**: For each use statement:
   - Extract full path
   - Extract alias if present
   - Mark glob imports

6. **Line Count**: Calculate total lines in file

7. **Batched Processing**: Support processing files in batches with SourceMap reset between batches

### Non-Functional Requirements

- Extraction should be ~2x slower than single-purpose parse (acceptable given 43x reduction in parses)
- Must handle parse errors gracefully (return error, don't panic)
- Must reset SourceMap after extraction to prevent overflow
- Should be callable from parallel rayon iterators

## Acceptance Criteria

- [ ] `UnifiedFileExtractor::extract(path, content) -> Result<ExtractedFileData>` implemented
- [ ] Extracts all function data including complexity metrics
- [ ] Extracts purity analysis using existing PurityDetector
- [ ] Extracts I/O operations using existing detection logic
- [ ] Extracts transformation patterns
- [ ] Extracts all call sites with correct CallType
- [ ] Extracts struct and impl block information
- [ ] Extracts imports
- [ ] Correctly calculates line count
- [ ] `extract_batch(files) -> Vec<Result<ExtractedFileData>>` for parallel processing
- [ ] Resets SourceMap after each batch
- [ ] All existing test files produce equivalent data to current analyzers
- [ ] Performance within 2x of single-purpose parse

## Technical Details

### Module Location

```
src/extraction/
├── mod.rs           # Public exports
├── types.rs         # Type definitions (spec 211)
├── extractor.rs     # UnifiedFileExtractor (this spec)
└── visitors/
    ├── mod.rs
    ├── function.rs  # Function visitor logic
    ├── struct_impl.rs # Struct/impl visitor
    └── calls.rs     # Call extraction
```

### Core Implementation

```rust
use crate::extraction::types::*;
use crate::analyzers::purity_detector::PurityDetector;
use crate::core::parsing::reset_span_locations;
use anyhow::Result;
use std::path::Path;
use syn::visit::Visit;

/// Single-pass extractor for all file data.
pub struct UnifiedFileExtractor {
    /// Content for line calculations
    content: String,
    /// Number of lines in content
    line_count: usize,
}

impl UnifiedFileExtractor {
    /// Extract all data from a file in a single pass.
    ///
    /// Parses the file once and extracts:
    /// - All function/method data with complexity and purity
    /// - All struct definitions
    /// - All impl blocks
    /// - All imports
    ///
    /// Resets SourceMap after extraction to prevent overflow.
    pub fn extract(path: &Path, content: &str) -> Result<ExtractedFileData> {
        let ast = syn::parse_file(content)
            .map_err(|e| anyhow::anyhow!("Parse error in {}: {}", path.display(), e))?;

        let extractor = Self {
            content: content.to_string(),
            line_count: content.lines().count(),
        };

        let data = extractor.extract_from_ast(path, &ast);

        // Reset SourceMap to prevent overflow
        reset_span_locations();

        Ok(data)
    }

    /// Extract from multiple files in parallel with batched SourceMap resets.
    ///
    /// Processes files in batches, resetting SourceMap between batches
    /// to prevent overflow on large codebases.
    pub fn extract_batch(
        files: &[(PathBuf, String)],
        batch_size: usize,
    ) -> Vec<(PathBuf, Result<ExtractedFileData>)> {
        use rayon::prelude::*;

        let mut results = Vec::with_capacity(files.len());

        for batch in files.chunks(batch_size) {
            // Read contents in parallel (I/O bound)
            let batch_results: Vec<_> = batch
                .par_iter()
                .map(|(path, content)| {
                    let result = Self::extract(path, content);
                    (path.clone(), result)
                })
                .collect();

            results.extend(batch_results);

            // Reset after each batch
            reset_span_locations();
        }

        results
    }

    fn extract_from_ast(&self, path: &Path, ast: &syn::File) -> ExtractedFileData {
        let mut data = ExtractedFileData::empty(path.to_path_buf());
        data.total_lines = self.line_count;

        // Track if we're in a test module
        let mut in_test_module = false;

        // Extract imports
        data.imports = self.extract_imports(ast);

        // Visit all items
        for item in &ast.items {
            match item {
                syn::Item::Fn(item_fn) => {
                    let func_data = self.extract_function(item_fn, None, in_test_module);
                    data.functions.push(func_data);
                }
                syn::Item::Struct(item_struct) => {
                    let struct_data = self.extract_struct(item_struct);
                    data.structs.push(struct_data);
                }
                syn::Item::Impl(item_impl) => {
                    let (impl_data, methods) = self.extract_impl(item_impl, in_test_module);
                    data.impls.push(impl_data);
                    data.functions.extend(methods);
                }
                syn::Item::Mod(item_mod) => {
                    // Check for #[cfg(test)]
                    let is_test_mod = item_mod.attrs.iter().any(|attr| {
                        attr.path().is_ident("cfg")
                            && attr.meta.require_list().ok().map_or(false, |list| {
                                list.tokens.to_string().contains("test")
                            })
                    });

                    if let Some((_, items)) = &item_mod.content {
                        let nested_in_test = in_test_module || is_test_mod;
                        self.extract_module_items(items, &mut data, nested_in_test);
                    }
                }
                _ => {}
            }
        }

        data
    }

    fn extract_function(
        &self,
        item_fn: &syn::ItemFn,
        impl_type: Option<&str>,
        in_test_module: bool,
    ) -> ExtractedFunctionData {
        let name = item_fn.sig.ident.to_string();
        let qualified_name = impl_type
            .map(|t| format!("{}::{}", t, name))
            .unwrap_or_else(|| name.clone());

        let line = self.span_to_line(&item_fn.sig.ident.span());
        let end_line = self.estimate_end_line(item_fn);
        let length = end_line.saturating_sub(line) + 1;

        // Calculate complexity
        let (cyclomatic, cognitive, nesting) = self.calculate_complexity(&item_fn.block);

        // Extract purity analysis
        let purity_analysis = self.extract_purity(item_fn);

        // Extract I/O operations
        let io_operations = self.extract_io_operations(&item_fn.block);

        // Extract parameters
        let parameter_names = self.extract_parameters(&item_fn.sig);

        // Extract transformation patterns
        let transformation_patterns = self.extract_transformations(&item_fn.block);

        // Extract calls
        let calls = self.extract_calls(&item_fn.block);

        // Extract metadata
        let is_test = self.is_test_function(&item_fn.attrs) || in_test_module;
        let is_async = item_fn.sig.asyncness.is_some();
        let visibility = self.extract_visibility(&item_fn.vis);
        let is_trait_method = false; // Top-level functions are not trait methods

        ExtractedFunctionData {
            name,
            qualified_name,
            line,
            end_line,
            length,
            cyclomatic,
            cognitive,
            nesting,
            purity_analysis,
            io_operations,
            parameter_names,
            transformation_patterns,
            calls,
            is_test,
            is_async,
            visibility,
            is_trait_method,
            in_test_module,
        }
    }

    fn extract_purity(&self, item_fn: &syn::ItemFn) -> PurityAnalysisData {
        let mut detector = PurityDetector::new();
        let analysis = detector.is_pure_function(item_fn);

        PurityAnalysisData {
            is_pure: analysis.is_pure,
            has_mutations: analysis.total_mutations > 0,
            has_io_operations: analysis.has_io,
            has_unsafe: analysis.has_unsafe,
            local_mutations: analysis.local_mutations.clone(),
            upvalue_mutations: analysis.upvalue_mutations.clone(),
            total_mutations: analysis.total_mutations,
            var_names: analysis.var_names.clone(),
            confidence: analysis.confidence,
            purity_level: Self::to_purity_level(&analysis),
        }
    }

    fn to_purity_level(analysis: &crate::analyzers::purity_detector::PurityAnalysis) -> PurityLevel {
        if analysis.is_pure && analysis.total_mutations == 0 && !analysis.has_unsafe {
            PurityLevel::StrictlyPure
        } else if analysis.is_pure {
            PurityLevel::LocallyPure
        } else if !analysis.has_io && analysis.upvalue_mutations.is_empty() {
            PurityLevel::ReadOnly
        } else {
            PurityLevel::Impure
        }
    }

    fn extract_io_operations(&self, block: &syn::Block) -> Vec<IoOperation> {
        // Reuse existing I/O detection logic
        crate::data_flow::io_detector::detect_io_operations_from_block(block)
            .into_iter()
            .map(|op| IoOperation {
                io_type: Self::convert_io_type(&op),
                description: op.to_string(),
                line: 0, // Could extract from span if needed
            })
            .collect()
    }

    fn extract_parameters(&self, sig: &syn::Signature) -> Vec<String> {
        sig.inputs
            .iter()
            .filter_map(|arg| {
                if let syn::FnArg::Typed(pat_type) = arg {
                    if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                        return Some(pat_ident.ident.to_string());
                    }
                }
                None
            })
            .collect()
    }

    fn extract_calls(&self, block: &syn::Block) -> Vec<CallSite> {
        let mut visitor = CallVisitor::new();
        visitor.visit_block(block);
        visitor.calls
    }

    // ... additional helper methods
}

/// Visitor to extract function calls from a block.
struct CallVisitor {
    calls: Vec<CallSite>,
}

impl CallVisitor {
    fn new() -> Self {
        Self { calls: Vec::new() }
    }
}

impl<'ast> syn::visit::Visit<'ast> for CallVisitor {
    fn visit_expr_call(&mut self, expr: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = &*expr.func {
            let name = path.path.segments.last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();

            self.calls.push(CallSite {
                callee_name: name,
                call_type: CallType::Direct,
                line: 0,
            });
        }
        syn::visit::visit_expr_call(self, expr);
    }

    fn visit_expr_method_call(&mut self, expr: &'ast syn::ExprMethodCall) {
        self.calls.push(CallSite {
            callee_name: expr.method.to_string(),
            call_type: CallType::Method,
            line: 0,
        });
        syn::visit::visit_expr_method_call(self, expr);
    }
}
```

### Integration with Existing Code

The extractor reuses existing analysis logic where possible:
- `PurityDetector::is_pure_function` for purity analysis
- `detect_io_operations_from_block` for I/O detection
- Complexity calculation follows existing algorithm

### Batched Processing

```rust
/// Process files with SourceMap reset between batches.
pub fn extract_files_batched(
    files: &[PathBuf],
    batch_size: usize, // Default: 200
) -> Vec<Result<ExtractedFileData>> {
    let mut results = Vec::with_capacity(files.len());

    for batch in files.chunks(batch_size) {
        // Read file contents
        let contents: Vec<_> = batch
            .iter()
            .filter_map(|p| std::fs::read_to_string(p).ok().map(|c| (p.clone(), c)))
            .collect();

        // Extract in parallel
        let batch_results: Vec<_> = contents
            .par_iter()
            .map(|(path, content)| UnifiedFileExtractor::extract(path, content))
            .collect();

        results.extend(batch_results);

        // Reset SourceMap after batch
        reset_span_locations();
    }

    results
}
```

## Dependencies

- **Prerequisites**: Spec 211 (types must exist)
- **Affected Components**: None (new module)
- **External Dependencies**: syn, rayon

## Testing Strategy

- **Unit Tests**: Extract from small code snippets, verify all fields populated correctly
- **Equivalence Tests**: Compare extracted complexity with existing analyzer output
- **Purity Tests**: Compare extracted purity with PurityDetector output
- **I/O Tests**: Compare extracted I/O ops with existing detector output
- **Batch Tests**: Verify batched processing resets SourceMap correctly
- **Large File Tests**: Verify no overflow on files with many functions

## Documentation Requirements

- **Code Documentation**: Rustdoc on all public methods
- **Examples**: Example usage in module docs

## Implementation Notes

- Use `syn::visit::Visit` trait for AST traversal
- Extract line numbers using `span.start().line` before SourceMap reset
- Handle nested items (functions in impl blocks, items in modules)
- Be careful with `#[cfg(test)]` modules - mark functions as in_test_module

## Migration and Compatibility

No migration needed - this is additive. Existing analyzers continue to work until spec 213/214 integrate this.
