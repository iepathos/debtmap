use std::collections::HashMap;
use syn::{
    Expr, ExprCall, ExprMethodCall, ExprPath, ExprStruct, Pat, PatIdent, PatType, Type, TypePath,
};

/// Tracks variable types within the current analysis scope
#[derive(Debug, Clone)]
pub struct TypeTracker {
    /// Stack of scopes, innermost last
    scopes: Vec<Scope>,
    /// Current module path for resolving imports
    #[allow(dead_code)]
    module_path: Vec<String>,
    /// Type definitions found in the file
    #[allow(dead_code)]
    type_definitions: HashMap<String, TypeInfo>,
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
        }
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
                    self.resolve_variable_type(&ident)
                } else {
                    // It's a path to a type or function
                    None
                }
            }
            Expr::MethodCall(ExprMethodCall { receiver, .. }) => {
                // Recursively resolve the receiver's type
                self.resolve_expr_type(receiver)
            }
            Expr::Field(field_expr) => {
                // For field access, we'd need more info about struct fields
                // For now, just try to resolve the base expression
                self.resolve_expr_type(&field_expr.base)
            }
            _ => None,
        }
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
