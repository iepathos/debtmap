//! Type-based clustering for idiomatic Rust god object refactoring recommendations.
//!
//! This module analyzes type signatures to group methods by the data they operate on,
//! rather than by behavioral patterns. This follows idiomatic Rust principles where
//! data owns its behavior through impl blocks.

use std::collections::{HashMap, HashSet};
use syn::{FnArg, ReturnType, Type};

/// Information about a type extracted from a method signature
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TypeInfo {
    pub name: String,
    pub is_reference: bool,
    pub is_mutable: bool,
    pub generics: Vec<String>,
}

/// Method signature with extracted type information
#[derive(Clone, Debug)]
pub struct MethodSignature {
    pub name: String,
    pub param_types: Vec<TypeInfo>,
    pub return_type: Option<TypeInfo>,
    pub self_type: Option<TypeInfo>,
}

/// Cluster of methods grouped by type affinity
#[derive(Clone, Debug)]
pub struct TypeCluster {
    pub primary_type: TypeInfo,
    pub methods: Vec<String>,
    pub type_affinity_score: f64,
    pub input_types: HashSet<String>,
    pub output_types: HashSet<String>,
}

/// Analyzes method signatures to extract type information
pub struct TypeSignatureAnalyzer;

impl TypeSignatureAnalyzer {
    /// Extract type information from method
    pub fn analyze_method(&self, method: &syn::ImplItemFn) -> MethodSignature {
        let param_types = method
            .sig
            .inputs
            .iter()
            .filter_map(|arg| self.extract_type_from_arg(arg))
            .collect();

        let return_type = match &method.sig.output {
            ReturnType::Type(_, ty) => Some(self.extract_type_info(ty)),
            _ => None,
        };

        MethodSignature {
            name: method.sig.ident.to_string(),
            param_types,
            return_type,
            self_type: None, // Extracted from impl context
        }
    }

    /// Extract type information from standalone function
    pub fn analyze_function(&self, func: &syn::ItemFn) -> MethodSignature {
        let param_types = func
            .sig
            .inputs
            .iter()
            .filter_map(|arg| self.extract_type_from_arg(arg))
            .collect();

        let return_type = match &func.sig.output {
            ReturnType::Type(_, ty) => Some(self.extract_type_info(ty)),
            _ => None,
        };

        MethodSignature {
            name: func.sig.ident.to_string(),
            param_types,
            return_type,
            self_type: None,
        }
    }

    fn extract_type_from_arg(&self, arg: &FnArg) -> Option<TypeInfo> {
        match arg {
            FnArg::Typed(pat_type) => Some(self.extract_type_info(&pat_type.ty)),
            FnArg::Receiver(_) => None, // self
        }
    }

    fn extract_type_info(&self, ty: &Type) -> TypeInfo {
        match ty {
            Type::Path(type_path) => {
                let segment = type_path.path.segments.last();
                let name = segment
                    .map(|seg| seg.ident.to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                // Extract generic parameters
                let generics = segment
                    .and_then(|seg| match &seg.arguments {
                        syn::PathArguments::AngleBracketed(args) => Some(
                            args.args
                                .iter()
                                .filter_map(|arg| match arg {
                                    syn::GenericArgument::Type(ty) => {
                                        Some(self.extract_type_info(ty).name)
                                    }
                                    _ => None,
                                })
                                .collect(),
                        ),
                        _ => None,
                    })
                    .unwrap_or_default();

                TypeInfo {
                    name,
                    is_reference: false,
                    is_mutable: false,
                    generics,
                }
            }
            Type::Reference(type_ref) => {
                let mut inner = self.extract_type_info(&type_ref.elem);
                inner.is_reference = true;
                inner.is_mutable = type_ref.mutability.is_some();
                inner
            }
            _ => TypeInfo {
                name: "Unknown".to_string(),
                is_reference: false,
                is_mutable: false,
                generics: vec![],
            },
        }
    }
}

/// Analyzes type affinity between methods to suggest type-based clustering
pub struct TypeAffinityAnalyzer;

impl TypeAffinityAnalyzer {
    /// Cluster methods by type affinity (shared type usage)
    pub fn cluster_by_type_affinity(&self, signatures: &[MethodSignature]) -> Vec<TypeCluster> {
        if signatures.is_empty() {
            return vec![];
        }

        // Build type affinity matrix
        let affinity_matrix = self.build_type_affinity_matrix(signatures);

        // Group methods by dominant types
        let type_groups = self.group_by_dominant_type(signatures, &affinity_matrix);

        // Convert to TypeCluster
        type_groups
            .into_iter()
            .map(|(_type_name, methods)| {
                let method_sigs: Vec<_> = signatures
                    .iter()
                    .filter(|s| methods.contains(&s.name))
                    .collect();

                let input_types: HashSet<_> = method_sigs
                    .iter()
                    .flat_map(|s| s.param_types.iter().map(|t| t.name.clone()))
                    .collect();

                let output_types: HashSet<_> = method_sigs
                    .iter()
                    .filter_map(|s| s.return_type.as_ref().map(|t| t.name.clone()))
                    .collect();

                let primary_type = self.identify_primary_type(&methods, signatures);
                let type_affinity_score =
                    self.calculate_cluster_affinity(&methods, &affinity_matrix);

                TypeCluster {
                    primary_type,
                    methods,
                    type_affinity_score,
                    input_types,
                    output_types,
                }
            })
            .collect()
    }

    fn build_type_affinity_matrix(
        &self,
        signatures: &[MethodSignature],
    ) -> HashMap<(String, String), f64> {
        let mut affinity = HashMap::new();

        for sig1 in signatures {
            for sig2 in signatures {
                if sig1.name == sig2.name {
                    continue;
                }

                let score = self.calculate_type_affinity(sig1, sig2);
                if score > 0.0 {
                    affinity.insert((sig1.name.clone(), sig2.name.clone()), score);
                }
            }
        }

        affinity
    }

    /// Calculate type affinity between two method signatures
    ///
    /// Simple counting approach - methods belong together if they share types:
    /// - Shared domain types: +1 per shared type (ignoring primitives)
    /// - Same self type: +1
    fn calculate_type_affinity(&self, sig1: &MethodSignature, sig2: &MethodSignature) -> f64 {
        let mut score = 0.0;

        // Count shared domain types (ignore primitives)
        let shared_domain_types = sig1
            .param_types
            .iter()
            .filter(|t1| self.is_domain_type(&t1.name))
            .filter(|t1| sig2.param_types.iter().any(|t2| t1.name == t2.name))
            .count();

        score += shared_domain_types as f64;

        // Same self type
        if sig1.self_type == sig2.self_type && sig1.self_type.is_some() {
            score += 1.0;
        }

        // Check if return type of one matches param type of another (pipeline)
        if let Some(ret1) = &sig1.return_type {
            if sig2.param_types.iter().any(|p| p.name == ret1.name) {
                score += 0.5;
            }
        }
        if let Some(ret2) = &sig2.return_type {
            if sig1.param_types.iter().any(|p| p.name == ret2.name) {
                score += 0.5;
            }
        }

        score
    }

    /// Check if type is domain-specific (not primitive or stdlib)
    fn is_domain_type(&self, type_name: &str) -> bool {
        !matches!(
            type_name,
            "String"
                | "str"
                | "Vec"
                | "Option"
                | "Result"
                | "HashMap"
                | "HashSet"
                | "BTreeMap"
                | "BTreeSet"
                | "usize"
                | "isize"
                | "u32"
                | "i32"
                | "u64"
                | "i64"
                | "f32"
                | "f64"
                | "bool"
                | "char"
                | "Path"
                | "PathBuf"
        ) && !type_name.starts_with('&')
    }

    /// Group methods by their dominant type (the type they most frequently work with)
    fn group_by_dominant_type(
        &self,
        signatures: &[MethodSignature],
        _affinity_matrix: &HashMap<(String, String), f64>,
    ) -> HashMap<String, Vec<String>> {
        let mut type_groups: HashMap<String, Vec<String>> = HashMap::new();

        for sig in signatures {
            // Find dominant type for this method
            let dominant_type = self.find_dominant_type_for_method(sig);

            type_groups
                .entry(dominant_type)
                .or_default()
                .push(sig.name.clone());
        }

        type_groups
    }

    fn find_dominant_type_for_method(&self, sig: &MethodSignature) -> String {
        // Priority:
        // 1. Self type if available
        // 2. First domain parameter type
        // 3. Return type if domain type
        // 4. "Unknown"

        if let Some(self_type) = &sig.self_type {
            return self_type.name.clone();
        }

        for param in &sig.param_types {
            if self.is_domain_type(&param.name) {
                return self.extract_base_type(&param.name);
            }
        }

        if let Some(ret) = &sig.return_type {
            if self.is_domain_type(&ret.name) {
                return self.extract_base_type(&ret.name);
            }
        }

        "Unknown".to_string()
    }

    fn calculate_cluster_affinity(
        &self,
        methods: &[String],
        affinity_matrix: &HashMap<(String, String), f64>,
    ) -> f64 {
        if methods.len() < 2 {
            return 0.0;
        }

        let mut total_affinity = 0.0;
        let mut pair_count = 0;

        for m1 in methods {
            for m2 in methods {
                if m1 != m2 {
                    if let Some(score) = affinity_matrix.get(&(m1.clone(), m2.clone())) {
                        total_affinity += score;
                        pair_count += 1;
                    }
                }
            }
        }

        if pair_count == 0 {
            0.0
        } else {
            total_affinity / pair_count as f64
        }
    }

    /// Identify primary type for a cluster of methods
    ///
    /// Algorithm:
    /// 1. Count type occurrences (params + returns)
    /// 2. If tie, use tie-breaking rules:
    ///    - Prefer domain types over primitives
    ///    - Prefer return types (output) over param types (input)
    ///    - Prefer non-wrapper types (avoid Option<T>, Vec<T>)
    ///    - Prefer longer, more specific type names
    /// 3. Extract base type from generics
    fn identify_primary_type(
        &self,
        methods: &[String],
        signatures: &[MethodSignature],
    ) -> TypeInfo {
        #[derive(Debug, Clone)]
        struct TypeCandidate {
            name: String,
            count: usize,
            is_domain_type: bool,
            return_occurrences: usize,
            param_occurrences: usize,
        }

        // Count type occurrences with detailed tracking
        let mut type_candidates: HashMap<String, TypeCandidate> = HashMap::new();

        for method in methods {
            if let Some(sig) = signatures.iter().find(|s| &s.name == method) {
                // Count parameter types
                for param in &sig.param_types {
                    let base_name = self.extract_base_type(&param.name);
                    type_candidates
                        .entry(base_name.clone())
                        .and_modify(|c| {
                            c.count += 1;
                            c.param_occurrences += 1;
                        })
                        .or_insert_with(|| TypeCandidate {
                            name: base_name.clone(),
                            count: 1,
                            is_domain_type: self.is_domain_type(&base_name),
                            return_occurrences: 0,
                            param_occurrences: 1,
                        });
                }

                // Count return types (with bonus weight)
                if let Some(ret) = &sig.return_type {
                    let base_name = self.extract_base_type(&ret.name);
                    type_candidates
                        .entry(base_name.clone())
                        .and_modify(|c| {
                            c.count += 1;
                            c.return_occurrences += 1;
                        })
                        .or_insert_with(|| TypeCandidate {
                            name: base_name.clone(),
                            count: 1,
                            is_domain_type: self.is_domain_type(&base_name),
                            return_occurrences: 1,
                            param_occurrences: 0,
                        });
                }
            }
        }

        // Remove primitives and stdlib types if domain types exist
        let has_domain_types = type_candidates.values().any(|c| c.is_domain_type);
        if has_domain_types {
            type_candidates.retain(|_, c| c.is_domain_type);
        }

        // Select primary type using tie-breaking rules
        let primary_candidate = type_candidates
            .values()
            .max_by(|a, b| {
                // Rule 1: Most occurrences wins
                match a.count.cmp(&b.count) {
                    std::cmp::Ordering::Equal => {
                        // Rule 2: Prefer types that appear as returns (outputs)
                        match a.return_occurrences.cmp(&b.return_occurrences) {
                            std::cmp::Ordering::Equal => {
                                // Rule 3: Prefer domain types
                                match a.is_domain_type.cmp(&b.is_domain_type) {
                                    std::cmp::Ordering::Equal => {
                                        // Rule 4: Prefer longer names (more specific)
                                        a.name.len().cmp(&b.name.len())
                                    }
                                    other => other,
                                }
                            }
                            other => other,
                        }
                    }
                    other => other,
                }
            })
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        TypeInfo {
            name: primary_candidate,
            is_reference: false,
            is_mutable: false,
            generics: vec![],
        }
    }

    /// Extract base type from generic wrappers
    ///
    /// Examples:
    /// - `Option<Metrics>` → `Metrics`
    /// - `Vec<Item>` → `Item`
    /// - `Result<Data, Error>` → `Data` (first generic arg)
    /// - `Metrics` → `Metrics` (unchanged)
    fn extract_base_type(&self, type_name: &str) -> String {
        // Handle generic types
        if let Some(start) = type_name.find('<') {
            if let Some(end) = type_name.rfind('>') {
                let inner = &type_name[start + 1..end];
                // For multi-generic types (e.g., Result<T, E>), take first
                if let Some(comma) = inner.find(',') {
                    return inner[..comma].trim().to_string();
                }
                return inner.trim().to_string();
            }
        }

        // Handle references
        type_name
            .trim_start_matches('&')
            .trim_start_matches("mut ")
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper to create a simple type
    fn simple_type(name: &str) -> TypeInfo {
        TypeInfo {
            name: name.to_string(),
            is_reference: false,
            is_mutable: false,
            generics: vec![],
        }
    }

    /// Test helper to create a method signature
    fn method_sig(name: &str, params: Vec<TypeInfo>, ret: Option<TypeInfo>) -> MethodSignature {
        MethodSignature {
            name: name.to_string(),
            param_types: params,
            return_type: ret,
            self_type: None,
        }
    }

    #[test]
    fn test_is_domain_type() {
        let analyzer = TypeAffinityAnalyzer;

        // Primitives should not be domain types
        assert!(!analyzer.is_domain_type("String"));
        assert!(!analyzer.is_domain_type("u32"));
        assert!(!analyzer.is_domain_type("Vec"));
        assert!(!analyzer.is_domain_type("Option"));
        assert!(!analyzer.is_domain_type("PathBuf"));

        // Custom types should be domain types
        assert!(analyzer.is_domain_type("Metrics"));
        assert!(analyzer.is_domain_type("PriorityItem"));
        assert!(analyzer.is_domain_type("GodObjectAnalysis"));
    }

    #[test]
    fn test_extract_base_type() {
        let analyzer = TypeAffinityAnalyzer;

        assert_eq!(analyzer.extract_base_type("Metrics"), "Metrics");
        assert_eq!(analyzer.extract_base_type("Option<Metrics>"), "Metrics");
        assert_eq!(analyzer.extract_base_type("Vec<Item>"), "Item");
        assert_eq!(analyzer.extract_base_type("Result<Data, Error>"), "Data");
        assert_eq!(analyzer.extract_base_type("&Metrics"), "Metrics");
        assert_eq!(analyzer.extract_base_type("&mut Config"), "Config");
    }

    #[test]
    fn test_type_affinity_shared_params() {
        let analyzer = TypeAffinityAnalyzer;

        let sig1 = method_sig(
            "analyze",
            vec![simple_type("Metrics")],
            Some(simple_type("Score")),
        );
        let sig2 = method_sig(
            "format",
            vec![simple_type("Metrics")],
            Some(simple_type("String")),
        );

        let affinity = analyzer.calculate_type_affinity(&sig1, &sig2);
        assert!(
            affinity > 0.0,
            "Methods sharing domain types should have positive affinity"
        );
    }

    #[test]
    fn test_type_affinity_pipeline() {
        let analyzer = TypeAffinityAnalyzer;

        let sig1 = method_sig(
            "parse",
            vec![simple_type("String")],
            Some(simple_type("Metrics")),
        );
        let sig2 = method_sig(
            "analyze",
            vec![simple_type("Metrics")],
            Some(simple_type("Score")),
        );

        let affinity = analyzer.calculate_type_affinity(&sig1, &sig2);
        assert!(
            affinity > 0.0,
            "Pipeline methods should have positive affinity"
        );
    }

    #[test]
    fn test_identify_primary_type_simple() {
        let analyzer = TypeAffinityAnalyzer;

        let signatures = vec![
            method_sig(
                "format",
                vec![simple_type("PriorityItem")],
                Some(simple_type("String")),
            ),
            method_sig(
                "calculate",
                vec![simple_type("PriorityItem")],
                Some(simple_type("f64")),
            ),
            method_sig("display", vec![simple_type("PriorityItem")], None),
        ];

        let methods = vec![
            "format".to_string(),
            "calculate".to_string(),
            "display".to_string(),
        ];

        let primary = analyzer.identify_primary_type(&methods, &signatures);
        assert_eq!(primary.name, "PriorityItem");
    }

    #[test]
    fn test_cluster_by_type_affinity() {
        let analyzer = TypeAffinityAnalyzer;

        let signatures = vec![
            method_sig(
                "format_item",
                vec![simple_type("PriorityItem")],
                Some(simple_type("String")),
            ),
            method_sig(
                "calculate_score",
                vec![simple_type("PriorityItem")],
                Some(simple_type("f64")),
            ),
            method_sig(
                "format_metrics",
                vec![simple_type("Metrics")],
                Some(simple_type("String")),
            ),
        ];

        let clusters = analyzer.cluster_by_type_affinity(&signatures);

        // Should have at least one cluster
        assert!(!clusters.is_empty());

        // Each cluster should have methods
        for cluster in &clusters {
            assert!(!cluster.methods.is_empty());
        }
    }

    #[test]
    fn test_empty_signatures() {
        let analyzer = TypeAffinityAnalyzer;
        let clusters = analyzer.cluster_by_type_affinity(&[]);
        assert!(clusters.is_empty());
    }
}
