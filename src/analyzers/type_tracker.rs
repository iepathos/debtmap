use crate::analyzers::function_registry::{FunctionSignatureRegistry, ReturnTypeInfo};
use crate::analyzers::type_registry::GlobalTypeRegistry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use syn::{
    Expr, ExprCall, ExprField, ExprMethodCall, ExprPath, ExprStruct, FnArg, ImplItemFn, ItemFn,
    Pat, PatIdent, PatType, Type, TypePath,
};

/// Tracks variable types within the current analysis scope
#[derive(Debug, Clone)]
pub struct TypeTracker {
    /// Stack of scopes, innermost last
    scopes: Vec<Scope>,
    /// Current module path for resolving imports
    module_path: Vec<String>,
    /// Type definitions found in the file
    #[allow(dead_code)]
    type_definitions: HashMap<String, TypeInfo>,
    /// Global type registry for struct field resolution
    type_registry: Option<Arc<GlobalTypeRegistry>>,
    /// Function signature registry for return type resolution
    function_registry: Option<Arc<FunctionSignatureRegistry>>,
    /// Current file path for import resolution
    current_file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Scope {
    /// Variable name to type mapping
    variables: HashMap<String, ResolvedType>,
    /// Scope kind (function, block, impl, module)
    kind: ScopeKind,
    /// Parent type for impl blocks
    impl_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedType {
    /// Fully qualified type name
    pub type_name: String,
    /// Source location where type was determined
    pub source: TypeSource,
    /// Generic parameters if any
    pub generics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeSource {
    /// Explicit type annotation
    Annotation,
    /// Constructor call (e.g., Type::new())
    Constructor,
    /// Struct literal
    StructLiteral,
    /// Function return type
    FunctionReturn,
    /// Field access
    FieldAccess,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeKind {
    Module,
    Function,
    Block,
    Impl,
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub kind: TypeKind,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Struct,
    Enum,
    Trait,
    TypeAlias,
}

impl Default for TypeTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeTracker {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope {
                variables: HashMap::new(),
                kind: ScopeKind::Module,
                impl_type: None,
            }],
            module_path: Vec::new(),
            type_definitions: HashMap::new(),
            type_registry: None,
            function_registry: None,
            current_file: None,
        }
    }

    /// Create a TypeTracker with a shared type registry
    pub fn with_registry(registry: Arc<GlobalTypeRegistry>) -> Self {
        Self {
            scopes: vec![Scope {
                variables: HashMap::new(),
                kind: ScopeKind::Module,
                impl_type: None,
            }],
            module_path: Vec::new(),
            type_definitions: HashMap::new(),
            type_registry: Some(registry),
            function_registry: None,
            current_file: None,
        }
    }

    /// Set the function registry for return type resolution
    pub fn set_function_registry(&mut self, registry: Arc<FunctionSignatureRegistry>) {
        self.function_registry = Some(registry);
    }

    /// Set the current file path for import resolution
    pub fn set_current_file(&mut self, file: PathBuf) {
        self.current_file = Some(file);
    }

    /// Set the module path
    pub fn set_module_path(&mut self, path: Vec<String>) {
        self.module_path = path;
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self, kind: ScopeKind, impl_type: Option<String>) {
        self.scopes.push(Scope {
            variables: HashMap::new(),
            kind,
            impl_type,
        });
    }

    /// Exit the current scope
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Record a variable with its type
    pub fn record_variable(&mut self, name: String, resolved_type: ResolvedType) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.variables.insert(name, resolved_type);
        }
    }

    /// Resolve a variable's type by looking through the scope stack
    pub fn resolve_variable_type(&self, name: &str) -> Option<ResolvedType> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(resolved_type) = scope.variables.get(name) {
                return Some(resolved_type.clone());
            }
        }
        None
    }

    /// Resolve the type of an expression
    pub fn resolve_expr_type(&self, expr: &Expr) -> Option<ResolvedType> {
        match expr {
            Expr::Path(ExprPath { path, .. }) => {
                // Check if it's a variable reference
                if path.segments.len() == 1 {
                    let ident = path.segments.first()?.ident.to_string();
                    // Special handling for "self"
                    if ident == "self" {
                        if let Some(impl_type) = self.current_impl_type() {
                            return Some(ResolvedType {
                                type_name: impl_type,
                                source: TypeSource::Annotation,
                                generics: Vec::new(),
                            });
                        }
                    }

                    // Try to resolve as a variable first
                    if let Some(resolved) = self.resolve_variable_type(&ident) {
                        return Some(resolved);
                    }

                    // If not a variable, check if it looks like a type (starts with uppercase)
                    // This handles unit structs: `let h = Handler;` where Handler is a unit struct
                    // But avoids treating function calls like `new()` as types
                    if ident.chars().next()?.is_uppercase() {
                        return Some(ResolvedType {
                            type_name: ident,
                            source: TypeSource::Annotation,
                            generics: Vec::new(),
                        });
                    }

                    None
                } else {
                    // It's a path to a type or function
                    None
                }
            }
            Expr::Struct(expr_struct) => {
                // Resolve struct literal type from its path
                self.resolve_struct_literal_type(expr_struct)
            }
            Expr::MethodCall(method_call) => {
                // Resolve method call return type
                self.resolve_method_call_type(method_call)
            }
            Expr::Field(field_expr) => {
                // Resolve field access through type registry
                self.resolve_field_access(field_expr)
            }
            Expr::Call(call_expr) => {
                // Resolve function call return type
                self.resolve_function_call_type(call_expr)
            }
            _ => None,
        }
    }

    /// Resolve struct literal type from its path
    fn resolve_struct_literal_type(&self, expr_struct: &ExprStruct) -> Option<ResolvedType> {
        // Extract type name from the struct path
        let type_name = expr_struct
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())?;

        Some(ResolvedType {
            type_name,
            source: TypeSource::StructLiteral,
            generics: Vec::new(),
        })
    }

    /// Resolve field access using the type registry
    fn resolve_field_access(&self, field_expr: &ExprField) -> Option<ResolvedType> {
        // First resolve the base expression type
        let base_type = self.resolve_expr_type(&field_expr.base)?;

        // Get the field name
        let field_name = match &field_expr.member {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(index) => {
                // Handle tuple field access
                if let Some(registry) = &self.type_registry {
                    let field_type =
                        registry.resolve_tuple_field(&base_type.type_name, index.index as usize)?;
                    return Some(ResolvedType {
                        type_name: field_type.type_name,
                        source: TypeSource::FieldAccess,
                        generics: Vec::new(),
                    });
                }
                return None;
            }
        };

        // Look up the field type in the registry
        if let Some(registry) = &self.type_registry {
            // Try to resolve the type with imports if needed
            let resolved_type_name = if let Some(file) = &self.current_file {
                registry
                    .resolve_type_with_imports(file, &base_type.type_name)
                    .unwrap_or_else(|| base_type.type_name.clone())
            } else {
                base_type.type_name.clone()
            };

            // Now resolve the field
            if let Some(field_type) = registry.resolve_field(&resolved_type_name, &field_name) {
                return Some(ResolvedType {
                    type_name: field_type.type_name,
                    source: TypeSource::FieldAccess,
                    generics: field_type.generic_args,
                });
            }
        }

        None
    }

    /// Get the current impl type if in an impl block
    pub fn current_impl_type(&self) -> Option<String> {
        for scope in self.scopes.iter().rev() {
            if scope.kind == ScopeKind::Impl {
                return scope.impl_type.clone();
            }
        }
        None
    }

    /// Check if a method name is a constructor pattern
    fn is_constructor_method(method_name: &str) -> bool {
        matches!(
            method_name,
            "new" | "default" | "from" | "create" | "builder"
        )
    }

    /// Extract function path from an expression
    fn extract_function_path(func_expr: &Expr) -> Option<String> {
        if let Expr::Path(path_expr) = func_expr {
            Some(
                path_expr
                    .path
                    .segments
                    .iter()
                    .map(|seg| seg.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::"),
            )
        } else {
            None
        }
    }

    /// Parse a function path into type and method components
    fn parse_function_path(func_path: &str) -> Option<(String, String)> {
        if func_path.contains("::") {
            let parts: Vec<&str> = func_path.split("::").collect();
            if parts.len() >= 2 {
                let type_name = parts[..parts.len() - 1].join("::");
                let method_name = parts.last().unwrap().to_string();
                return Some((type_name, method_name));
            }
        }
        None
    }

    /// Resolve the actual type name considering Self references
    fn resolve_type_name(&self, return_info: &ReturnTypeInfo) -> String {
        if return_info.is_self {
            self.current_impl_type()
                .unwrap_or_else(|| return_info.type_name.clone())
        } else {
            return_info.type_name.clone()
        }
    }

    /// Create a resolved type from return information
    fn create_resolved_type(
        type_name: String,
        source: TypeSource,
        generics: Vec<String>,
    ) -> ResolvedType {
        ResolvedType {
            type_name,
            source,
            generics,
        }
    }

    /// Resolve function call return type
    fn resolve_function_call_type(&self, call_expr: &ExprCall) -> Option<ResolvedType> {
        let func_path = Self::extract_function_path(&call_expr.func)?;

        // Check if it's a constructor pattern FIRST (before requiring registry)
        // This handles Type::new(), Type::default(), etc. even without full type information
        if let Some((type_name, method_name)) = Self::parse_function_path(&func_path) {
            if Self::is_constructor_method(&method_name) {
                // If we have a registry, check if it has specific return type info
                if let Some(registry) = &self.function_registry {
                    if let Some(return_info) =
                        registry.resolve_method_return(&type_name, &method_name)
                    {
                        let resolved_type_name = if return_info.is_self {
                            type_name
                        } else {
                            return_info.type_name.clone()
                        };
                        return Some(Self::create_resolved_type(
                            resolved_type_name,
                            TypeSource::Constructor,
                            return_info.generic_args.clone(),
                        ));
                    }
                }

                // For common constructors (new, default), assume they return the type itself
                // This is a reasonable heuristic that works in most cases
                if matches!(method_name.as_str(), "new" | "default" | "from") {
                    return Some(Self::create_resolved_type(
                        type_name,
                        TypeSource::Constructor,
                        Vec::new(),
                    ));
                }
            }
        }

        // Try to resolve via registry (if available)
        if let Some(registry) = &self.function_registry {
            if let Some(return_info) = registry.resolve_function_return(&func_path, &[]) {
                let type_name = self.resolve_type_name(&return_info);
                return Some(Self::create_resolved_type(
                    type_name,
                    TypeSource::FunctionReturn,
                    return_info.generic_args.clone(),
                ));
            }
        }

        None
    }

    /// Resolve method call return type
    fn resolve_method_call_type(&self, method_call: &ExprMethodCall) -> Option<ResolvedType> {
        // First resolve the receiver type
        let receiver_type = self.resolve_expr_type(&method_call.receiver)?;
        let method_name = method_call.method.to_string();

        if let Some(registry) = &self.function_registry {
            // Try to get the method's return type
            if let Some(return_info) =
                registry.resolve_method_return(&receiver_type.type_name, &method_name)
            {
                let type_name = if return_info.is_self {
                    // Method returns Self, so it returns the receiver type
                    receiver_type.type_name.clone()
                } else {
                    return_info.type_name.clone()
                };

                return Some(ResolvedType {
                    type_name,
                    source: TypeSource::FunctionReturn,
                    generics: return_info.generic_args.clone(),
                });
            }

            // Check if this is a builder pattern
            if registry.is_builder(&receiver_type.type_name) {
                if let Some(builder_info) = registry.get_builder(&receiver_type.type_name) {
                    if method_name == builder_info.build_method {
                        // This is the terminal build method
                        return Some(ResolvedType {
                            type_name: builder_info.target_type.clone(),
                            source: TypeSource::FunctionReturn,
                            generics: Vec::new(),
                        });
                    } else if builder_info.chain_methods.contains(&method_name) {
                        // This is a chaining method, returns the builder itself
                        return Some(receiver_type);
                    }
                }
            }
        }

        // Default: method calls typically don't change the type
        // unless we have specific information
        None
    }

    /// Track self parameter in a function
    pub fn track_self_param(&mut self, fn_item: Option<&ItemFn>, impl_fn: Option<&ImplItemFn>) {
        // Handle impl method
        if let Some(impl_fn) = impl_fn {
            if let Some(_self_param) = extract_self_param(&impl_fn.sig) {
                if let Some(impl_type) = self.current_impl_type() {
                    let resolved_type = ResolvedType {
                        type_name: impl_type,
                        source: TypeSource::Annotation,
                        generics: Vec::new(),
                    };
                    self.record_variable("self".to_string(), resolved_type);
                }
            }
        }
        // Handle regular function (shouldn't have self, but check anyway)
        if let Some(fn_item) = fn_item {
            if let Some(_self_param) = extract_self_param(&fn_item.sig) {
                if let Some(impl_type) = self.current_impl_type() {
                    let resolved_type = ResolvedType {
                        type_name: impl_type,
                        source: TypeSource::Annotation,
                        generics: Vec::new(),
                    };
                    self.record_variable("self".to_string(), resolved_type);
                }
            }
        }
    }
}

/// Extract self parameter information from a function signature
pub fn extract_self_param(sig: &syn::Signature) -> Option<SelfParam> {
    if let Some(FnArg::Receiver(receiver)) = sig.inputs.first() {
        Some(SelfParam {
            is_reference: receiver.reference.is_some(),
            is_mutable: receiver.mutability.is_some(),
        })
    } else {
        None
    }
}

/// Information about a self parameter
#[derive(Debug, Clone)]
pub struct SelfParam {
    pub is_reference: bool,
    pub is_mutable: bool,
}

/// Extract type from various AST patterns
pub fn extract_type_from_pattern(pat: &Pat, init: &Option<Box<Expr>>) -> Option<ResolvedType> {
    match pat {
        Pat::Type(PatType { ty, .. }) => {
            // Explicit type annotation: let x: Type = ...
            Some(extract_type_from_type(ty))
        }
        Pat::Ident(PatIdent { .. }) if init.is_some() => {
            // Type inference from initializer
            extract_type_from_expr(init.as_ref().unwrap())
        }
        _ => None,
    }
}

/// Extract type from a Type AST node
pub fn extract_type_from_type(ty: &Type) -> ResolvedType {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let type_name = path
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            ResolvedType {
                type_name,
                source: TypeSource::Annotation,
                generics: Vec::new(),
            }
        }
        _ => ResolvedType {
            type_name: "Unknown".to_string(),
            source: TypeSource::Annotation,
            generics: Vec::new(),
        },
    }
}

/// Extract type from an expression (for type inference)
pub fn extract_type_from_expr(expr: &Expr) -> Option<ResolvedType> {
    match expr {
        Expr::Call(ExprCall { func, .. }) => {
            // Constructor call: Type::new()
            if let Expr::Path(ExprPath { path, .. }) = &**func {
                extract_type_from_constructor_path(path)
            } else {
                None
            }
        }
        Expr::Struct(ExprStruct { path, .. }) => {
            // Struct literal: Type { field: value }
            Some(extract_type_from_path(path))
        }
        _ => None,
    }
}

/// Extract type from a constructor path (e.g., DependencyGraph::new)
fn extract_type_from_constructor_path(path: &syn::Path) -> Option<ResolvedType> {
    // For constructors like Type::new(), extract "Type"
    if path.segments.len() >= 2 {
        // Get all segments except the last one (which is usually "new")
        let type_segments: Vec<String> = path
            .segments
            .iter()
            .take(path.segments.len() - 1)
            .map(|seg| seg.ident.to_string())
            .collect();

        if !type_segments.is_empty() {
            return Some(ResolvedType {
                type_name: type_segments.join("::"),
                source: TypeSource::Constructor,
                generics: Vec::new(),
            });
        }
    }
    None
}

/// Extract type from a path (for struct literals)
fn extract_type_from_path(path: &syn::Path) -> ResolvedType {
    let type_name = path
        .segments
        .iter()
        .map(|seg| seg.ident.to_string())
        .collect::<Vec<_>>()
        .join("::");

    ResolvedType {
        type_name,
        source: TypeSource::StructLiteral,
        generics: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::function_registry::{FunctionSignature, MethodSignature, VisibilityInfo};

    #[test]
    fn test_is_constructor_method() {
        // Test common constructor methods
        assert!(TypeTracker::is_constructor_method("new"));
        assert!(TypeTracker::is_constructor_method("default"));
        assert!(TypeTracker::is_constructor_method("from"));
        assert!(TypeTracker::is_constructor_method("create"));
        assert!(TypeTracker::is_constructor_method("builder"));

        // Test non-constructor methods
        assert!(!TypeTracker::is_constructor_method("get"));
        assert!(!TypeTracker::is_constructor_method("set"));
        assert!(!TypeTracker::is_constructor_method("update"));
        assert!(!TypeTracker::is_constructor_method("delete"));
        assert!(!TypeTracker::is_constructor_method("process"));
    }

    #[test]
    fn test_extract_function_path() {
        // Test path expression
        let code = "MyType::new()";
        let expr: Expr = syn::parse_str(code).unwrap();
        if let Expr::Call(call) = expr {
            let path = TypeTracker::extract_function_path(&call.func);
            assert_eq!(path, Some("MyType::new".to_string()));
        }

        // Test simple function call
        let code = "process_data()";
        let expr: Expr = syn::parse_str(code).unwrap();
        if let Expr::Call(call) = expr {
            let path = TypeTracker::extract_function_path(&call.func);
            assert_eq!(path, Some("process_data".to_string()));
        }

        // Test nested module path
        let code = "std::collections::HashMap::new()";
        let expr: Expr = syn::parse_str(code).unwrap();
        if let Expr::Call(call) = expr {
            let path = TypeTracker::extract_function_path(&call.func);
            assert_eq!(path, Some("std::collections::HashMap::new".to_string()));
        }
    }

    #[test]
    fn test_parse_function_path() {
        // Test simple type::method pattern
        let result = TypeTracker::parse_function_path("MyType::new");
        assert_eq!(result, Some(("MyType".to_string(), "new".to_string())));

        // Test nested module path
        let result = TypeTracker::parse_function_path("std::collections::HashMap::new");
        assert_eq!(
            result,
            Some(("std::collections::HashMap".to_string(), "new".to_string()))
        );

        // Test single name (no ::)
        let result = TypeTracker::parse_function_path("simple_function");
        assert_eq!(result, None);

        // Test multiple levels
        let result = TypeTracker::parse_function_path("crate::module::Type::method");
        assert_eq!(
            result,
            Some(("crate::module::Type".to_string(), "method".to_string()))
        );
    }

    #[test]
    fn test_resolve_type_name() {
        let tracker = TypeTracker::new();

        // Test non-self type
        let return_info = ReturnTypeInfo {
            type_name: "String".to_string(),
            is_self: false,
            is_result: false,
            is_option: false,
            generic_args: vec![],
        };
        assert_eq!(tracker.resolve_type_name(&return_info), "String");

        // Test self type without impl context
        let return_info = ReturnTypeInfo {
            type_name: "MyType".to_string(),
            is_self: true,
            is_result: false,
            is_option: false,
            generic_args: vec![],
        };
        assert_eq!(tracker.resolve_type_name(&return_info), "MyType");
    }

    #[test]
    fn test_resolve_type_name_with_impl_context() {
        let mut tracker = TypeTracker::new();

        // Manually add an impl scope
        tracker.scopes.push(Scope {
            variables: HashMap::new(),
            kind: ScopeKind::Impl,
            impl_type: Some("ActualType".to_string()),
        });

        // Test self type with impl context
        let return_info = ReturnTypeInfo {
            type_name: "FallbackType".to_string(),
            is_self: true,
            is_result: false,
            is_option: false,
            generic_args: vec![],
        };
        assert_eq!(tracker.resolve_type_name(&return_info), "ActualType");
    }

    #[test]
    fn test_create_resolved_type() {
        let resolved = TypeTracker::create_resolved_type(
            "HashMap".to_string(),
            TypeSource::FunctionReturn,
            vec!["String".to_string(), "i32".to_string()],
        );

        assert_eq!(resolved.type_name, "HashMap");
        assert_eq!(resolved.source, TypeSource::FunctionReturn);
        assert_eq!(resolved.generics, vec!["String", "i32"]);
    }

    #[test]
    fn test_resolve_function_call_type_simple_function() {
        let mut tracker = TypeTracker::new();
        let mut registry = FunctionSignatureRegistry::new();

        // Register a simple function
        registry.register_function(FunctionSignature {
            name: "get_string".to_string(),
            module_path: vec![],
            return_type: ReturnTypeInfo {
                type_name: "String".to_string(),
                is_self: false,
                is_result: false,
                is_option: false,
                generic_args: vec![],
            },
            is_async: false,
            visibility: VisibilityInfo::Public,
            generic_params: vec![],
        });

        tracker.function_registry = Some(Arc::new(registry));

        // Parse a function call
        let code = "get_string()";
        let expr: Expr = syn::parse_str(code).unwrap();
        if let Expr::Call(call) = expr {
            let result = tracker.resolve_function_call_type(&call);
            assert!(result.is_some());
            let resolved = result.unwrap();
            assert_eq!(resolved.type_name, "String");
            assert_eq!(resolved.source, TypeSource::FunctionReturn);
        }
    }

    #[test]
    fn test_resolve_function_call_type_constructor() {
        let mut tracker = TypeTracker::new();
        let mut registry = FunctionSignatureRegistry::new();

        // Register a constructor method
        registry.register_method(
            "MyType".to_string(),
            MethodSignature {
                name: "new".to_string(),
                return_type: ReturnTypeInfo {
                    type_name: "MyType".to_string(),
                    is_self: true,
                    is_result: false,
                    is_option: false,
                    generic_args: vec![],
                },
                is_async: false,
                takes_self: false,
                takes_mut_self: false,
                visibility: VisibilityInfo::Public,
                param_types: vec![],
                generic_params: vec![],
            },
        );

        tracker.function_registry = Some(Arc::new(registry));

        // Parse a constructor call
        let code = "MyType::new()";
        let expr: Expr = syn::parse_str(code).unwrap();
        if let Expr::Call(call) = expr {
            let result = tracker.resolve_function_call_type(&call);
            assert!(result.is_some());
            let resolved = result.unwrap();
            assert_eq!(resolved.type_name, "MyType");
            assert_eq!(resolved.source, TypeSource::Constructor);
        }
    }

    #[test]
    fn test_resolve_function_call_type_default_constructor() {
        let mut tracker = TypeTracker::new();
        let registry = FunctionSignatureRegistry::new();

        // Don't register anything - test fallback behavior
        tracker.function_registry = Some(Arc::new(registry));

        // Parse a default constructor call
        let code = "HashMap::default()";
        let expr: Expr = syn::parse_str(code).unwrap();
        if let Expr::Call(call) = expr {
            let result = tracker.resolve_function_call_type(&call);
            assert!(result.is_some());
            let resolved = result.unwrap();
            assert_eq!(resolved.type_name, "HashMap");
            assert_eq!(resolved.source, TypeSource::Constructor);
        }
    }

    #[test]
    fn test_resolve_function_call_type_no_registry() {
        let tracker = TypeTracker::new();

        // No registry set
        let code = "some_function()";
        let expr: Expr = syn::parse_str(code).unwrap();
        if let Expr::Call(call) = expr {
            let result = tracker.resolve_function_call_type(&call);
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_resolve_function_call_type_unknown_function() {
        let mut tracker = TypeTracker::new();
        let registry = FunctionSignatureRegistry::new();
        tracker.function_registry = Some(Arc::new(registry));

        // Call to unregistered function
        let code = "unknown_function()";
        let expr: Expr = syn::parse_str(code).unwrap();
        if let Expr::Call(call) = expr {
            let result = tracker.resolve_function_call_type(&call);
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_extract_self_param_with_reference() {
        // Test &self
        let code = "fn method(&self) {}";
        let item_fn: syn::ItemFn = syn::parse_str(code).unwrap();
        let result = extract_self_param(&item_fn.sig);
        assert!(result.is_some());
        let self_param = result.unwrap();
        assert!(self_param.is_reference);
        assert!(!self_param.is_mutable);
    }

    #[test]
    fn test_extract_self_param_with_mutable_reference() {
        // Test &mut self
        let code = "fn method(&mut self) {}";
        let item_fn: syn::ItemFn = syn::parse_str(code).unwrap();
        let result = extract_self_param(&item_fn.sig);
        assert!(result.is_some());
        let self_param = result.unwrap();
        assert!(self_param.is_reference);
        assert!(self_param.is_mutable);
    }

    #[test]
    fn test_extract_self_param_owned() {
        // Test self (owned)
        let code = "fn method(self) {}";
        let item_fn: syn::ItemFn = syn::parse_str(code).unwrap();
        let result = extract_self_param(&item_fn.sig);
        assert!(result.is_some());
        let self_param = result.unwrap();
        assert!(!self_param.is_reference);
        assert!(!self_param.is_mutable);
    }

    #[test]
    fn test_extract_self_param_no_self() {
        // Test function without self
        let code = "fn function(x: i32) {}";
        let item_fn: syn::ItemFn = syn::parse_str(code).unwrap();
        let result = extract_self_param(&item_fn.sig);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_self_param_static_method() {
        // Test static method (no self, but part of impl block)
        let code = "fn new() -> Self {}";
        let item_fn: syn::ItemFn = syn::parse_str(code).unwrap();
        let result = extract_self_param(&item_fn.sig);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_type_from_pattern_with_explicit_type() {
        // Test explicit type annotation
        let code = "let x: HashMap<String, i32> = HashMap::new();";
        let stmt: syn::Stmt = syn::parse_str(code).unwrap();
        if let syn::Stmt::Local(local) = stmt {
            let result = extract_type_from_pattern(
                &local.pat,
                &local.init.as_ref().map(|init| Box::new(*init.expr.clone())),
            );
            assert!(result.is_some());
            let resolved_type = result.unwrap();
            // The function currently only extracts the base type name without generics
            assert_eq!(resolved_type.type_name, "HashMap");
            assert_eq!(resolved_type.source, TypeSource::Annotation);
        }
    }

    #[test]
    fn test_extract_type_from_pattern_with_inference() {
        // Test type inference from initializer
        let code = "let graph = DependencyGraph::new();";
        let stmt: syn::Stmt = syn::parse_str(code).unwrap();
        if let syn::Stmt::Local(local) = stmt {
            let result = extract_type_from_pattern(
                &local.pat,
                &local.init.as_ref().map(|init| Box::new(*init.expr.clone())),
            );
            assert!(result.is_some());
            let resolved_type = result.unwrap();
            assert_eq!(resolved_type.type_name, "DependencyGraph");
            assert_eq!(resolved_type.source, TypeSource::Constructor);
        }
    }

    #[test]
    fn test_extract_type_from_pattern_no_type_info() {
        // Test pattern without type information
        let code = "let x;";
        let stmt: syn::Stmt = syn::parse_str(code).unwrap();
        if let syn::Stmt::Local(local) = stmt {
            let result = extract_type_from_pattern(
                &local.pat,
                &local.init.as_ref().map(|init| Box::new(*init.expr.clone())),
            );
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_extract_type_from_type_simple() {
        // Test simple type
        let ty: syn::Type = syn::parse_str("String").unwrap();
        let result = extract_type_from_type(&ty);
        assert_eq!(result.type_name, "String");
        assert_eq!(result.source, TypeSource::Annotation);
    }

    #[test]
    fn test_extract_type_from_type_path() {
        // Test path type
        let ty: syn::Type = syn::parse_str("std::collections::HashMap").unwrap();
        let result = extract_type_from_type(&ty);
        assert_eq!(result.type_name, "std::collections::HashMap");
        assert_eq!(result.source, TypeSource::Annotation);
    }

    #[test]
    fn test_extract_type_from_type_generic() {
        // Test generic type
        let ty: syn::Type = syn::parse_str("Vec<String>").unwrap();
        let result = extract_type_from_type(&ty);
        // The function currently only extracts the base type name without generics
        assert_eq!(result.type_name, "Vec");
        assert_eq!(result.source, TypeSource::Annotation);
    }

    #[test]
    fn test_extract_type_from_type_reference() {
        // Test reference type
        let ty: syn::Type = syn::parse_str("&str").unwrap();
        let result = extract_type_from_type(&ty);
        // References are not path types, so they return "Unknown"
        assert_eq!(result.type_name, "Unknown");
        assert_eq!(result.source, TypeSource::Annotation);
    }

    #[test]
    fn test_extract_type_from_expr_constructor() {
        // Test constructor call
        let expr: syn::Expr = syn::parse_str("HashMap::new()").unwrap();
        let result = extract_type_from_expr(&expr);
        assert!(result.is_some());
        let resolved_type = result.unwrap();
        assert_eq!(resolved_type.type_name, "HashMap");
        assert_eq!(resolved_type.source, TypeSource::Constructor);
    }

    #[test]
    fn test_extract_type_from_expr_nested_constructor() {
        // Test nested module constructor
        let expr: syn::Expr = syn::parse_str("std::collections::HashMap::new()").unwrap();
        let result = extract_type_from_expr(&expr);
        assert!(result.is_some());
        let resolved_type = result.unwrap();
        assert_eq!(resolved_type.type_name, "std::collections::HashMap");
        assert_eq!(resolved_type.source, TypeSource::Constructor);
    }

    #[test]
    fn test_extract_type_from_expr_struct_literal() {
        // Test struct literal
        let expr: syn::Expr = syn::parse_str("Point { x: 0, y: 0 }").unwrap();
        let result = extract_type_from_expr(&expr);
        assert!(result.is_some());
        let resolved_type = result.unwrap();
        assert_eq!(resolved_type.type_name, "Point");
        assert_eq!(resolved_type.source, TypeSource::StructLiteral);
    }

    #[test]
    fn test_extract_type_from_expr_module_struct_literal() {
        // Test module path struct literal
        let expr: syn::Expr = syn::parse_str("geometry::Point { x: 0, y: 0 }").unwrap();
        let result = extract_type_from_expr(&expr);
        assert!(result.is_some());
        let resolved_type = result.unwrap();
        assert_eq!(resolved_type.type_name, "geometry::Point");
        assert_eq!(resolved_type.source, TypeSource::StructLiteral);
    }

    #[test]
    fn test_extract_type_from_expr_non_constructor() {
        // Test non-constructor function call
        let expr: syn::Expr = syn::parse_str("process_data()").unwrap();
        let result = extract_type_from_expr(&expr);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_type_from_expr_literal() {
        // Test literal expression
        let expr: syn::Expr = syn::parse_str("42").unwrap();
        let result = extract_type_from_expr(&expr);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_type_from_expr_binary() {
        // Test binary expression
        let expr: syn::Expr = syn::parse_str("x + y").unwrap();
        let result = extract_type_from_expr(&expr);
        assert!(result.is_none());
    }
}
