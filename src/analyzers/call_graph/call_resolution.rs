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
        let all_functions: Vec<FunctionId> = self.call_graph.get_all_functions().cloned().collect();
        
        // Use functional approach with clear precedence rules
        Self::resolve_function_call(
            &all_functions,
            &call.callee_name,
            &self.current_file,
            call.same_file_hint
        )
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
        if name.starts_with("crate::") || name.starts_with("self::") || name.starts_with("super::")
        {
            name.to_string()
        } else {
            name.to_string()
        }
    }

    /// Pure function to select the best candidate from multiple matches
    /// Uses clear preference rules and functional composition
    fn select_best_candidate(
        candidates: Vec<FunctionId>,
        current_file: &PathBuf,
        same_file_hint: bool,
    ) -> Option<FunctionId> {
        if candidates.len() == 1 {
            return candidates.into_iter().next();
        }
        
        // Apply selection strategies as a functional pipeline
        let result = candidates
            .into_iter()
            .collect::<Vec<_>>()
            .pipe(|funcs| Self::apply_same_file_preference(funcs, current_file, same_file_hint))
            .pipe(Self::apply_qualification_preference)
            .pipe(Self::apply_generic_preference);
            
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
            || Self::is_exact_match(func_name, original_name) {
            return true;
        }
        
        // 2. Qualified name match (e.g., "module::func" ends with "::func")
        if Self::is_qualified_match(func_name, normalized_name)
            || Self::is_qualified_match(func_name, original_name) {
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
    fn test_functional_refactoring_integration() {
        let current_file = PathBuf::from("test.rs");
        
        // Test the functional pipeline with owned values
        let functions = vec![
            FunctionId {
                file: current_file.clone(),
                name: "simple_func".to_string(),
                line: 10,
            },
            FunctionId {
                file: current_file.clone(),
                name: "module::complex_func".to_string(),
                line: 20,
            },
            FunctionId {
                file: PathBuf::from("other.rs"),
                name: "other_func".to_string(),
                line: 30,
            },
        ];
        
        // Test resolution with same_file_hint
        let result = CallResolver::resolve_function_call(
            &functions,
            "simple_func",
            &current_file,
            true
        );
        
        assert!(result.is_some());
        let resolved = result.unwrap();
        assert_eq!(resolved.name, "simple_func");
        assert_eq!(resolved.file, current_file);
        
        // Test resolution without same_file_hint
        let result_no_hint = CallResolver::resolve_function_call(
            &functions,
            "simple_func",
            &current_file,
            false
        );
        
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
        let simple_func = FunctionId {
            file: current_file.clone(),
            name: "calculate".to_string(),
            line: 10,
        };
        
        let qualified_func = FunctionId {
            file: current_file.clone(),
            name: "utils::calculate".to_string(),
            line: 20,
        };
        
        let method_func = FunctionId {
            file: current_file.clone(),
            name: "Calculator::calculate".to_string(),
            line: 30,
        };
        
        let functions = vec![qualified_func.clone(), method_func.clone(), simple_func.clone()];
        
        // When searching for "calculate" with same_file_hint, should prefer simpler match
        let result = CallResolver::resolve_function_call(
            &functions,
            "calculate",
            &current_file,
            true
        );
        
        assert!(result.is_some());
        // Should prefer the simple, unqualified name
        assert_eq!(result.unwrap().name, "calculate");
    }
    
    #[test]
    fn test_functional_pipeline_composition() {
        let current_file = PathBuf::from("test.rs");
        
        // Create functions with different qualification levels
        let functions = vec![
            FunctionId {
                file: current_file.clone(),
                name: "func".to_string(),
                line: 10,
            },
            FunctionId {
                file: current_file.clone(),
                name: "mod::func".to_string(),
                line: 20,
            },
            FunctionId {
                file: current_file.clone(),
                name: "deep::mod::func".to_string(),
                line: 30,
            },
        ];
        
        // Test that qualification preference works
        let result = CallResolver::apply_qualification_preference(functions.clone());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "func"); // Least qualified should win
        
        // Test generic filtering
        let generic_functions = vec![
            FunctionId {
                file: current_file.clone(),
                name: "regular_func".to_string(),
                line: 10,
            },
            FunctionId {
                file: current_file.clone(),
                name: "generic_func<T>".to_string(),
                line: 20,
            },
        ];
        
        let result = CallResolver::apply_generic_preference(generic_functions.clone());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "regular_func"); // Non-generic should win
    }
}

// Summary: Refactored call resolution using functional programming principles
// - Replaced complex lifetime management with owned values
// - Used functional composition with pipe() for clean data flow
// - Made all functions pure and side-effect free
// - Eliminated mutable state in favor of immutable transformations
