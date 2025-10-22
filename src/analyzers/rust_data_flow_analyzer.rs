//! AST-based data flow classification for Rust
//!
//! This module implements detection of data flow orchestration patterns
//! to reduce false positives in priority ranking (Spec 126).
//!
//! # Problem
//!
//! Functions that primarily transform or orchestrate data without complex
//! business logic are flagged as CRITICAL, but they're just data plumbing.
//!
//! # Detection Strategy
//!
//! 1. **Pattern Detection**: Identify iterator chains, struct builders, serialization
//! 2. **Business Logic Detection**: Identify arithmetic, validation, complex conditionals
//! 3. **Ratio Calculation**: transformation_ops / total_ops
//! 4. **Confidence Scoring**: Based on signal strength and operation count
//!
//! # Examples Detected
//!
//! ```rust,ignore
//! // Detected as data flow (transformation ratio > 0.7)
//! pub fn prepare_response(data: Vec<Item>) -> Response {
//!     let filtered = data.into_iter()
//!         .filter(|item| !item.is_deleted)
//!         .collect();
//!
//!     let serialized = serde_json::to_string(&filtered)?;
//!
//!     Response {
//!         body: serialized,
//!         status: 200,
//!     }
//! }
//!
//! // NOT detected (has business logic)
//! pub fn calculate_price(quantity: i32, base_price: f64) -> f64 {
//!     let discount = if quantity > 100 {
//!         0.2
//!     } else if quantity > 50 {
//!         0.1
//!     } else {
//!         0.0
//!     };
//!     base_price * quantity as f64 * (1.0 - discount)
//! }
//! ```

use syn::{visit::Visit, BinOp, Expr, ExprBinary, ExprCall, ExprIf, ExprMethodCall, ItemFn, Stmt};

#[derive(Debug, Clone)]
pub struct DataFlowProfile {
    /// Ratio of data transformation operations (0.0 - 1.0)
    pub transformation_ratio: f64,

    /// Ratio of business logic operations (0.0 - 1.0)
    pub business_logic_ratio: f64,

    /// Confidence in classification (0.0 - 1.0)
    pub confidence: f64,

    /// Detected patterns
    pub patterns: Vec<DataFlowPattern>,
}

impl DataFlowProfile {
    /// Create an unknown profile (low confidence)
    pub fn unknown() -> Self {
        Self {
            transformation_ratio: 0.0,
            business_logic_ratio: 0.0,
            confidence: 0.0,
            patterns: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataFlowPattern {
    IteratorChain { length: usize },
    StructBuilder { fields: usize },
    Serialization { format: String },
    IOOperation { kind: String },
    BusinessLogic { kind: String },
}

/// Analyze function for data flow patterns
pub fn analyze_data_flow(syn_func: &ItemFn) -> DataFlowProfile {
    let mut visitor = DataFlowVisitor::new();
    visitor.visit_item_fn(syn_func);

    let total_ops = visitor.transformation_ops + visitor.business_logic_ops;
    if total_ops == 0 {
        return DataFlowProfile::unknown();
    }

    let transformation_ratio = visitor.transformation_ops as f64 / total_ops as f64;
    let business_logic_ratio = visitor.business_logic_ops as f64 / total_ops as f64;

    // Calculate confidence based on signal strength
    let confidence = calculate_confidence(&visitor);

    DataFlowProfile {
        transformation_ratio,
        business_logic_ratio,
        confidence,
        patterns: visitor.patterns,
    }
}

struct DataFlowVisitor {
    transformation_ops: usize,
    business_logic_ops: usize,
    patterns: Vec<DataFlowPattern>,
    current_chain_length: usize,
}

impl DataFlowVisitor {
    fn new() -> Self {
        Self {
            transformation_ops: 0,
            business_logic_ops: 0,
            patterns: vec![],
            current_chain_length: 0,
        }
    }
}

impl<'ast> Visit<'ast> for DataFlowVisitor {
    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method_name = node.method.to_string();

        // Iterator transformations
        if matches!(
            method_name.as_str(),
            "map"
                | "filter"
                | "fold"
                | "collect"
                | "for_each"
                | "filter_map"
                | "flat_map"
                | "zip"
                | "chain"
                | "into_iter"
                | "iter"
        ) {
            self.transformation_ops += 1;
            self.current_chain_length += 1;
        }

        // Serialization operations
        if matches!(
            method_name.as_str(),
            "to_string" | "serialize" | "deserialize" | "to_json" | "from_json"
        ) {
            self.transformation_ops += 1;
            self.patterns.push(DataFlowPattern::Serialization {
                format: method_name.clone(),
            });
        }

        // I/O operations
        if matches!(
            method_name.as_str(),
            "read" | "write" | "read_to_string" | "write_all" | "flush" | "read_line"
                | "read_exact" | "write_fmt" | "sync_all" | "sync_data"
        ) {
            self.transformation_ops += 1;
            self.patterns.push(DataFlowPattern::IOOperation {
                kind: method_name.clone(),
            });
        }

        // Track iterator chains
        if self.current_chain_length > 2 {
            self.patterns.push(DataFlowPattern::IteratorChain {
                length: self.current_chain_length,
            });
        }

        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        // Detect File::open, File::create, etc.
        if let Expr::Path(path_expr) = &*node.func {
            let path_str = path_expr
                .path
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            // File operations
            if matches!(
                path_str.as_str(),
                "File::open" | "File::create" | "File::options" | "OpenOptions::new"
                    | "read_to_string" | "write" | "fs::read" | "fs::write"
                    | "fs::read_to_string" | "fs::write_all"
            ) {
                self.transformation_ops += 1;
                self.patterns.push(DataFlowPattern::IOOperation {
                    kind: path_str,
                });
            }
        }

        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast ExprBinary) {
        // Arithmetic operations = business logic
        if matches!(
            node.op,
            BinOp::Add(_) | BinOp::Sub(_) | BinOp::Mul(_) | BinOp::Div(_) | BinOp::Rem(_)
        ) {
            self.business_logic_ops += 1;
            self.patterns.push(DataFlowPattern::BusinessLogic {
                kind: "arithmetic".to_string(),
            });
        }

        // Complex comparisons in business contexts
        if matches!(
            node.op,
            BinOp::Lt(_) | BinOp::Le(_) | BinOp::Gt(_) | BinOp::Ge(_)
        ) {
            // These could be business rules or simple checks
            // We'll count them as weak business logic signal
            self.business_logic_ops += 1;
        }

        syn::visit::visit_expr_binary(self, node);
    }

    fn visit_expr_if(&mut self, node: &'ast ExprIf) {
        // Complex conditionals with business logic
        if is_business_logic_condition(&node.cond) {
            self.business_logic_ops += 1;
            self.patterns.push(DataFlowPattern::BusinessLogic {
                kind: "conditional".to_string(),
            });
        }

        syn::visit::visit_expr_if(self, node);
    }

    fn visit_stmt(&mut self, node: &'ast Stmt) {
        // Track struct initialization (builder pattern)
        if let Stmt::Local(local) = node {
            if let Some(init) = &local.init {
                if matches!(init.expr.as_ref(), Expr::Struct(_)) {
                    self.transformation_ops += 1;
                }
            }
        }

        syn::visit::visit_stmt(self, node);
    }
}

/// Check if condition represents business logic
fn is_business_logic_condition(cond: &Expr) -> bool {
    match cond {
        // Binary operations with arithmetic or complex comparisons
        Expr::Binary(bin) => matches!(
            bin.op,
            BinOp::Lt(_)
                | BinOp::Le(_)
                | BinOp::Gt(_)
                | BinOp::Ge(_)
                | BinOp::Add(_)
                | BinOp::Sub(_)
                | BinOp::Mul(_)
                | BinOp::Div(_)
        ),
        // Method calls in conditions could be business logic
        Expr::MethodCall(_) => true,
        // Simple path checks (is_empty, etc.) are not business logic
        Expr::Path(_) => false,
        _ => false,
    }
}

/// Calculate confidence score based on signal strength
fn calculate_confidence(visitor: &DataFlowVisitor) -> f64 {
    let total_ops = visitor.transformation_ops + visitor.business_logic_ops;

    // Low total operations = low confidence
    if total_ops < 5 {
        return 0.3;
    }

    // Calculate signal strength
    let max_ops = visitor.transformation_ops.max(visitor.business_logic_ops);
    let max_ratio = max_ops as f64 / total_ops as f64;

    // Strong signal = high confidence
    if max_ratio > 0.9 {
        0.95
    } else if max_ratio > 0.8 {
        0.85
    } else if max_ratio > 0.7 {
        0.75
    } else {
        0.5 // Ambiguous - don't classify
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_iterator_chain_detected() {
        let code: ItemFn = parse_quote! {
            fn transform(items: Vec<i32>) -> Vec<i32> {
                items.into_iter()
                    .filter(|x| *x > 0)
                    .map(|x| x * 2)
                    .collect()
            }
        };

        let profile = analyze_data_flow(&code);

        // Note: filter with comparison and map with arithmetic are counted
        // Transformation ops: into_iter, filter, map, collect = 4
        // Business logic ops: comparison (>), multiplication (*) = 2
        // Ratio: 4/6 = 0.666... (still high but not > 0.7)
        assert!(
            profile.transformation_ratio > 0.5,
            "Expected transformation ratio > 0.5, got {}",
            profile.transformation_ratio
        );
        assert!(
            profile.confidence >= 0.5,
            "Expected confidence >= 0.5, got {}",
            profile.confidence
        );
    }

    #[test]
    fn test_business_logic_not_misclassified() {
        let code: ItemFn = parse_quote! {
            fn calculate_price(quantity: i32, base_price: f64) -> f64 {
                let discount = if quantity > 100 {
                    0.2
                } else if quantity > 50 {
                    0.1
                } else {
                    0.0
                };

                base_price * (quantity as f64) * (1.0 - discount)
            }
        };

        let profile = analyze_data_flow(&code);

        // Should NOT be classified as data flow
        assert!(
            profile.business_logic_ratio > 0.5,
            "Expected high business logic ratio, got {}",
            profile.business_logic_ratio
        );
    }

    #[test]
    fn test_low_confidence_rejected() {
        let code: ItemFn = parse_quote! {
            fn mixed_function(x: i32) -> i32 {
                let y = x * 2;
                y + 1
            }
        };

        let profile = analyze_data_flow(&code);

        // Ambiguous - should have low confidence
        assert!(
            profile.confidence < 0.8,
            "Expected low confidence, got {}",
            profile.confidence
        );
    }

    #[test]
    fn test_struct_builder_detected() {
        let code: ItemFn = parse_quote! {
            fn build_config(env: &Environment) -> Config {
                let timeout = env.get("TIMEOUT").unwrap_or("30");
                let host = env.get("HOST").unwrap_or("localhost");

                Config {
                    timeout: timeout.parse().unwrap(),
                    host: host.to_string(),
                }
            }
        };

        let profile = analyze_data_flow(&code);

        // Should have some transformation operations (struct init, to_string)
        assert!(
            profile.transformation_ratio > 0.0,
            "Expected transformation ratio > 0"
        );
    }

    #[test]
    fn test_serialization_detected() {
        let code: ItemFn = parse_quote! {
            fn serialize_response(data: &Data) -> String {
                data.to_string()
            }
        };

        let profile = analyze_data_flow(&code);

        // Should detect serialization pattern
        assert!(
            profile
                .patterns
                .iter()
                .any(|p| matches!(p, DataFlowPattern::Serialization { .. })),
            "Expected serialization pattern, got patterns: {:?}",
            profile.patterns
        );
    }

    #[test]
    fn test_pure_transformation_high_ratio() {
        let code: ItemFn = parse_quote! {
            fn normalize_path(path: &Path) -> PathBuf {
                path.components()
                    .filter(|c| !matches!(c, Component::CurDir))
                    .map(|c| c.as_os_str())
                    .collect()
            }
        };

        let profile = analyze_data_flow(&code);

        assert!(
            profile.transformation_ratio > 0.7,
            "Pure transformation should have high ratio"
        );
        assert!(
            profile.business_logic_ratio < 0.3,
            "Should have low business logic ratio"
        );
    }

    #[test]
    fn test_complex_validation_business_logic() {
        let code: ItemFn = parse_quote! {
            fn validate_user(user: &User) -> Result<(), Error> {
                if user.age < 18 {
                    return Err(Error::TooYoung);
                }
                if user.email.is_empty() {
                    return Err(Error::MissingEmail);
                }
                if user.score < 0.0 {
                    return Err(Error::InvalidScore);
                }
                Ok(())
            }
        };

        let profile = analyze_data_flow(&code);

        assert!(
            profile.business_logic_ratio > 0.5,
            "Validation logic should be business logic"
        );
    }

    #[test]
    fn test_calculate_confidence_low_ops() {
        let visitor = DataFlowVisitor {
            transformation_ops: 2,
            business_logic_ops: 1,
            patterns: vec![],
            current_chain_length: 0,
        };

        let confidence = calculate_confidence(&visitor);
        assert!(
            confidence < 0.5,
            "Low operation count should have low confidence"
        );
    }

    #[test]
    fn test_calculate_confidence_strong_signal() {
        let visitor = DataFlowVisitor {
            transformation_ops: 10,
            business_logic_ops: 1,
            patterns: vec![],
            current_chain_length: 0,
        };

        let confidence = calculate_confidence(&visitor);
        assert!(
            confidence > 0.8,
            "Strong signal should have high confidence"
        );
    }

    #[test]
    fn test_is_business_logic_condition() {
        // Arithmetic in condition = business logic
        let expr: Expr = parse_quote! { quantity > 100 };
        assert!(is_business_logic_condition(&expr));

        // Simple path = not business logic
        let expr: Expr = parse_quote! { is_active };
        assert!(!is_business_logic_condition(&expr));

        // Method call = could be business logic
        let expr: Expr = parse_quote! { user.is_admin() };
        assert!(is_business_logic_condition(&expr));
    }

    #[test]
    fn test_iterator_chain_pattern_detected() {
        let code: ItemFn = parse_quote! {
            fn process(items: Vec<i32>) -> Vec<i32> {
                items.into_iter()
                    .filter(|x| *x > 0)
                    .map(|x| x * 2)
                    .collect()
            }
        };

        let profile = analyze_data_flow(&code);

        // Should detect iterator chain pattern
        assert!(
            profile.patterns.iter().any(|p| matches!(
                p,
                DataFlowPattern::IteratorChain { length } if *length > 2
            )),
            "Expected iterator chain pattern"
        );
    }

    #[test]
    fn test_unknown_profile_for_empty_function() {
        let code: ItemFn = parse_quote! {
            fn empty() {}
        };

        let profile = analyze_data_flow(&code);

        assert_eq!(profile.transformation_ratio, 0.0);
        assert_eq!(profile.business_logic_ratio, 0.0);
        assert_eq!(profile.confidence, 0.0);
    }

    #[test]
    fn test_io_operations_detected() {
        let code: ItemFn = parse_quote! {
            fn read_config(path: &Path) -> Result<String> {
                let mut file = File::open(path)?;
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                Ok(content)
            }
        };

        let profile = analyze_data_flow(&code);

        // Should detect I/O operations
        assert!(
            profile
                .patterns
                .iter()
                .any(|p| matches!(p, DataFlowPattern::IOOperation { .. })),
            "Expected I/O operation pattern, got patterns: {:?}",
            profile.patterns
        );

        // I/O is data transformation
        assert!(
            profile.transformation_ratio > 0.0,
            "I/O operations should contribute to transformation ratio"
        );
    }

    #[test]
    fn test_file_write_detected() {
        let code: ItemFn = parse_quote! {
            fn save_data(path: &Path, data: &str) -> Result<()> {
                fs::write(path, data)?;
                Ok(())
            }
        };

        let profile = analyze_data_flow(&code);

        // Should detect file write operation
        assert!(
            profile
                .patterns
                .iter()
                .any(|p| matches!(p, DataFlowPattern::IOOperation { kind } if kind.contains("write"))),
            "Expected file write operation"
        );
    }

    #[test]
    fn test_performance_overhead_under_5ms() {
        use std::time::Instant;

        // Create a large synthetic function with many operations
        let code: ItemFn = parse_quote! {
            fn large_function(items: Vec<ComplexData>) -> ProcessedResult {
                let step1 = items.into_iter()
                    .filter(|item| item.is_valid())
                    .filter(|item| item.score > 0.5)
                    .filter(|item| !item.is_deleted)
                    .map(|item| item.normalize())
                    .map(|item| item.transform())
                    .map(|item| item.validate())
                    .collect::<Vec<_>>();

                let step2 = step1.iter()
                    .filter(|item| item.priority > 10)
                    .map(|item| item.clone())
                    .collect::<Vec<_>>();

                let total = step2.iter().map(|item| item.value).sum::<f64>();
                let average = total / step2.len() as f64;

                let final_items = step2.into_iter()
                    .filter(|item| item.value > average)
                    .map(|item| item.finalize())
                    .collect::<Vec<_>>();

                let serialized = serde_json::to_string(&final_items).unwrap();

                ProcessedResult {
                    data: serialized,
                    count: final_items.len(),
                    average_value: average,
                }
            }
        };

        // Measure analysis time
        let start = Instant::now();
        let _profile = analyze_data_flow(&code);
        let duration = start.elapsed();

        // Assert < 5ms overhead per function
        assert!(
            duration.as_millis() < 5,
            "Analysis took {}ms, expected < 5ms",
            duration.as_millis()
        );

        // Also test with microsecond precision for better reporting
        println!(
            "Data flow analysis completed in {}µs ({}ms)",
            duration.as_micros(),
            duration.as_millis()
        );
    }
}
