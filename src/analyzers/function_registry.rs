use std::collections::HashMap;
use syn::{ReturnType as SynReturnType, Type, Visibility};

/// Registry of function signatures and their return types
#[derive(Debug, Clone, Default)]
pub struct FunctionSignatureRegistry {
    /// Map from fully-qualified function name to signature
    functions: HashMap<String, FunctionSignature>,
    /// Map from type to its methods
    methods: HashMap<String, Vec<MethodSignature>>,
    /// Builder pattern detection
    builders: HashMap<String, BuilderInfo>,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub return_type: ReturnTypeInfo,
    pub generic_params: Vec<String>,
    pub is_async: bool,
    pub visibility: VisibilityInfo,
    pub module_path: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub return_type: ReturnTypeInfo,
    pub generic_params: Vec<String>,
    pub is_async: bool,
    pub takes_self: bool,
    pub takes_mut_self: bool,
    pub visibility: VisibilityInfo,
}

#[derive(Debug, Clone)]
pub struct ReturnTypeInfo {
    pub type_name: String,
    pub is_result: bool,
    pub is_option: bool,
    pub generic_args: Vec<String>,
    pub is_self: bool,
}

#[derive(Debug, Clone)]
pub enum VisibilityInfo {
    Public,
    PublicCrate,
    Private,
}

#[derive(Debug, Clone)]
pub struct BuilderInfo {
    pub builder_type: String,
    pub target_type: String,
    pub build_method: String,
    pub chain_methods: Vec<String>,
}

impl FunctionSignatureRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a function signature
    pub fn register_function(&mut self, signature: FunctionSignature) {
        let key = self.make_function_key(&signature.module_path, &signature.name);
        self.functions.insert(key, signature);
    }

    /// Register a method signature
    pub fn register_method(&mut self, type_name: String, method: MethodSignature) {
        self.methods.entry(type_name).or_default().push(method);
    }

    /// Register builder pattern information
    pub fn register_builder(&mut self, builder: BuilderInfo) {
        self.builders.insert(builder.builder_type.clone(), builder);
    }

    /// Get a function's signature by its fully-qualified name
    pub fn get_function(&self, qualified_name: &str) -> Option<&FunctionSignature> {
        self.functions.get(qualified_name)
    }

    /// Get a method's signature by type and method name
    pub fn get_method(&self, type_name: &str, method_name: &str) -> Option<&MethodSignature> {
        self.methods
            .get(type_name)?
            .iter()
            .find(|m| m.name == method_name)
    }

    /// Check if a type is a builder
    pub fn is_builder(&self, type_name: &str) -> bool {
        self.builders.contains_key(type_name)
    }

    /// Get builder information
    pub fn get_builder(&self, type_name: &str) -> Option<&BuilderInfo> {
        self.builders.get(type_name)
    }

    /// Get all methods for iteration (used for builder detection)
    pub fn get_all_methods(&self) -> impl Iterator<Item = (&String, &Vec<MethodSignature>)> {
        self.methods.iter()
    }

    /// Make a fully-qualified function key
    fn make_function_key(&self, module_path: &[String], name: &str) -> String {
        if module_path.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", module_path.join("::"), name)
        }
    }

    /// Resolve a function call's return type
    pub fn resolve_function_return(
        &self,
        func_path: &str,
        _generic_args: &[String],
    ) -> Option<ReturnTypeInfo> {
        let signature = self.get_function(func_path)?;
        Some(signature.return_type.clone())
    }

    /// Resolve a method call's return type
    pub fn resolve_method_return(
        &self,
        receiver_type: &str,
        method_name: &str,
    ) -> Option<ReturnTypeInfo> {
        let method = self.get_method(receiver_type, method_name)?;
        Some(method.return_type.clone())
    }
}

impl ReturnTypeInfo {
    /// Create return type info from syn ReturnType
    pub fn from_syn_return(return_type: &SynReturnType) -> Self {
        match return_type {
            SynReturnType::Default => Self {
                type_name: "()".to_string(),
                is_result: false,
                is_option: false,
                generic_args: Vec::new(),
                is_self: false,
            },
            SynReturnType::Type(_, ty) => Self::from_type(ty),
        }
    }

    /// Create return type info from a syn Type
    pub fn from_type(ty: &Type) -> Self {
        match ty {
            Type::Path(type_path) => {
                let path = &type_path.path;
                let type_name = path
                    .segments
                    .iter()
                    .map(|seg| seg.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");

                let is_result = type_name == "Result" || type_name.ends_with("::Result");
                let is_option = type_name == "Option" || type_name.ends_with("::Option");
                let is_self = type_name == "Self";

                let generic_args = if let Some(last_seg) = path.segments.last() {
                    if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                        args.args
                            .iter()
                            .filter_map(|arg| {
                                if let syn::GenericArgument::Type(Type::Path(tp)) = arg {
                                    Some(tp.path.segments.last()?.ident.to_string())
                                } else {
                                    None
                                }
                            })
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                Self {
                    type_name,
                    is_result,
                    is_option,
                    generic_args,
                    is_self,
                }
            }
            Type::Reference(type_ref) => Self::from_type(&type_ref.elem),
            _ => Self {
                type_name: "Unknown".to_string(),
                is_result: false,
                is_option: false,
                generic_args: Vec::new(),
                is_self: false,
            },
        }
    }
}

impl From<&Visibility> for VisibilityInfo {
    fn from(vis: &Visibility) -> Self {
        match vis {
            Visibility::Public(_) => VisibilityInfo::Public,
            Visibility::Restricted(res) if res.path.is_ident("crate") => {
                VisibilityInfo::PublicCrate
            }
            _ => VisibilityInfo::Private,
        }
    }
}
