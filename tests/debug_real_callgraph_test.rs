use debtmap::analyzers::rust_call_graph::extract_call_graph;
use debtmap::core::FunctionMetrics;
use debtmap::priority::unified_scorer::create_unified_debt_item_enhanced;
use std::path::PathBuf;

#[test]
fn test_real_world_call_graph_extraction() {
    // Test case matching exactly what we see in the debtmap output
    let code = r#"
pub struct CallGraphExtractor {
    pub call_graph: CallGraph,
    unresolved_calls: Vec<UnresolvedCall>,
    current_function: Option<FunctionId>,
    current_impl_type: Option<String>,
    current_file: PathBuf,
}

impl CallGraphExtractor {
    pub fn new(file: PathBuf) -> Self {
        Self {
            call_graph: CallGraph::new(),
            unresolved_calls: Vec::new(),
            current_function: None,
            current_impl_type: None,
            current_file: file,
        }
    }

    fn resolve_function(&self, name: &str, caller: &FunctionId, same_file_hint: bool) -> Option<FunctionId> {
        let all_functions = self.call_graph.find_all_functions();
        
        // If same_file_hint is true, prioritize same-file matches
        if same_file_hint {
            // First try exact match in same file
            if let Some(func) = all_functions.iter().find(|f| 
                f.name == name && f.file == caller.file
            ) {
                return Some(func.clone());
            }
            
            // For method calls, try with type prefix
            if let Some(impl_type) = self.extract_impl_type_from_caller(&caller.name) {
                let qualified_name = format!("{}::{}", impl_type, name);
                if let Some(func) = all_functions.iter().find(|f| 
                    f.name == qualified_name && f.file == caller.file
                ) {
                    return Some(func.clone());
                }
            }
        }
        
        None
    }
    
    fn extract_impl_type_from_caller(&self, name: &str) -> Option<String> {
        if name.contains("::") {
            let parts: Vec<&str> = name.split("::").collect();
            if parts.len() >= 2 {
                return Some(parts[0].to_string());
            }
        }
        None
    }
    
    fn visit_expr(&mut self, expr: &syn::Expr) {
        match expr {
            syn::Expr::Call(call) => self.visit_call_expr(call),
            syn::Expr::MethodCall(method) => self.visit_method_call(method),
            _ => {}
        }
        
        // Recurse
        syn::visit::visit_expr(self, expr);
    }
    
    fn visit_call_expr(&mut self, call: &syn::ExprCall) {
        // Handle function calls
        if let Some(current_fn) = &self.current_function {
            if let syn::Expr::Path(path_expr) = &*call.func {
                let path_str = self.path_to_string(&path_expr.path);
                self.unresolved_calls.push(UnresolvedCall {
                    caller: current_fn.clone(),
                    callee_name: path_str,
                    call_type: CallType::Direct,
                    same_file_hint: true,
                });
            }
        }
    }
    
    fn visit_method_call(&mut self, method: &syn::ExprMethodCall) {
        // Handle method calls
        if let Some(current_fn) = &self.current_function {
            let method_name = method.method.to_string();
            self.unresolved_calls.push(UnresolvedCall {
                caller: current_fn.clone(),
                callee_name: method_name,
                call_type: CallType::Method,
                same_file_hint: true,
            });
        }
    }
    
    fn path_to_string(&self, path: &syn::Path) -> String {
        path.segments.iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    }
}

// Related types for testing
#[derive(Debug, Clone)]
struct UnresolvedCall {
    caller: FunctionId,
    callee_name: String,
    call_type: CallType,
    same_file_hint: bool,
}

#[derive(Debug)]
enum CallType {
    Direct,
    Method,
}

pub struct FrameworkPatternDetector;

impl FrameworkPatternDetector {
    pub fn get_exclusions(&self) -> Vec<String> {
        self.get_standard_exclusions()
            .into_iter()
            .chain(self.get_test_exclusions())
            .collect()
    }
    
    fn get_standard_exclusions(&self) -> Vec<String> {
        vec!["target".to_string(), "node_modules".to_string()]
    }
    
    fn get_test_exclusions(&self) -> Vec<String> {
        vec!["tests".to_string(), "test".to_string()]
    }
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Debug: Print all found functions
    let all_functions = call_graph.find_all_functions();
    println!("\n=== All Functions Found ===");
    for func in &all_functions {
        println!("  - {} at line {}", func.name, func.line);
    }

    // Look for specific functions from debtmap output
    let resolve_function = all_functions
        .iter()
        .find(|f| f.name.contains("resolve_function"));
    let visit_expr = all_functions.iter().find(|f| f.name.contains("visit_expr"));
    let get_exclusions = all_functions
        .iter()
        .find(|f| f.name.contains("get_exclusions"));

    println!("\n=== Looking for Functions ===");
    println!("resolve_function: {:?}", resolve_function);
    println!("visit_expr: {:?}", visit_expr);
    println!("get_exclusions: {:?}", get_exclusions);

    // Check if functions have callers/callees
    if let Some(resolve_fn) = resolve_function {
        let callers = call_graph.get_callers(resolve_fn);
        let callees = call_graph.get_callees(resolve_fn);
        println!("\n=== resolve_function Dependencies ===");
        println!("Callers: {:?}", callers);
        println!("Callees: {:?}", callees);

        // Test with FunctionMetrics to see what UnifiedDebtItem would show
        let metrics = FunctionMetrics {
            file: resolve_fn.file.clone(),
            name: resolve_fn.name.clone(),
            line: resolve_fn.line,
            cyclomatic: 5,
            cognitive: 7,
            length: 30,
            nesting: 2,
            is_test: false,
            visibility: Some("pub".to_string()),
            is_trait_method: false,
        };

        let debt_item = create_unified_debt_item_enhanced(&metrics, &call_graph, None, None, 5.0);

        println!("\n=== UnifiedDebtItem for resolve_function ===");
        println!("Upstream callers: {:?}", debt_item.upstream_callers);
        println!("Downstream callees: {:?}", debt_item.downstream_callees);

        assert!(
            !debt_item.downstream_callees.is_empty(),
            "resolve_function should have callees (find_all_functions, extract_impl_type_from_caller)"
        );
    }

    if let Some(visit_expr_fn) = visit_expr {
        let callers = call_graph.get_callers(visit_expr_fn);
        let callees = call_graph.get_callees(visit_expr_fn);
        println!("\n=== visit_expr Dependencies ===");
        println!("Callers: {:?}", callers);
        println!("Callees: {:?}", callees);

        // Should call visit_call_expr and visit_method_call
        assert!(!callees.is_empty(), "visit_expr should have callees");
    }

    if let Some(get_exclusions_fn) = get_exclusions {
        let callers = call_graph.get_callers(get_exclusions_fn);
        let callees = call_graph.get_callees(get_exclusions_fn);
        println!("\n=== get_exclusions Dependencies ===");
        println!("Callers: {:?}", callers);
        println!("Callees: {:?}", callees);

        // Should call get_standard_exclusions and get_test_exclusions
        assert!(!callees.is_empty(), "get_exclusions should have callees");
    }
}

#[test]
fn test_simple_call_graph_to_verify_system_works() {
    // Very simple test to verify the system works at all
    let code = r#"
fn main() {
    helper();
}

fn helper() {
    println!("test");
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("simple.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    let all_functions = call_graph.find_all_functions();
    println!("\n=== Simple Test Functions ===");
    for func in &all_functions {
        println!("  - {} at line {}", func.name, func.line);
        let callees = call_graph.get_callees(func);
        println!("    Callees: {:?}", callees);
    }

    // Find main function
    let main_fn = all_functions
        .iter()
        .find(|f| f.name == "main")
        .expect("Should find main");

    let main_callees = call_graph.get_callees(main_fn);
    assert!(!main_callees.is_empty(), "main should call helper");
    assert_eq!(main_callees[0].name, "helper");
}
