use super::{
    MaintainabilityImpact, OrganizationAntiPattern, OrganizationDetector, PrimitiveUsageContext,
};
use crate::common::{capitalize_first, SourceLocation};
use std::collections::HashMap;
use syn::{self, visit::Visit};

pub struct PrimitiveObsessionDetector {
    track_string_identifiers: bool,
    track_numeric_measurements: bool,
    min_occurrences: usize,
}

impl Default for PrimitiveObsessionDetector {
    fn default() -> Self {
        Self {
            track_string_identifiers: true,
            track_numeric_measurements: true,
            min_occurrences: 3,
        }
    }
}

impl PrimitiveObsessionDetector {
    pub fn new() -> Self {
        Self::default()
    }

    fn analyze_primitive_usage(&self, type_usage: &TypeUsage) -> Option<PrimitiveUsageContext> {
        let name_lower = type_usage.context.to_lowercase();
        let type_name = &type_usage.type_name;

        // Use pattern matching with guards for cleaner logic
        match () {
            _ if self.track_string_identifiers
                && type_name == "String"
                && Self::contains_any(&name_lower, &["id", "key", "code"]) =>
            {
                Some(PrimitiveUsageContext::Identifier)
            }
            _ if self.track_numeric_measurements
                && Self::is_numeric_type(type_name)
                && Self::contains_any(
                    &name_lower,
                    &[
                        "distance",
                        "weight",
                        "height",
                        "temperature",
                        "price",
                        "amount",
                    ],
                ) =>
            {
                Some(PrimitiveUsageContext::Measurement)
            }
            _ if type_name == "bool"
                && Self::contains_any(&name_lower, &["status", "state", "flag"]) =>
            {
                Some(PrimitiveUsageContext::Status)
            }
            _ if Self::is_category_type(type_name)
                && Self::contains_any(&name_lower, &["type", "category", "kind", "mode"]) =>
            {
                Some(PrimitiveUsageContext::Category)
            }
            _ => None,
        }
    }

    // Helper methods to simplify pattern matching
    fn contains_any(text: &str, patterns: &[&str]) -> bool {
        patterns.iter().any(|pattern| text.contains(pattern))
    }

    fn is_numeric_type(type_name: &str) -> bool {
        matches!(type_name, "f64" | "f32" | "i32" | "u32")
    }

    fn is_category_type(type_name: &str) -> bool {
        matches!(type_name, "String" | "i32")
    }

    fn suggest_domain_type(&self, primitive_type: &str, context: &PrimitiveUsageContext) -> String {
        match context {
            PrimitiveUsageContext::Identifier => match primitive_type {
                "String" => "Id<T>".to_string(),
                _ => format!("{}Id", capitalize_first(primitive_type)),
            },
            PrimitiveUsageContext::Measurement => "Measurement<Unit>".to_string(),
            PrimitiveUsageContext::Status => "StatusEnum".to_string(),
            PrimitiveUsageContext::Category => "CategoryEnum".to_string(),
            PrimitiveUsageContext::BusinessRule => {
                format!("{}Rule", capitalize_first(primitive_type))
            }
        }
    }

    fn group_similar_usages(
        &self,
        usages: &[TypeUsage],
    ) -> HashMap<(String, PrimitiveUsageContext), Vec<TypeUsage>> {
        let mut groups = HashMap::new();

        for usage in usages {
            if let Some(context) = self.analyze_primitive_usage(usage) {
                let key = (usage.type_name.clone(), context);
                groups
                    .entry(key)
                    .or_insert_with(Vec::new)
                    .push(usage.clone());
            }
        }

        groups
    }
}

impl OrganizationDetector for PrimitiveObsessionDetector {
    fn detect_anti_patterns(&self, file: &syn::File) -> Vec<OrganizationAntiPattern> {
        let mut patterns = Vec::new();
        let mut visitor = TypeUsageVisitor::new();
        visitor.visit_file(file);

        let grouped = self.group_similar_usages(&visitor.type_usages);

        for ((primitive_type, usage_context), usages) in grouped {
            if usages.len() >= self.min_occurrences {
                patterns.push(OrganizationAntiPattern::PrimitiveObsession {
                    primitive_type: primitive_type.clone(),
                    usage_context: usage_context.clone(),
                    occurrence_count: usages.len(),
                    suggested_domain_type: self
                        .suggest_domain_type(&primitive_type, &usage_context),
                    locations: vec![SourceLocation::default()], // TODO: Extract actual locations
                });
            }
        }

        patterns
    }

    fn detector_name(&self) -> &'static str {
        "PrimitiveObsessionDetector"
    }

    fn estimate_maintainability_impact(
        &self,
        pattern: &OrganizationAntiPattern,
    ) -> MaintainabilityImpact {
        match pattern {
            OrganizationAntiPattern::PrimitiveObsession {
                occurrence_count,
                usage_context,
                ..
            } => match usage_context {
                PrimitiveUsageContext::Identifier | PrimitiveUsageContext::BusinessRule => {
                    if *occurrence_count > 10 {
                        MaintainabilityImpact::High
                    } else if *occurrence_count > 5 {
                        MaintainabilityImpact::Medium
                    } else {
                        MaintainabilityImpact::Low
                    }
                }
                _ => {
                    if *occurrence_count > 15 {
                        MaintainabilityImpact::Medium
                    } else {
                        MaintainabilityImpact::Low
                    }
                }
            },
            _ => MaintainabilityImpact::Low,
        }
    }
}

#[derive(Clone)]
struct TypeUsage {
    type_name: String,
    context: String, // Variable or field name
}

struct TypeUsageVisitor {
    type_usages: Vec<TypeUsage>,
}

impl TypeUsageVisitor {
    fn new() -> Self {
        Self {
            type_usages: Vec::new(),
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn extract_type_name(&self, ty: &syn::Type) -> String {
        match ty {
            syn::Type::Path(type_path) => type_path
                .path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            syn::Type::Reference(type_ref) => self.extract_type_name(&type_ref.elem),
            _ => "Unknown".to_string(),
        }
    }
}

impl<'ast> Visit<'ast> for TypeUsageVisitor {
    fn visit_field(&mut self, node: &'ast syn::Field) {
        if let Some(ident) = &node.ident {
            let type_name = self.extract_type_name(&node.ty);

            // Track primitive types
            if is_primitive_type(&type_name) {
                self.type_usages.push(TypeUsage {
                    type_name,
                    context: ident.to_string(),
                });
            }
        }

        syn::visit::visit_field(self, node);
    }

    fn visit_fn_arg(&mut self, node: &'ast syn::FnArg) {
        if let syn::FnArg::Typed(pat_type) = node {
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                let type_name = self.extract_type_name(&pat_type.ty);

                // Track primitive types
                if is_primitive_type(&type_name) {
                    self.type_usages.push(TypeUsage {
                        type_name,
                        context: pat_ident.ident.to_string(),
                    });
                }
            }
        }

        syn::visit::visit_fn_arg(self, node);
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        if let syn::Pat::Ident(pat_ident) = &node.pat {
            if let Some(init) = &node.init {
                // Try to infer type from initialization
                let type_name = self.infer_type_from_expr(&init.expr);

                if is_primitive_type(&type_name) {
                    self.type_usages.push(TypeUsage {
                        type_name,
                        context: pat_ident.ident.to_string(),
                    });
                }
            }
        }

        syn::visit::visit_local(self, node);
    }
}

impl TypeUsageVisitor {
    fn infer_type_from_expr(&self, expr: &syn::Expr) -> String {
        match expr {
            syn::Expr::Lit(expr_lit) => Self::infer_type_from_literal(&expr_lit.lit),
            syn::Expr::Call(expr_call) => Self::extract_function_name(expr_call),
            _ => "Unknown".to_string(),
        }
    }

    fn infer_type_from_literal(lit: &syn::Lit) -> String {
        match lit {
            syn::Lit::Str(_) => "String",
            syn::Lit::Int(_) => "i32",
            syn::Lit::Float(_) => "f64",
            syn::Lit::Bool(_) => "bool",
            _ => "Unknown",
        }
        .to_string()
    }

    fn extract_function_name(expr_call: &syn::ExprCall) -> String {
        if let syn::Expr::Path(path) = &*expr_call.func {
            path.path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            "Unknown".to_string()
        }
    }
}

fn is_primitive_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "bool"
            | "char"
            | "str"
            | "String"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
    )
}
