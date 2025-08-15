use std::collections::HashMap;
use std::path::PathBuf;
use syn::{Field, Fields, Item, ItemStruct, Type, TypePath};

/// Global type registry for tracking struct definitions across the codebase
#[derive(Debug, Clone)]
pub struct GlobalTypeRegistry {
    /// Map from fully-qualified type name to type definition
    pub types: HashMap<String, TypeDefinition>,
    /// Map from module path to exported types
    pub module_exports: HashMap<Vec<String>, Vec<String>>,
    /// Type alias mappings
    pub type_aliases: HashMap<String, String>,
    /// Import mappings for each file
    pub imports: HashMap<PathBuf, ImportScope>,
}

/// Definition of a type (struct, enum, etc.)
#[derive(Debug, Clone)]
pub struct TypeDefinition {
    pub name: String,
    pub kind: TypeKind,
    pub fields: Option<FieldRegistry>,
    pub methods: Vec<MethodSignature>,
    pub generic_params: Vec<String>,
    pub module_path: Vec<String>,
}

/// Registry of fields for a struct
#[derive(Debug, Clone)]
pub struct FieldRegistry {
    /// Named fields for structs
    pub named_fields: HashMap<String, ResolvedFieldType>,
    /// Positional fields for tuple structs
    pub tuple_fields: Vec<ResolvedFieldType>,
}

/// A resolved field type
#[derive(Debug, Clone)]
pub struct ResolvedFieldType {
    pub type_name: String,
    pub is_reference: bool,
    pub is_mutable: bool,
    pub generic_args: Vec<String>,
}

/// Method signature information
#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub self_param: Option<SelfParam>,
    pub return_type: Option<String>,
}

/// Self parameter information
#[derive(Debug, Clone)]
pub struct SelfParam {
    pub is_reference: bool,
    pub is_mutable: bool,
}

/// Kind of type definition
#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Struct,
    Enum,
    Trait,
    TypeAlias,
    TupleStruct,
    UnitStruct,
}

/// Import scope for a file
#[derive(Debug, Clone)]
pub struct ImportScope {
    /// Direct imports (use statements)
    pub imports: HashMap<String, String>, // local name -> fully qualified name
    /// Module imports (use module::*)
    pub wildcard_imports: Vec<Vec<String>>,
}

impl Default for GlobalTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalTypeRegistry {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            module_exports: HashMap::new(),
            type_aliases: HashMap::new(),
            imports: HashMap::new(),
        }
    }

    /// Register a struct definition
    pub fn register_struct(&mut self, module_path: Vec<String>, item: &ItemStruct) {
        let name = item.ident.to_string();
        let full_name = if module_path.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", module_path.join("::"), name)
        };

        let fields = self.extract_fields(&item.fields);
        let generic_params = item
            .generics
            .params
            .iter()
            .filter_map(|param| match param {
                syn::GenericParam::Type(type_param) => Some(type_param.ident.to_string()),
                _ => None,
            })
            .collect();

        let kind = match &item.fields {
            Fields::Named(_) => TypeKind::Struct,
            Fields::Unnamed(_) => TypeKind::TupleStruct,
            Fields::Unit => TypeKind::UnitStruct,
        };

        let type_def = TypeDefinition {
            name: full_name.clone(),
            kind,
            fields: Some(fields),
            methods: Vec::new(),
            generic_params,
            module_path: module_path.clone(),
        };

        self.types.insert(full_name, type_def);

        // Update module exports
        self.module_exports
            .entry(module_path)
            .or_default()
            .push(name);
    }

    /// Extract fields from a struct
    fn extract_fields(&self, fields: &Fields) -> FieldRegistry {
        match fields {
            Fields::Named(named_fields) => {
                let mut named = HashMap::new();
                for field in &named_fields.named {
                    if let Some(ident) = &field.ident {
                        let field_type = self.extract_field_type(field);
                        named.insert(ident.to_string(), field_type);
                    }
                }
                FieldRegistry {
                    named_fields: named,
                    tuple_fields: Vec::new(),
                }
            }
            Fields::Unnamed(unnamed_fields) => {
                let tuple_fields = unnamed_fields
                    .unnamed
                    .iter()
                    .map(|field| self.extract_field_type(field))
                    .collect();
                FieldRegistry {
                    named_fields: HashMap::new(),
                    tuple_fields,
                }
            }
            Fields::Unit => FieldRegistry {
                named_fields: HashMap::new(),
                tuple_fields: Vec::new(),
            },
        }
    }

    /// Extract type information from a field
    fn extract_field_type(&self, field: &Field) -> ResolvedFieldType {
        match &field.ty {
            Type::Path(TypePath { path, .. }) => {
                let type_name = path
                    .segments
                    .iter()
                    .map(|seg| seg.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");

                let generic_args = if let Some(last_seg) = path.segments.last() {
                    match &last_seg.arguments {
                        syn::PathArguments::AngleBracketed(args) => args
                            .args
                            .iter()
                            .filter_map(|arg| match arg {
                                syn::GenericArgument::Type(Type::Path(type_path)) => {
                                    Some(type_path.path.segments.last()?.ident.to_string())
                                }
                                _ => None,
                            })
                            .collect(),
                        _ => Vec::new(),
                    }
                } else {
                    Vec::new()
                };

                ResolvedFieldType {
                    type_name,
                    is_reference: false,
                    is_mutable: false,
                    generic_args,
                }
            }
            Type::Reference(type_ref) => {
                let mut field_type = match &*type_ref.elem {
                    Type::Path(type_path) => {
                        let type_name = type_path
                            .path
                            .segments
                            .iter()
                            .map(|seg| seg.ident.to_string())
                            .collect::<Vec<_>>()
                            .join("::");
                        ResolvedFieldType {
                            type_name,
                            is_reference: true,
                            is_mutable: type_ref.mutability.is_some(),
                            generic_args: Vec::new(),
                        }
                    }
                    _ => ResolvedFieldType {
                        type_name: "Unknown".to_string(),
                        is_reference: true,
                        is_mutable: type_ref.mutability.is_some(),
                        generic_args: Vec::new(),
                    },
                };
                field_type.is_reference = true;
                field_type.is_mutable = type_ref.mutability.is_some();
                field_type
            }
            _ => ResolvedFieldType {
                type_name: "Unknown".to_string(),
                is_reference: false,
                is_mutable: false,
                generic_args: Vec::new(),
            },
        }
    }

    /// Get a type definition by name
    pub fn get_type(&self, name: &str) -> Option<&TypeDefinition> {
        self.types.get(name)
    }

    /// Resolve a field on a type
    pub fn resolve_field(&self, type_name: &str, field_name: &str) -> Option<ResolvedFieldType> {
        let type_def = self.get_type(type_name)?;
        let fields = type_def.fields.as_ref()?;
        fields.named_fields.get(field_name).cloned()
    }

    /// Get field by index for tuple structs
    pub fn resolve_tuple_field(&self, type_name: &str, index: usize) -> Option<ResolvedFieldType> {
        let type_def = self.get_type(type_name)?;
        let fields = type_def.fields.as_ref()?;
        fields.tuple_fields.get(index).cloned()
    }

    /// Add a method to a type
    pub fn add_method(&mut self, type_name: &str, method: MethodSignature) {
        if let Some(type_def) = self.types.get_mut(type_name) {
            type_def.methods.push(method);
        }
    }

    /// Register a type alias
    pub fn register_type_alias(&mut self, alias: String, target: String) {
        self.type_aliases.insert(alias, target);
    }

    /// Resolve a type alias
    pub fn resolve_type_alias(&self, alias: &str) -> Option<&String> {
        self.type_aliases.get(alias)
    }

    /// Register imports for a file
    pub fn register_imports(&mut self, file: PathBuf, imports: ImportScope) {
        self.imports.insert(file, imports);
    }

    /// Get imports for a file
    pub fn get_imports(&self, file: &PathBuf) -> Option<&ImportScope> {
        self.imports.get(file)
    }

    /// Resolve a type name using imports
    pub fn resolve_type_with_imports(&self, file: &PathBuf, name: &str) -> Option<String> {
        // First check if it's already fully qualified
        if self.types.contains_key(name) {
            return Some(name.to_string());
        }

        // Check imports for this file
        if let Some(import_scope) = self.get_imports(file) {
            // Check direct imports
            if let Some(full_name) = import_scope.imports.get(name) {
                return Some(full_name.clone());
            }

            // Check wildcard imports
            for module_path in &import_scope.wildcard_imports {
                let potential_name = format!("{}::{}", module_path.join("::"), name);
                if self.types.contains_key(&potential_name) {
                    return Some(potential_name);
                }
            }
        }

        // Check if it's a type alias
        if let Some(target) = self.resolve_type_alias(name) {
            return Some(target.clone());
        }

        None
    }
}

/// Extract type definitions from a parsed file
pub fn extract_type_definitions(
    file: &syn::File,
    module_path: Vec<String>,
    registry: &mut GlobalTypeRegistry,
) {
    for item in &file.items {
        match item {
            Item::Struct(item_struct) => {
                registry.register_struct(module_path.clone(), item_struct);
            }
            Item::Mod(item_mod) => {
                if let Some((_, items)) = &item_mod.content {
                    let mut nested_path = module_path.clone();
                    nested_path.push(item_mod.ident.to_string());
                    for item in items {
                        if let Item::Struct(item_struct) = item {
                            registry.register_struct(nested_path.clone(), item_struct);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
