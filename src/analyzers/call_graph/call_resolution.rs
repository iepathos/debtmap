/// Call resolution logic for the call graph extractor
use crate::priority::call_graph::{CallGraph, CallType, FunctionId};
use std::path::PathBuf;

/// Represents an unresolved function call that needs to be resolved in phase 2
#[derive(Debug, Clone)]
pub struct UnresolvedCall {
    pub caller: FunctionId,
    pub callee_name: String,
    pub call_type: CallType,
    pub same_file_hint: bool, // Hint that this is likely a same-file call
}

/// Handles resolution of function calls
pub struct CallResolver<'a> {
    call_graph: &'a CallGraph,
    current_file: &'a PathBuf,
}

impl<'a> CallResolver<'a> {
    pub fn new(call_graph: &'a CallGraph, current_file: &'a PathBuf) -> Self {
        Self {
            call_graph,
            current_file,
        }
    }

    /// Resolve an unresolved call to a concrete function
    pub fn resolve_call(&self, call: &UnresolvedCall) -> Option<FunctionId> {
        let resolved_name = Self::normalize_path_prefix(&call.callee_name);

        // First try resolving in the same file if hinted
        if call.same_file_hint {
            if let Some(func) = self.resolve_in_same_file(&resolved_name, &call.callee_name) {
                return Some(func);
            }
        }

        // Then try global resolution
        self.resolve_function(&resolved_name, &call.callee_name, call.same_file_hint)
    }

    /// Normalize path prefixes in function names
    pub fn normalize_path_prefix(name: &str) -> String {
        if name.starts_with("crate::") || name.starts_with("self::") || name.starts_with("super::")
        {
            name.to_string()
        } else {
            name.to_string()
        }
    }

    /// Resolve a function call
    fn resolve_function(
        &self,
        resolved_name: &str,
        original_name: &str,
        same_file_hint: bool,
    ) -> Option<FunctionId> {
        // Get all functions in the graph
        let all_functions = self.call_graph.get_all_functions();

        // Find all matches
        let mut matches: Vec<&FunctionId> = all_functions
            .filter(|func| Self::matches_function_name(func, resolved_name, original_name))
            .collect();

        if matches.is_empty() {
            return None;
        }

        // Sort by qualification level (prefer less qualified matches for simple names)
        Self::sort_by_qualification(&mut matches);

        // If we have multiple matches, try to disambiguate
        if matches.len() > 1 && same_file_hint {
            matches = self.disambiguate_matches(matches, same_file_hint);
        }

        matches.first().map(|f| (*f).clone())
    }

    /// Resolve function in the same file
    fn resolve_in_same_file(&self, resolved_name: &str, original_name: &str) -> Option<FunctionId> {
        let current_file_str = self.current_file.to_string_lossy();

        // Get all functions in the current file
        let file_functions: Vec<&FunctionId> = self
            .call_graph
            .get_all_functions()
            .filter(|func| func.file == *self.current_file)
            .collect();

        // Find matches in the same file
        let mut matches: Vec<&FunctionId> = file_functions
            .into_iter()
            .filter(|func| Self::matches_function_name(func, resolved_name, original_name))
            .collect();

        if matches.is_empty() {
            return None;
        }

        // Sort by qualification
        Self::sort_by_qualification(&mut matches);

        // For same-file resolution, prefer the least qualified match
        matches.first().map(|f| (*f).clone())
    }

    /// Check if a function matches the given name
    pub fn matches_function_name(
        func: &&FunctionId,
        resolved_name: &str,
        original_name: &str,
    ) -> bool {
        let func_name = &func.name;

        // Direct match
        if func_name == resolved_name || func_name == original_name {
            return true;
        }

        // Check if function ends with the search name (for qualified names)
        if func_name.ends_with(&format!("::{}", resolved_name))
            || func_name.ends_with(&format!("::{}", original_name))
        {
            return true;
        }

        // Check for type-qualified names (e.g., MyStruct::method matches method)
        Self::matches_base_name_with_type_check(func_name, resolved_name)
            || Self::matches_base_name_with_type_check(func_name, original_name)
    }

    /// Check if a function base name matches, accounting for type qualification
    fn matches_base_name_with_type_check(func_name: &str, search_name: &str) -> bool {
        // Handle impl methods (Type::method)
        if let Some(pos) = func_name.rfind("::") {
            let base_name = &func_name[pos + 2..];
            if base_name == search_name {
                return true;
            }
        }

        // Handle module paths
        let func_parts: Vec<&str> = func_name.split("::").collect();
        let search_parts: Vec<&str> = search_name.split("::").collect();

        // If search has fewer parts, check if func ends with search pattern
        if search_parts.len() <= func_parts.len() {
            let func_suffix: Vec<&str> =
                func_parts[func_parts.len() - search_parts.len()..].to_vec();
            if func_suffix == search_parts {
                return true;
            }
        }

        false
    }

    /// Sort functions by qualification level (less qualified first)
    fn sort_by_qualification(matches: &mut Vec<&FunctionId>) {
        matches.sort_by_key(|func| {
            let qualification_level = func.name.matches("::").count();
            // Prioritize functions without impl blocks (lower qualification)
            let has_impl = func.name.contains("<") && func.name.contains(">");
            (has_impl as usize * 1000) + qualification_level
        });
    }

    /// Disambiguate multiple matches
    fn disambiguate_matches<'b>(
        &self,
        matches: Vec<&'b FunctionId>,
        same_file_hint: bool,
    ) -> Vec<&'b FunctionId> {
        if matches.len() <= 1 {
            return matches;
        }

        // Apply disambiguation strategies
        self.apply_disambiguation_strategies(matches, same_file_hint)
    }

    /// Apply various strategies to disambiguate matches
    fn apply_disambiguation_strategies<'b>(
        &self,
        matches: Vec<&'b FunctionId>,
        same_file_hint: bool,
    ) -> Vec<&'b FunctionId> {
        let mut filtered = matches.clone();

        // Strategy 1: Prefer same-file matches
        if same_file_hint {
            let same_file: Vec<&FunctionId> = filtered
                .iter()
                .filter(|f| f.file == *self.current_file)
                .copied()
                .collect();

            if !same_file.is_empty() {
                filtered = same_file;
            }
        }

        // Strategy 2: Prefer non-generic functions
        let non_generic: Vec<&FunctionId> = filtered
            .iter()
            .filter(|f| !f.name.contains("<") || !f.name.contains(">"))
            .copied()
            .collect();

        if !non_generic.is_empty() {
            filtered = non_generic;
        }

        // Strategy 3: Prefer shorter names (less qualified)
        if filtered.len() > 1 {
            let min_len = filtered.iter().map(|f| f.name.len()).min().unwrap_or(0);
            filtered = filtered
                .into_iter()
                .filter(|f| f.name.len() == min_len)
                .collect();
        }

        filtered
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
        } else if name.contains("::") {
            CallType::Direct
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_matches_base_name_with_type_check() {
        assert!(CallResolver::matches_base_name_with_type_check(
            "MyStruct::method",
            "method"
        ));
        assert!(CallResolver::matches_base_name_with_type_check(
            "module::MyStruct::method",
            "method"
        ));
        assert!(CallResolver::matches_base_name_with_type_check(
            "module::function",
            "function"
        ));
        assert!(!CallResolver::matches_base_name_with_type_check(
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
}
