use crate::analysis::multi_signal_aggregation::ResponsibilityCategory;
use crate::analysis::rust_patterns::context::RustFunctionContext;
use serde::{Deserialize, Serialize};
use syn::{FnArg, ItemFn, ReturnType, Type};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuilderPatternType {
    Constructor,
    BuilderMethod,
    WithMethod,
    SetterMethod,
    BuildFinalization,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuilderPattern {
    pub pattern_type: BuilderPatternType,
    pub confidence: f64,
    pub evidence: String,
}

pub struct RustBuilderDetector;

impl RustBuilderDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_builder_patterns(&self, context: &RustFunctionContext) -> Vec<BuilderPattern> {
        let mut patterns = Vec::new();
        let fn_name = context.item_fn.sig.ident.to_string();

        // Constructor pattern
        if matches!(fn_name.as_str(), "new" | "default" | "create") {
            patterns.push(BuilderPattern {
                pattern_type: BuilderPatternType::Constructor,
                confidence: 0.9,
                evidence: format!("Constructor method: {}", fn_name),
            });
        }

        // Check return type for Self
        if let ReturnType::Type(_, ty) = &context.item_fn.sig.output {
            if Self::returns_self(ty) {
                // with_* pattern
                if fn_name.starts_with("with_") {
                    patterns.push(BuilderPattern {
                        pattern_type: BuilderPatternType::WithMethod,
                        confidence: 0.95,
                        evidence: format!("Builder with_* method: {}", fn_name),
                    });
                    patterns.push(BuilderPattern {
                        pattern_type: BuilderPatternType::BuilderMethod,
                        confidence: 0.9,
                        evidence: "Returns Self for chaining".into(),
                    });
                }
                // set_* pattern
                else if fn_name.starts_with("set_") {
                    patterns.push(BuilderPattern {
                        pattern_type: BuilderPatternType::SetterMethod,
                        confidence: 0.85,
                        evidence: format!("Builder set_* method: {}", fn_name),
                    });
                    patterns.push(BuilderPattern {
                        pattern_type: BuilderPatternType::BuilderMethod,
                        confidence: 0.85,
                        evidence: "Returns Self for chaining".into(),
                    });
                }
                // Generic builder method returning Self
                else if Self::takes_self_param(context.item_fn) {
                    patterns.push(BuilderPattern {
                        pattern_type: BuilderPatternType::BuilderMethod,
                        confidence: 0.75,
                        evidence: "Method returns Self for chaining".into(),
                    });
                }
            }
        }

        // Build finalization pattern
        if matches!(fn_name.as_str(), "build" | "finalize" | "finish") {
            patterns.push(BuilderPattern {
                pattern_type: BuilderPatternType::BuildFinalization,
                confidence: 0.9,
                evidence: format!("Builder finalization method: {}", fn_name),
            });
        }

        patterns
    }

    /// Check if return type is Self
    fn returns_self(ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                return segment.ident == "Self";
            }
        }
        false
    }

    /// Check if function takes &self or &mut self
    fn takes_self_param(item_fn: &ItemFn) -> bool {
        item_fn
            .sig
            .inputs
            .iter()
            .any(|arg| matches!(arg, FnArg::Receiver(_)))
    }

    pub fn classify_from_builder_patterns(
        &self,
        patterns: &[BuilderPattern],
    ) -> Option<ResponsibilityCategory> {
        if patterns.is_empty() {
            return None;
        }

        // Constructor = Pure Computation (construction logic)
        if patterns
            .iter()
            .any(|p| p.pattern_type == BuilderPatternType::Constructor)
        {
            return Some(ResponsibilityCategory::PureComputation);
        }

        // Builder methods = Transformation (building/configuring)
        if patterns.iter().any(|p| {
            matches!(
                p.pattern_type,
                BuilderPatternType::WithMethod | BuilderPatternType::SetterMethod
            )
        }) {
            return Some(ResponsibilityCategory::Transformation);
        }

        // Build finalization = Transformation
        if patterns
            .iter()
            .any(|p| p.pattern_type == BuilderPatternType::BuildFinalization)
        {
            return Some(ResponsibilityCategory::Transformation);
        }

        None
    }
}

impl Default for RustBuilderDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn create_test_context(code: &str) -> RustFunctionContext<'static> {
        let item_fn: &'static syn::ItemFn = Box::leak(Box::new(syn::parse_str(code).unwrap()));
        RustFunctionContext {
            item_fn,
            metrics: None,
            impl_context: None,
            file_path: Path::new("test.rs"),
        }
    }

    #[test]
    fn test_detect_new_constructor() {
        let detector = RustBuilderDetector::new();
        let code = r#"
            fn new() -> Self {
                Self { field: 0 }
            }
        "#;
        let context = create_test_context(code);
        let patterns = detector.detect_builder_patterns(&context);
        assert!(patterns.iter().any(|p| p.pattern_type == BuilderPatternType::Constructor));
    }

    #[test]
    fn test_detect_with_methods() {
        let detector = RustBuilderDetector::new();
        let code = r#"
            fn with_value(mut self, value: i32) -> Self {
                self.value = value;
                self
            }
        "#;
        let context = create_test_context(code);
        let patterns = detector.detect_builder_patterns(&context);
        assert!(patterns.iter().any(|p| p.pattern_type == BuilderPatternType::WithMethod));
    }

    #[test]
    fn test_detect_set_methods() {
        let detector = RustBuilderDetector::new();
        let code = r#"
            fn set_value(mut self, value: i32) -> Self {
                self.value = value;
                self
            }
        "#;
        let context = create_test_context(code);
        let patterns = detector.detect_builder_patterns(&context);
        assert!(patterns.iter().any(|p| p.pattern_type == BuilderPatternType::SetterMethod));
    }

    #[test]
    fn test_detect_build_method() {
        let detector = RustBuilderDetector::new();
        let code = r#"
            fn build(self) -> Target {
                Target { value: self.value }
            }
        "#;
        let context = create_test_context(code);
        let patterns = detector.detect_builder_patterns(&context);
        assert!(patterns.iter().any(|p| p.pattern_type == BuilderPatternType::BuildFinalization));
    }
}
