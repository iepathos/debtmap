/// Call resolution logic for the call graph extractor
use crate::priority::call_graph::{CallGraph, CallType, FunctionId};
use std::path::PathBuf;

/// Extension trait for functional pipeline composition
trait FunctionalPipe<T> {
    fn pipe<F, U>(self, f: F) -> U
    where
        F: FnOnce(T) -> U;
}

impl<T> FunctionalPipe<T> for T {
    fn pipe<F, U>(self, f: F) -> U
    where
        F: FnOnce(T) -> U,
    {
        f(self)
    }
}

/// Call site type categorization for method disambiguation
#[derive(Debug, Clone, PartialEq)]
pub enum CallSiteType {
    /// Static/associated function: Type::function()
    Static,

    /// Instance method call: receiver.method()
    Instance { receiver_type: Option<String> },

    /// Trait method call (known trait): receiver.trait_method()
    TraitMethod {
        trait_name: String,
        receiver_type: Option<String>,
    },

    /// Call through function pointer or closure
    Indirect,
}

/// Represents an unresolved function call that needs to be resolved in phase 2
#[derive(Debug, Clone)]
pub struct UnresolvedCall {
    pub caller: FunctionId,
    pub callee_name: String,
    pub call_type: CallType,
    pub call_site_type: CallSiteType,
    pub same_file_hint: bool, // Hint that this is likely a same-file call
}

/// Handles resolution of function calls
use std::collections::HashMap;

pub struct CallResolver<'a> {
    #[allow(dead_code)]
    call_graph: &'a CallGraph,
    current_file: &'a PathBuf,
    function_index: HashMap<String, Vec<FunctionId>>,
}

impl<'a> CallResolver<'a> {
    pub fn new(call_graph: &'a CallGraph, current_file: &'a PathBuf) -> Self {
        // Build function name index once during construction
        let mut function_index: HashMap<String, Vec<FunctionId>> = HashMap::new();

        for func_id in call_graph.get_all_functions() {
            let key = Self::normalize_path_prefix(&func_id.name);
            function_index.entry(key).or_default().push(func_id.clone());

            // Also index by just the function name without qualification
            if let Some(simple_name) = func_id.name.split("::").last() {
                if simple_name != func_id.name {
                    function_index
                        .entry(simple_name.to_string())
                        .or_default()
                        .push(func_id.clone());
                }
            }
        }

        Self {
            call_graph,
            current_file,
            function_index,
        }
    }

    /// Resolve an unresolved call to a concrete function - now O(1) lookup!
    pub fn resolve_call(&self, call: &UnresolvedCall) -> Option<FunctionId> {
        // Exclude standard library trait methods from the call graph
        if let CallSiteType::TraitMethod { trait_name, .. } = &call.call_site_type {
            // Exclude known std traits
            if matches!(
                trait_name.as_str(),
                "Iterator"
                    | "Option"
                    | "Clone"
                    | "ToString"
                    | "Display"
                    | "Default"
                    | "Hash"
                    | "IteratorOrOption"
            ) {
                return None;
            }
        }

        // For instance calls with unknown receiver type, exclude if it's a std method
        if let CallSiteType::Instance {
            receiver_type: None,
        } = &call.call_site_type
        {
            if Self::is_std_trait_method(&call.callee_name) {
                // Conservative: assume it's a std library method
                return None;
            }
        }

        let normalized_name = Self::normalize_path_prefix(&call.callee_name);

        // Fast O(1) lookup instead of O(n) linear search
        let candidates = self.function_index.get(&normalized_name).or_else(|| {
            // Try looking up by simple name if qualified lookup fails
            if let Some(simple_name) = call.callee_name.split("::").last() {
                self.function_index.get(simple_name)
            } else {
                None
            }
        })?;

        // Filter candidates based on call site type
        let matching_candidates: Vec<FunctionId> = match &call.call_site_type {
            CallSiteType::Static => {
                // Static calls: require exact or qualified match
                candidates
                    .iter()
                    .filter(|func| {
                        Self::is_exact_match(&func.name, &normalized_name)
                            || Self::is_exact_match(&func.name, &call.callee_name)
                            || Self::is_qualified_match(&func.name, &normalized_name)
                            || Self::is_qualified_match(&func.name, &call.callee_name)
                    })
                    .cloned()
                    .collect()
            }
            CallSiteType::Instance {
                receiver_type: Some(recv_type),
            } => {
                // Instance call with known receiver: match Type::method
                let expected_name = format!(
                    "{}::{}",
                    recv_type,
                    call.callee_name
                        .split("::")
                        .last()
                        .unwrap_or(&call.callee_name)
                );
                candidates
                    .iter()
                    .filter(|func| {
                        func.name == expected_name
                            || func.name.starts_with(&format!("{}::", recv_type))
                    })
                    .cloned()
                    .collect()
            }
            CallSiteType::Instance {
                receiver_type: None,
            } => {
                // Instance call with unknown receiver: be conservative
                // Only exclude if it looks like a std trait method
                if Self::is_std_trait_method(&call.callee_name) {
                    return None;
                }

                if call.same_file_hint {
                    // Only match same-file functions
                    candidates
                        .iter()
                        .filter(|func| {
                            func.file == *self.current_file
                                && Self::is_function_match(
                                    func,
                                    &normalized_name,
                                    &call.callee_name,
                                )
                        })
                        .cloned()
                        .collect()
                } else {
                    // For non-std methods with unknown receiver, use base name matching
                    // This preserves existing behavior for user-defined methods
                    candidates
                        .iter()
                        .filter(|func| {
                            Self::is_function_match(func, &normalized_name, &call.callee_name)
                        })
                        .cloned()
                        .collect()
                }
            }
            CallSiteType::TraitMethod { receiver_type, .. } => {
                // Trait method call - try to resolve by receiver type
                if let Some(recv_type) = receiver_type {
                    let expected_name = format!(
                        "{}::{}",
                        recv_type,
                        call.callee_name
                            .split("::")
                            .last()
                            .unwrap_or(&call.callee_name)
                    );
                    candidates
                        .iter()
                        .filter(|func| func.name == expected_name)
                        .cloned()
                        .collect()
                } else if call.same_file_hint {
                    candidates
                        .iter()
                        .filter(|func| func.file == *self.current_file)
                        .cloned()
                        .collect()
                } else {
                    return None;
                }
            }
            CallSiteType::Indirect => {
                // Indirect call - use existing logic but prefer same-file
                candidates
                    .iter()
                    .filter(|func| {
                        Self::is_function_match(func, &normalized_name, &call.callee_name)
                    })
                    .cloned()
                    .collect()
            }
        };

        if matching_candidates.is_empty() {
            return None;
        }

        // Apply resolution strategies
        Self::select_best_candidate(matching_candidates, self.current_file, call.same_file_hint)
    }

    /// Pure function to resolve a function call against a list of candidates
    /// This is the core resolution logic extracted as a pure function
    pub fn resolve_function_call(
        all_functions: &[FunctionId],
        callee_name: &str,
        current_file: &PathBuf,
        same_file_hint: bool,
    ) -> Option<FunctionId> {
        let normalized_name = Self::normalize_path_prefix(callee_name);

        // Find all matching functions using functional pipeline
        let candidates: Vec<FunctionId> = all_functions
            .iter()
            .filter(|func| Self::is_function_match(func, &normalized_name, callee_name))
            .cloned()
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Apply resolution strategies in order of preference
        Self::select_best_candidate(candidates, current_file, same_file_hint)
    }

    /// Normalize path prefixes in function names
    pub fn normalize_path_prefix(name: &str) -> String {
        // Strip generic type parameters for better matching
        Self::strip_generic_params(name)
    }

    /// Strip generic type parameters from function names
    /// Examples: "foo<T>" -> "foo", "bar::<Type>" -> "bar"
    pub fn strip_generic_params(name: &str) -> String {
        // Handle turbofish syntax (::< >) and regular generics (< >)
        let without_turbofish = if let Some(pos) = name.find("::<") {
            // Find matching closing bracket
            if let Some(end) = Self::find_matching_bracket(&name[pos + 3..]) {
                format!("{}{}", &name[..pos], &name[pos + 3 + end + 1..])
            } else {
                name.to_string()
            }
        } else {
            name.to_string()
        };

        // Handle regular generics
        if let Some(pos) = without_turbofish.find('<') {
            if let Some(end) = Self::find_matching_bracket(&without_turbofish[pos + 1..]) {
                format!(
                    "{}{}",
                    &without_turbofish[..pos],
                    &without_turbofish[pos + 1 + end + 1..]
                )
            } else {
                without_turbofish
            }
        } else {
            without_turbofish
        }
    }

    /// Find matching closing bracket, accounting for nested brackets
    fn find_matching_bracket(s: &str) -> Option<usize> {
        let mut depth = 1;
        for (i, ch) in s.chars().enumerate() {
            match ch {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Pure function to select the best candidate from multiple matches
    /// Uses clear preference rules and functional composition
    /// Returns None if candidates are ambiguous (multiple equally valid options)
    fn select_best_candidate(
        candidates: Vec<FunctionId>,
        current_file: &PathBuf,
        same_file_hint: bool,
    ) -> Option<FunctionId> {
        if candidates.len() == 1 {
            return candidates.into_iter().next();
        }

        // If there are multiple candidates and no same-file hint, check for true ambiguity
        // We only consider it ambiguous if the candidates are truly indistinguishable:
        // - Same exact name (not one being Type::method and another being just method)
        // - All in different files
        // - No clear qualification differences
        if !same_file_hint && candidates.len() > 1 {
            // Check if all candidates have exactly the same name (not just matching)
            let all_same_name = candidates.iter().all(|f| f.name == candidates[0].name);

            if all_same_name {
                // Check if all candidates are in different files (truly ambiguous)
                let unique_files: std::collections::HashSet<_> =
                    candidates.iter().map(|f| &f.file).collect();

                // If all have the same exact name and are in different files, it's ambiguous
                if unique_files.len() == candidates.len() {
                    // All candidates are in different files with same exact name - this is ambiguous
                    return None;
                }
            }
        }

        // Apply selection strategies as a functional pipeline
        let result = candidates
            .pipe(|funcs| Self::apply_same_file_preference(funcs, current_file, same_file_hint))
            .pipe(Self::apply_qualification_preference)
            .pipe(Self::apply_generic_preference);

        // Return the best candidate after applying preferences
        result.into_iter().next()
    }

    /// Apply same-file preference filter
    fn apply_same_file_preference(
        candidates: Vec<FunctionId>,
        current_file: &PathBuf,
        same_file_hint: bool,
    ) -> Vec<FunctionId> {
        if !same_file_hint {
            return candidates;
        }

        let same_file_matches: Vec<FunctionId> = candidates
            .iter()
            .filter(|func| &func.file == current_file)
            .cloned()
            .collect();

        if same_file_matches.is_empty() {
            candidates
        } else {
            same_file_matches
        }
    }

    /// Apply qualification preference filter (prefer less qualified names)
    fn apply_qualification_preference(candidates: Vec<FunctionId>) -> Vec<FunctionId> {
        if candidates.len() <= 1 {
            return candidates;
        }

        let min_qualification = candidates
            .iter()
            .map(|func| Self::calculate_qualification_score(&func.name))
            .min()
            .unwrap_or(0);

        candidates
            .into_iter()
            .filter(|func| Self::calculate_qualification_score(&func.name) == min_qualification)
            .collect()
    }

    /// Apply generic function preference filter (prefer non-generic)
    fn apply_generic_preference(candidates: Vec<FunctionId>) -> Vec<FunctionId> {
        if candidates.len() <= 1 {
            return candidates;
        }

        let non_generic: Vec<FunctionId> = candidates
            .iter()
            .filter(|func| !Self::is_generic_function(&func.name))
            .cloned()
            .collect();

        if non_generic.is_empty() {
            candidates
        } else {
            non_generic
        }
    }

    /// Pure function to calculate qualification score
    fn calculate_qualification_score(name: &str) -> usize {
        let qualification_level = name.matches("::").count();
        let has_impl = name.contains("<") && name.contains(">");
        qualification_level + if has_impl { 1000 } else { 0 }
    }

    /// Pure function to check if function is generic
    fn is_generic_function(name: &str) -> bool {
        name.contains("<") && name.contains(">")
    }

    /// Pure function to check if a function matches the given name
    /// Simplified logic with clear precedence
    pub fn is_function_match(
        func: &FunctionId,
        normalized_name: &str,
        original_name: &str,
    ) -> bool {
        let func_name = &func.name;

        // 1. Exact match has highest priority
        if Self::is_exact_match(func_name, normalized_name)
            || Self::is_exact_match(func_name, original_name)
        {
            return true;
        }

        // 2. Qualified name match (e.g., "module::func" ends with "::func")
        if Self::is_qualified_match(func_name, normalized_name)
            || Self::is_qualified_match(func_name, original_name)
        {
            return true;
        }

        // 3. Base name match (e.g., "MyStruct::method" matches "method")
        Self::is_base_name_match(func_name, normalized_name)
            || Self::is_base_name_match(func_name, original_name)
    }

    /// Pure function for exact name matching
    fn is_exact_match(func_name: &str, search_name: &str) -> bool {
        func_name == search_name
    }

    /// Pure function for qualified name matching
    fn is_qualified_match(func_name: &str, search_name: &str) -> bool {
        func_name.ends_with(&format!("::{}", search_name))
    }

    /// Pure function for base name matching
    fn is_base_name_match(func_name: &str, search_name: &str) -> bool {
        // Handle impl methods (Type::method)
        if let Some(pos) = func_name.rfind("::") {
            let base_name = &func_name[pos + 2..];
            return base_name == search_name;
        }
        false
    }

    /// Extract impl type from a caller function name
    pub fn extract_impl_type_from_caller(caller_name: &str) -> Option<String> {
        // Look for Type:: pattern
        if let Some(pos) = caller_name.rfind("::") {
            let prefix = &caller_name[..pos];
            // Make sure it's not a module path
            if !prefix.contains("::") && prefix.chars().next()?.is_uppercase() {
                return Some(prefix.to_string());
            }
        }
        None
    }

    /// Classify the type of a function call
    pub fn classify_call_type(name: &str) -> CallType {
        if name.starts_with("self.") {
            CallType::Delegate
        } else {
            CallType::Direct
        }
    }

    /// Resolve self type references
    pub fn resolve_self_type(name: &str, current_impl_type: &Option<String>) -> String {
        if let Some(impl_type) = current_impl_type {
            name.replace("Self", impl_type)
        } else {
            name.to_string()
        }
    }

    /// Check if this is likely a same-file call
    pub fn is_same_file_call(name: &str, current_impl_type: &Option<String>) -> bool {
        // Simple unqualified names are likely same-file
        if !name.contains("::") && !name.starts_with("self.") {
            return true;
        }

        // Self type references are same-file
        if name.contains("Self::") && current_impl_type.is_some() {
            return true;
        }

        false
    }

    /// Check if an expression is a self receiver
    pub fn is_self_receiver(receiver: &syn::Expr) -> bool {
        matches!(receiver, syn::Expr::Path(path) if path.path.is_ident("self"))
    }

    /// Construct a method name from receiver type and method name
    pub fn construct_method_name(
        receiver_type: Option<String>,
        method_name: &str,
        current_impl_type: &Option<String>,
    ) -> String {
        if let Some(recv_type) = receiver_type {
            // Handle self references
            if recv_type == "Self" {
                if let Some(impl_type) = current_impl_type {
                    return format!("{}::{}", impl_type, method_name);
                }
            }
            format!("{}::{}", recv_type, method_name)
        } else {
            method_name.to_string()
        }
    }

    /// Check if a method name belongs to a standard library trait
    pub fn is_std_trait_method(method_name: &str) -> bool {
        matches!(
            method_name,
            // Iterator methods
            "any" | "all" | "map" | "filter" | "fold" | "reduce" |
            "collect" | "find" | "position" | "enumerate" | "zip" |
            "chain" | "flat_map" | "flatten" | "skip" | "take" |
            "cloned" | "copied" | "cycle" | "rev" | "peekable" |
            "for_each" | "nth" | "last" | "step_by" | "scan" |
            "fuse" | "inspect" | "partition" | "try_fold" | "try_for_each" |

            // Option/Result methods
            "unwrap" | "expect" | "unwrap_or" | "unwrap_or_else" |
            "and_then" | "or_else" | "is_some" | "is_none" |
            "is_ok" | "is_err" | "as_ref" | "as_mut" | "ok" | "err" |
            "transpose" | "unwrap_or_default" |

            // Common trait methods
            "clone" | "to_string" | "to_owned" | "into" | "from" |
            "default" | "eq" | "ne" | "cmp" | "partial_cmp" |
            "hash" | "fmt" | "display"
        )
    }

    /// Infer trait name from method name
    pub fn infer_trait_name(method_name: &str) -> String {
        match method_name {
            "any" | "all" | "filter" | "fold" | "reduce" | "collect" | "find" | "position"
            | "enumerate" | "zip" | "chain" | "flat_map" | "flatten" | "skip" | "take"
            | "cloned" | "copied" | "cycle" | "rev" | "peekable" | "for_each" | "nth" | "last"
            | "step_by" | "scan" | "fuse" | "inspect" | "partition" | "try_fold"
            | "try_for_each" => "Iterator".to_string(),

            "map" => "IteratorOrOption".to_string(), // Ambiguous

            "unwrap" | "expect" | "unwrap_or" | "unwrap_or_else" | "and_then" | "or_else"
            | "is_some" | "is_none" | "is_ok" | "is_err" | "as_ref" | "as_mut" | "ok" | "err"
            | "transpose" | "unwrap_or_default" => "Option".to_string(),

            "clone" => "Clone".to_string(),
            "to_string" | "display" => "ToString".to_string(),
            "fmt" => "Display".to_string(),
            "default" => "Default".to_string(),
            "hash" => "Hash".to_string(),

            _ => "Unknown".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_functional_refactoring_integration() {
        let current_file = PathBuf::from("test.rs");

        // Test the functional pipeline with owned values
        let functions = vec![
            FunctionId::new(current_file.clone(), "simple_func".to_string(), 10),
            FunctionId::new(current_file.clone(), "module::complex_func".to_string(), 20),
            FunctionId::new(PathBuf::from("other.rs"), "other_func".to_string(), 30),
        ];

        // Test resolution with same_file_hint
        let result =
            CallResolver::resolve_function_call(&functions, "simple_func", &current_file, true);

        assert!(result.is_some());
        let resolved = result.unwrap();
        assert_eq!(resolved.name, "simple_func");
        assert_eq!(resolved.file, current_file);

        // Test resolution without same_file_hint
        let result_no_hint =
            CallResolver::resolve_function_call(&functions, "simple_func", &current_file, false);

        assert!(result_no_hint.is_some());
        assert_eq!(result_no_hint.unwrap().name, "simple_func");
    }

    #[test]
    fn test_normalize_path_prefix() {
        assert_eq!(
            CallResolver::normalize_path_prefix("crate::module::func"),
            "crate::module::func"
        );
        assert_eq!(
            CallResolver::normalize_path_prefix("self::func"),
            "self::func"
        );
        assert_eq!(
            CallResolver::normalize_path_prefix("super::func"),
            "super::func"
        );
        assert_eq!(CallResolver::normalize_path_prefix("func"), "func");
    }

    #[test]
    fn test_is_base_name_match() {
        assert!(CallResolver::is_base_name_match(
            "MyStruct::method",
            "method"
        ));
        assert!(CallResolver::is_base_name_match(
            "module::MyStruct::method",
            "method"
        ));
        assert!(CallResolver::is_base_name_match(
            "module::function",
            "function"
        ));
        assert!(!CallResolver::is_base_name_match(
            "MyStruct::method",
            "other"
        ));
    }

    #[test]
    fn test_classify_call_type() {
        assert_eq!(
            CallResolver::classify_call_type("module::func"),
            CallType::Direct
        );
        assert_eq!(
            CallResolver::classify_call_type("Type::method"),
            CallType::Direct
        );
        assert_eq!(
            CallResolver::classify_call_type("self.method"),
            CallType::Delegate
        );
        assert_eq!(CallResolver::classify_call_type("func"), CallType::Direct);
    }

    #[test]
    fn test_resolve_self_type() {
        let impl_type = Some("MyStruct".to_string());
        assert_eq!(
            CallResolver::resolve_self_type("Self::new", &impl_type),
            "MyStruct::new"
        );
        assert_eq!(
            CallResolver::resolve_self_type("Self", &impl_type),
            "MyStruct"
        );

        let no_impl = None;
        assert_eq!(
            CallResolver::resolve_self_type("Self::new", &no_impl),
            "Self::new"
        );
    }

    #[test]
    fn test_is_same_file_call() {
        let impl_type = Some("MyStruct".to_string());

        assert!(CallResolver::is_same_file_call("simple_func", &None));
        assert!(CallResolver::is_same_file_call("Self::method", &impl_type));
        assert!(!CallResolver::is_same_file_call("module::func", &None));
        assert!(!CallResolver::is_same_file_call("self.method", &None));
    }

    #[test]
    fn test_extract_impl_type_from_caller() {
        assert_eq!(
            CallResolver::extract_impl_type_from_caller("MyStruct::method"),
            Some("MyStruct".to_string())
        );
        assert_eq!(
            CallResolver::extract_impl_type_from_caller("module::MyStruct::method"),
            None
        );
        assert_eq!(
            CallResolver::extract_impl_type_from_caller("function"),
            None
        );
    }

    #[test]
    fn test_complex_matching_scenarios() {
        let current_file = PathBuf::from("test.rs");

        // Mix of qualified and unqualified names
        let simple_func = FunctionId::new(current_file.clone(), "calculate".to_string(), 10);

        let qualified_func =
            FunctionId::new(current_file.clone(), "utils::calculate".to_string(), 20);

        let method_func = FunctionId::new(
            current_file.clone(),
            "Calculator::calculate".to_string(),
            30,
        );

        let functions = vec![
            qualified_func.clone(),
            method_func.clone(),
            simple_func.clone(),
        ];

        // When searching for "calculate" with same_file_hint, should prefer simpler match
        let result =
            CallResolver::resolve_function_call(&functions, "calculate", &current_file, true);

        assert!(result.is_some());
        // Should prefer the simple, unqualified name
        assert_eq!(result.unwrap().name, "calculate");
    }

    #[test]
    fn test_functional_pipeline_composition() {
        let current_file = PathBuf::from("test.rs");

        // Create functions with different qualification levels
        let functions = vec![
            FunctionId::new(current_file.clone(), "func".to_string(), 10),
            FunctionId::new(current_file.clone(), "mod::func".to_string(), 20),
            FunctionId::new(current_file.clone(), "deep::mod::func".to_string(), 30),
        ];

        // Test that qualification preference works
        let result = CallResolver::apply_qualification_preference(functions.clone());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "func"); // Least qualified should win

        // Test generic filtering
        let generic_functions = vec![
            FunctionId::new(current_file.clone(), "regular_func".to_string(), 10),
            FunctionId::new(current_file.clone(), "generic_func<T>".to_string(), 20),
        ];

        let result = CallResolver::apply_generic_preference(generic_functions.clone());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "regular_func"); // Non-generic should win
    }

    #[test]
    fn test_strip_generic_params() {
        // Test simple generic
        assert_eq!(CallResolver::strip_generic_params("foo<T>"), "foo");

        // Test turbofish syntax
        assert_eq!(CallResolver::strip_generic_params("bar::<Type>"), "bar");

        // Test nested generics
        assert_eq!(
            CallResolver::strip_generic_params("func<Vec<String>>"),
            "func"
        );

        // Test multiple type parameters
        assert_eq!(CallResolver::strip_generic_params("map<K, V>"), "map");

        // Test qualified name with generics
        assert_eq!(
            CallResolver::strip_generic_params("module::function<T>"),
            "module::function"
        );

        // Test method with generics
        assert_eq!(
            CallResolver::strip_generic_params("Type::method<T, U>"),
            "Type::method"
        );

        // Test non-generic function (should return unchanged)
        assert_eq!(
            CallResolver::strip_generic_params("simple_function"),
            "simple_function"
        );

        // Test complex nested generics
        assert_eq!(
            CallResolver::strip_generic_params("complex<HashMap<String, Vec<i32>>>"),
            "complex"
        );
    }

    #[test]
    fn test_find_matching_bracket() {
        // Simple case
        assert_eq!(CallResolver::find_matching_bracket("T>"), Some(1));

        // Nested brackets
        assert_eq!(
            CallResolver::find_matching_bracket("Vec<String>>"),
            Some(11)
        );

        // Multiple levels of nesting
        assert_eq!(
            CallResolver::find_matching_bracket("HashMap<String, Vec<i32>>>"),
            Some(25)
        );

        // No closing bracket
        assert_eq!(CallResolver::find_matching_bracket("T"), None);

        // Mismatched brackets
        assert_eq!(CallResolver::find_matching_bracket("T<U"), None);
    }
}

// Summary: Refactored call resolution using functional programming principles
// - Replaced complex lifetime management with owned values
// - Used functional composition with pipe() for clean data flow
// - Made all functions pure and side-effect free
// - Eliminated mutable state in favor of immutable transformations
