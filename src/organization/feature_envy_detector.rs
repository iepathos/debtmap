use super::{MaintainabilityImpact, OrganizationAntiPattern, OrganizationDetector};
use crate::common::SourceLocation;
use std::collections::HashMap;
use syn::{self, visit::Visit};

pub struct FeatureEnvyDetector {
    external_call_threshold: usize,
    internal_call_ratio: f64,
}

impl Default for FeatureEnvyDetector {
    fn default() -> Self {
        Self {
            external_call_threshold: 5,
            internal_call_ratio: 0.3,
        }
    }
}

impl FeatureEnvyDetector {
    pub fn new() -> Self {
        Self::default()
    }

    fn analyze_method_calls(&self, method: &MethodInfo) -> MethodCallAnalysis {
        let mut analysis = MethodCallAnalysis {
            method_name: method.name.clone(),
            internal_calls: 0,
            external_calls: HashMap::new(),
        };

        for call in &method.calls {
            if call.is_self_call {
                analysis.internal_calls += 1;
            } else {
                *analysis
                    .external_calls
                    .entry(call.receiver_type.clone())
                    .or_insert(0) += 1;
            }
        }

        analysis
    }

    fn find_envied_type(&self, analysis: &MethodCallAnalysis) -> Option<(String, usize)> {
        analysis
            .external_calls
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(type_name, count)| (type_name.clone(), *count))
    }

    fn should_report_feature_envy(&self, analysis: &MethodCallAnalysis) -> bool {
        let total_external: usize = analysis.external_calls.values().sum();
        let total_calls = analysis.internal_calls + total_external;

        if total_calls == 0 {
            return false;
        }

        let internal_ratio = analysis.internal_calls as f64 / total_calls as f64;

        total_external >= self.external_call_threshold && internal_ratio < self.internal_call_ratio
    }
}

impl OrganizationDetector for FeatureEnvyDetector {
    fn detect_anti_patterns(&self, file: &syn::File) -> Vec<OrganizationAntiPattern> {
        let mut patterns = Vec::new();
        let mut visitor = MethodVisitor::new();
        visitor.visit_file(file);

        for method in visitor.methods {
            let analysis = self.analyze_method_calls(&method);

            if self.should_report_feature_envy(&analysis) {
                if let Some((envied_type, external_count)) = self.find_envied_type(&analysis) {
                    patterns.push(OrganizationAntiPattern::FeatureEnvy {
                        method_name: method.name.clone(),
                        envied_type,
                        external_calls: external_count,
                        internal_calls: analysis.internal_calls,
                        suggested_move: external_count > analysis.internal_calls * 2,
                        location: SourceLocation::default(), // TODO: Extract actual location
                    });
                }
            }
        }

        patterns
    }

    fn detector_name(&self) -> &'static str {
        "FeatureEnvyDetector"
    }

    fn estimate_maintainability_impact(
        &self,
        pattern: &OrganizationAntiPattern,
    ) -> MaintainabilityImpact {
        match pattern {
            OrganizationAntiPattern::FeatureEnvy {
                external_calls,
                internal_calls,
                ..
            } => {
                let ratio = if *internal_calls > 0 {
                    *external_calls as f64 / *internal_calls as f64
                } else {
                    *external_calls as f64
                };

                if ratio > 5.0 {
                    MaintainabilityImpact::High
                } else if ratio > 2.0 {
                    MaintainabilityImpact::Medium
                } else {
                    MaintainabilityImpact::Low
                }
            }
            _ => MaintainabilityImpact::Low,
        }
    }
}

struct MethodCallAnalysis {
    #[allow(dead_code)]
    method_name: String,
    internal_calls: usize,
    external_calls: HashMap<String, usize>,
}

struct MethodInfo {
    name: String,
    calls: Vec<CallInfo>,
}

struct CallInfo {
    receiver_type: String,
    is_self_call: bool,
}

struct MethodVisitor {
    methods: Vec<MethodInfo>,
    current_method: Option<MethodInfo>,
}

impl MethodVisitor {
    fn new() -> Self {
        Self {
            methods: Vec::new(),
            current_method: None,
        }
    }
}

impl<'ast> Visit<'ast> for MethodVisitor {
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let old_method = self.current_method.take();

        self.current_method = Some(MethodInfo {
            name: node.sig.ident.to_string(),
            calls: Vec::new(),
        });

        // Visit the method body to collect calls
        syn::visit::visit_impl_item_fn(self, node);

        if let Some(method) = self.current_method.take() {
            self.methods.push(method);
        }

        self.current_method = old_method;
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let (receiver_type, is_self_call) = match &*node.receiver {
            syn::Expr::Path(path) => {
                let is_self = path
                    .path
                    .segments
                    .first()
                    .map(|seg| seg.ident == "self")
                    .unwrap_or(false);

                if is_self {
                    ("Self".to_string(), true)
                } else {
                    (self.extract_receiver_type(&node.receiver), false)
                }
            }
            _ => (self.extract_receiver_type(&node.receiver), false),
        };

        if let Some(ref mut method) = self.current_method {
            method.calls.push(CallInfo {
                receiver_type,
                is_self_call,
            });
        }

        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Some(ref mut method) = self.current_method {
            if let syn::Expr::Path(path) = &*node.func {
                let is_self_call = path
                    .path
                    .segments
                    .first()
                    .map(|seg| seg.ident == "Self" || seg.ident == "self")
                    .unwrap_or(false);

                if !is_self_call {
                    // Track static function calls as external
                    let receiver_type = path
                        .path
                        .segments
                        .first()
                        .map(|seg| seg.ident.to_string())
                        .unwrap_or_else(|| "Unknown".to_string());

                    method.calls.push(CallInfo {
                        receiver_type,
                        is_self_call: false,
                    });
                }
            }
        }

        syn::visit::visit_expr_call(self, node);
    }
}

impl MethodVisitor {
    #[allow(clippy::only_used_in_recursion)]
    fn extract_receiver_type(&self, expr: &syn::Expr) -> String {
        match expr {
            syn::Expr::Path(path) => path
                .path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            syn::Expr::Field(field) => self.extract_receiver_type(&field.base),
            _ => "Unknown".to_string(),
        }
    }
}
