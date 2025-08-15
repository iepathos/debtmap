/// Comprehensive trait implementation tracking for dynamic dispatch resolution
///
/// This module extends the trait registry with full support for:
/// - Generic trait implementations
/// - Trait object resolution
/// - Blanket implementations
/// - Associated types and methods
use crate::priority::call_graph::FunctionId;
use im::{HashMap, HashSet, Vector};
use std::path::PathBuf;
use syn::visit::Visit;
use syn::{
    GenericParam, Generics, ImplItem, Item, ItemImpl, ItemTrait, Path as SynPath, TraitItem, Type,
    TypeParam, TypePath, WhereClause, WherePredicate,
};

/// Represents a trait definition with all its details
#[derive(Debug, Clone)]
pub struct TraitDefinition {
    pub name: String,
    pub methods: Vector<TraitMethod>,
    pub associated_types: Vector<AssociatedType>,
    pub supertraits: Vector<String>,
    pub generic_params: Vector<GenericParameter>,
    pub module_path: Vector<String>,
}

/// Represents a trait method
#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub has_default: bool,
    pub is_async: bool,
    pub signature: String,
}

/// Represents an associated type in a trait
#[derive(Debug, Clone)]
pub struct AssociatedType {
    pub name: String,
    pub bounds: Vector<String>,
    pub default: Option<String>,
}

/// Represents a generic parameter on a trait
#[derive(Debug, Clone)]
pub struct GenericParameter {
    pub name: String,
    pub bounds: Vector<String>,
}

/// Represents a trait implementation
#[derive(Debug, Clone)]
pub struct Implementation {
    pub trait_name: String,
    pub implementing_type: String,
    pub methods: HashMap<String, MethodImpl>,
    pub generic_constraints: Vector<WhereClauseItem>,
    pub is_blanket: bool,
    pub is_negative: bool,
    pub module_path: Vector<String>,
}

/// Represents a method implementation
#[derive(Debug, Clone)]
pub struct MethodImpl {
    pub name: String,
    pub function_id: FunctionId,
    pub overrides_default: bool,
}

/// Represents a where clause constraint
#[derive(Debug, Clone)]
pub struct WhereClauseItem {
    pub type_param: String,
    pub bounds: Vector<String>,
}

/// Trait object information
#[derive(Debug, Clone)]
pub struct TraitObject {
    pub trait_name: String,
    pub additional_bounds: Vector<String>,
    pub lifetime: Option<String>,
}

/// The main trait implementation tracker
#[derive(Debug, Clone, Default)]
pub struct TraitImplementationTracker {
    /// All trait definitions indexed by name
    pub traits: HashMap<String, TraitDefinition>,
    /// All implementations indexed by trait name
    pub implementations: HashMap<String, Vector<Implementation>>,
    /// Trait object candidates (types that can be behind trait objects)
    pub trait_objects: HashMap<String, HashSet<String>>,
    /// Generic bounds registry
    pub generic_bounds: HashMap<String, Vector<TraitBound>>,
    /// Type to trait mapping for quick lookup
    pub type_to_traits: HashMap<String, HashSet<String>>,
    /// Blanket implementations
    pub blanket_impls: Vector<Implementation>,
    /// Associated type projections
    pub associated_types: HashMap<(String, String), String>, // (Type, AssocType) -> ResolvedType
}

/// Represents a trait bound
#[derive(Debug, Clone)]
pub struct TraitBound {
    pub trait_name: String,
    pub type_params: Vector<String>,
}

impl TraitImplementationTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a trait definition
    pub fn register_trait(&mut self, trait_def: TraitDefinition) {
        let name = trait_def.name.clone();
        self.traits.insert(name, trait_def);
    }

    /// Register a trait implementation
    pub fn register_implementation(&mut self, implementation: Implementation) {
        let trait_name = implementation.trait_name.clone();
        let implementing_type = implementation.implementing_type.clone();

        // Update type to trait mapping
        self.type_to_traits
            .entry(implementing_type.clone())
            .or_default()
            .insert(trait_name.clone());

        // Track blanket implementations separately
        if implementation.is_blanket {
            self.blanket_impls.push_back(implementation.clone());
        }

        // Add to regular implementations
        self.implementations
            .entry(trait_name.clone())
            .or_default()
            .push_back(implementation.clone());

        // Track trait object candidates
        if !implementation.is_negative {
            self.trait_objects
                .entry(trait_name)
                .or_default()
                .insert(implementing_type);
        }
    }

    /// Get all types that implement a trait
    pub fn get_implementors(&self, trait_name: &str) -> Option<HashSet<String>> {
        self.trait_objects.get(trait_name).cloned()
    }

    /// Resolve a trait object method call to concrete implementations
    pub fn resolve_trait_object_call(
        &self,
        trait_name: &str,
        method_name: &str,
    ) -> Vector<FunctionId> {
        let mut implementations = Vector::new();

        // Find all types that implement this trait
        if let Some(implementors) = self.get_implementors(trait_name) {
            for impl_type in implementors {
                if let Some(method_id) = self.resolve_method(&impl_type, trait_name, method_name) {
                    implementations.push_back(method_id);
                }
            }
        }

        implementations
    }

    /// Resolve a method call on a specific type for a specific trait
    pub fn resolve_method(
        &self,
        type_name: &str,
        trait_name: &str,
        method_name: &str,
    ) -> Option<FunctionId> {
        self.implementations
            .get(trait_name)?
            .iter()
            .find(|impl_info| impl_info.implementing_type == type_name)
            .and_then(|impl_info| impl_info.methods.get(method_name))
            .map(|method| method.function_id.clone())
    }

    /// Resolve generic constraint to possible implementations
    pub fn resolve_generic_bound(
        &self,
        bound: &TraitBound,
        method_name: &str,
    ) -> Vector<FunctionId> {
        let mut implementations = Vector::new();

        // Find all types satisfying the bound
        if let Some(impls) = self.implementations.get(&bound.trait_name) {
            for impl_info in impls {
                // Check if this implementation satisfies the bound
                // This is simplified - real implementation would need constraint checking
                if let Some(method) = impl_info.methods.get(method_name) {
                    implementations.push_back(method.function_id.clone());
                }
            }
        }

        // Check blanket implementations
        for blanket in &self.blanket_impls {
            if blanket.trait_name == bound.trait_name {
                if let Some(method) = blanket.methods.get(method_name) {
                    implementations.push_back(method.function_id.clone());
                }
            }
        }

        implementations
    }

    /// Check if a type implements a trait
    pub fn implements_trait(&self, type_name: &str, trait_name: &str) -> bool {
        self.type_to_traits
            .get(type_name)
            .map(|traits| traits.contains(trait_name))
            .unwrap_or(false)
    }

    /// Get all traits implemented by a type
    pub fn get_traits_for_type(&self, type_name: &str) -> Option<&HashSet<String>> {
        self.type_to_traits.get(type_name)
    }

    /// Resolve associated type projection
    pub fn resolve_associated_type(&self, type_name: &str, assoc_type: &str) -> Option<String> {
        self.associated_types
            .get(&(type_name.to_string(), assoc_type.to_string()))
            .cloned()
    }

    /// Register an associated type projection
    pub fn register_associated_type(
        &mut self,
        type_name: String,
        assoc_type: String,
        resolved_type: String,
    ) {
        self.associated_types
            .insert((type_name, assoc_type), resolved_type);
    }

    /// Check if an implementation is a blanket implementation
    pub fn is_blanket_impl(&self, implementation: &Implementation) -> bool {
        // Check if the implementing type contains generic parameters
        implementation.implementing_type.contains('<')
            || !implementation.generic_constraints.is_empty()
    }

    /// Get trait definition by name
    pub fn get_trait(&self, name: &str) -> Option<&TraitDefinition> {
        self.traits.get(name)
    }

    /// Get all blanket implementations
    pub fn get_blanket_impls(&self) -> &Vector<Implementation> {
        &self.blanket_impls
    }

    /// Check if a method exists in a trait
    pub fn trait_has_method(&self, trait_name: &str, method_name: &str) -> bool {
        self.traits
            .get(trait_name)
            .map(|trait_def| {
                trait_def
                    .methods
                    .iter()
                    .any(|method| method.name == method_name)
            })
            .unwrap_or(false)
    }
}

/// AST visitor for extracting trait definitions and implementations
pub struct TraitExtractor {
    file_path: PathBuf,
    module_path: Vec<String>,
    tracker: TraitImplementationTracker,
}

impl TraitExtractor {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            module_path: Vec::new(),
            tracker: TraitImplementationTracker::new(),
        }
    }

    /// Extract trait information from a file
    pub fn extract(&mut self, file: &syn::File) -> TraitImplementationTracker {
        self.visit_file(file);
        self.tracker.clone()
    }

    fn extract_trait_definition(&self, item_trait: &ItemTrait) -> TraitDefinition {
        let mut methods = Vector::new();
        let mut associated_types = Vector::new();

        for trait_item in &item_trait.items {
            match trait_item {
                TraitItem::Fn(method) => {
                    methods.push_back(TraitMethod {
                        name: method.sig.ident.to_string(),
                        has_default: method.default.is_some(),
                        is_async: method.sig.asyncness.is_some(),
                        signature: format!("{}", quote::quote! { #method }),
                    });
                }
                TraitItem::Type(assoc_type) => {
                    let bounds = assoc_type
                        .bounds
                        .iter()
                        .map(|b| format!("{}", quote::quote! { #b }))
                        .collect();
                    let default = assoc_type
                        .default
                        .as_ref()
                        .map(|(_, ty)| format!("{}", quote::quote! { #ty }));

                    associated_types.push_back(AssociatedType {
                        name: assoc_type.ident.to_string(),
                        bounds,
                        default,
                    });
                }
                _ => {}
            }
        }

        let generic_params = self.extract_generic_params(&item_trait.generics);
        let supertraits = self.extract_supertraits(&item_trait.supertraits);

        TraitDefinition {
            name: item_trait.ident.to_string(),
            methods,
            associated_types,
            supertraits,
            generic_params,
            module_path: self.module_path.clone().into(),
        }
    }

    fn extract_generic_params(&self, generics: &Generics) -> Vector<GenericParameter> {
        generics
            .params
            .iter()
            .filter_map(|param| match param {
                GenericParam::Type(type_param) => Some(self.extract_type_param(type_param)),
                _ => None,
            })
            .collect()
    }

    fn extract_type_param(&self, type_param: &TypeParam) -> GenericParameter {
        let bounds = type_param
            .bounds
            .iter()
            .map(|b| format!("{}", quote::quote! { #b }))
            .collect();

        GenericParameter {
            name: type_param.ident.to_string(),
            bounds,
        }
    }

    fn extract_supertraits(&self, supertraits: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::token::Plus>) -> Vector<String> {
        supertraits
            .iter()
            .filter_map(|bound| match bound {
                syn::TypeParamBound::Trait(trait_bound) => {
                    Some(self.path_to_string(&trait_bound.path))
                }
                _ => None,
            })
            .collect()
    }

    fn extract_implementation(&mut self, item_impl: &ItemImpl) -> Option<Implementation> {
        let (_, trait_path, _) = item_impl.trait_.as_ref()?;
        let trait_name = self.path_to_string(trait_path);
        let implementing_type = self.type_to_string(&item_impl.self_ty);

        let mut methods = HashMap::new();
        for impl_item in &item_impl.items {
            if let ImplItem::Fn(method) = impl_item {
                let method_name = method.sig.ident.to_string();
                let line = method.sig.ident.span().start().line;

                let method_impl = MethodImpl {
                    name: method_name.clone(),
                    function_id: FunctionId {
                        file: self.file_path.clone(),
                        name: format!("{}::{}", implementing_type, method_name),
                        line,
                    },
                    overrides_default: false, // Would need trait definition to determine
                };

                methods.insert(method_name, method_impl);
            }
        }

        let generic_constraints =
            self.extract_where_clause(item_impl.generics.where_clause.as_ref());
        let is_blanket = self.is_blanket_implementation(item_impl);
        let is_negative = false; // Negative implementations are not directly supported in stable Rust

        Some(Implementation {
            trait_name,
            implementing_type,
            methods,
            generic_constraints,
            is_blanket,
            is_negative,
            module_path: self.module_path.clone().into(),
        })
    }

    fn extract_where_clause(&self, where_clause: Option<&WhereClause>) -> Vector<WhereClauseItem> {
        where_clause
            .map(|wc| {
                wc.predicates
                    .iter()
                    .filter_map(|pred| match pred {
                        WherePredicate::Type(type_pred) => {
                            let type_param = self.type_to_string(&type_pred.bounded_ty);
                            let bounds = type_pred
                                .bounds
                                .iter()
                                .map(|b| format!("{}", quote::quote! { #b }))
                                .collect();
                            Some(WhereClauseItem { type_param, bounds })
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn is_blanket_implementation(&self, item_impl: &ItemImpl) -> bool {
        // Check if implementing type is generic
        matches!(&*item_impl.self_ty, Type::Path(TypePath { path, .. }) if path.segments.iter().any(|seg| !seg.arguments.is_empty()))
            || !item_impl.generics.params.is_empty()
    }

    fn type_to_string(&self, ty: &Type) -> String {
        format!("{}", quote::quote! { #ty })
            .replace(" ", "")
            .replace(",", ", ")
    }

    fn path_to_string(&self, path: &SynPath) -> String {
        path.segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    }
}

impl<'ast> Visit<'ast> for TraitExtractor {
    fn visit_item(&mut self, item: &'ast Item) {
        match item {
            Item::Trait(item_trait) => {
                let trait_def = self.extract_trait_definition(item_trait);
                self.tracker.register_trait(trait_def);
            }
            Item::Impl(item_impl) => {
                if let Some(implementation) = self.extract_implementation(item_impl) {
                    self.tracker.register_implementation(implementation);
                }
            }
            Item::Mod(item_mod) => {
                self.module_path.push(item_mod.ident.to_string());
            }
            _ => {}
        }

        syn::visit::visit_item(self, item);

        // Pop module path after visiting
        if matches!(item, Item::Mod(_)) {
            self.module_path.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trait_implementation_tracker_new() {
        let tracker = TraitImplementationTracker::new();
        assert!(tracker.traits.is_empty());
        assert!(tracker.implementations.is_empty());
    }

    #[test]
    fn test_register_trait() {
        let mut tracker = TraitImplementationTracker::new();
        let trait_def = TraitDefinition {
            name: "TestTrait".to_string(),
            methods: Vector::new(),
            associated_types: Vector::new(),
            supertraits: Vector::new(),
            generic_params: Vector::new(),
            module_path: Vector::new(),
        };

        tracker.register_trait(trait_def);
        assert!(tracker.get_trait("TestTrait").is_some());
    }

    #[test]
    fn test_implements_trait() {
        let mut tracker = TraitImplementationTracker::new();
        let implementation = Implementation {
            trait_name: "Display".to_string(),
            implementing_type: "MyType".to_string(),
            methods: HashMap::new(),
            generic_constraints: Vector::new(),
            is_blanket: false,
            is_negative: false,
            module_path: Vector::new(),
        };

        tracker.register_implementation(implementation);
        assert!(tracker.implements_trait("MyType", "Display"));
        assert!(!tracker.implements_trait("MyType", "Debug"));
    }
}
