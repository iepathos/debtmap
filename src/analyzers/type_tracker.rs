use crate::analyzers::function_registry::FunctionSignatureRegistry;
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

#[derive(Debug, Clone)]
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
                    self.resolve_variable_type(&ident)
                } else {
                    // It's a path to a type or function
                    None
                }
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

    /// Resolve function call return type
    fn resolve_function_call_type(&self, call_expr: &ExprCall) -> Option<ResolvedType> {
        if let Some(registry) = &self.function_registry {
            // Extract function path
            if let Expr::Path(path_expr) = &*call_expr.func {
                let func_path = path_expr
                    .path
                    .segments
                    .iter()
                    .map(|seg| seg.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");

                // Try to resolve the function
                if let Some(return_info) = registry.resolve_function_return(&func_path, &[]) {
                    // Handle special cases like Result and Option
                    let type_name = if return_info.is_self {
                        // If return type is Self, we need context
                        self.current_impl_type()
                            .unwrap_or_else(|| return_info.type_name.clone())
                    } else {
                        return_info.type_name.clone()
                    };

                    return Some(ResolvedType {
                        type_name,
                        source: TypeSource::FunctionReturn,
                        generics: return_info.generic_args.clone(),
                    });
                }

                // Check if it's a constructor pattern (Type::new, Type::default, etc.)
                if func_path.contains("::") {
                    let parts: Vec<&str> = func_path.split("::").collect();
                    if parts.len() >= 2 {
                        let type_name = parts[..parts.len() - 1].join("::");
                        let method_name = parts.last().unwrap();

                        // Common constructor patterns
                        if matches!(
                            *method_name,
                            "new" | "default" | "from" | "create" | "builder"
                        ) {
                            // Check if this is a known method
                            if let Some(return_info) =
                                registry.resolve_method_return(&type_name, method_name)
                            {
                                let resolved_type_name = if return_info.is_self {
                                    type_name.clone()
                                } else {
                                    return_info.type_name.clone()
                                };

                                return Some(ResolvedType {
                                    type_name: resolved_type_name,
                                    source: TypeSource::Constructor,
                                    generics: return_info.generic_args.clone(),
                                });
                            } else if matches!(*method_name, "new" | "default") {
                                // Assume it returns the type itself for common constructors
                                return Some(ResolvedType {
                                    type_name,
                                    source: TypeSource::Constructor,
                                    generics: Vec::new(),
                                });
                            }
                        }
                    }
                }
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
