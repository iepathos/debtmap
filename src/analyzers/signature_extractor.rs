use crate::analyzers::function_registry::{
    BuilderInfo, FunctionSignature, FunctionSignatureRegistry, MethodSignature, ReturnTypeInfo,
    VisibilityInfo,
};
use syn::visit::Visit;
use syn::{File, FnArg, ImplItem, ImplItemFn, Item, ItemFn, ItemImpl, Type};

/// Extracts function signatures from a Rust AST
pub struct SignatureExtractor {
    pub registry: FunctionSignatureRegistry,
    current_module_path: Vec<String>,
    current_impl_type: Option<String>,
}

impl Default for SignatureExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl SignatureExtractor {
    pub fn new() -> Self {
        Self {
            registry: FunctionSignatureRegistry::new(),
            current_module_path: Vec::new(),
            current_impl_type: None,
        }
    }

    /// Extract signatures from a parsed file
    pub fn extract_from_file(&mut self, file: &File) {
        self.visit_file(file);
        self.detect_builder_patterns();
    }

    /// Extract function signature from ItemFn
    fn extract_function_signature(&self, item_fn: &ItemFn) -> FunctionSignature {
        let name = item_fn.sig.ident.to_string();
        let return_type = ReturnTypeInfo::from_syn_return(&item_fn.sig.output);
        let generic_params = item_fn
            .sig
            .generics
            .params
            .iter()
            .filter_map(|param| {
                if let syn::GenericParam::Type(type_param) = param {
                    Some(type_param.ident.to_string())
                } else {
                    None
                }
            })
            .collect();
        let is_async = item_fn.sig.asyncness.is_some();
        let visibility = VisibilityInfo::from(&item_fn.vis);

        FunctionSignature {
            name,
            return_type,
            generic_params,
            is_async,
            visibility,
            module_path: self.current_module_path.clone(),
        }
    }

    /// Extract method signature from ImplItemFn
    fn extract_method_signature(&self, impl_fn: &ImplItemFn) -> MethodSignature {
        let name = impl_fn.sig.ident.to_string();
        let mut return_type = ReturnTypeInfo::from_syn_return(&impl_fn.sig.output);

        // Handle methods that return Self
        if return_type.is_self && self.current_impl_type.is_some() {
            return_type.type_name = self.current_impl_type.as_ref().unwrap().clone();
            return_type.is_self = false;
        }

        let generic_params = impl_fn
            .sig
            .generics
            .params
            .iter()
            .filter_map(|param| {
                if let syn::GenericParam::Type(type_param) = param {
                    Some(type_param.ident.to_string())
                } else {
                    None
                }
            })
            .collect();

        let is_async = impl_fn.sig.asyncness.is_some();
        let visibility = VisibilityInfo::from(&impl_fn.vis);

        let (takes_self, takes_mut_self) = self.analyze_self_parameter(&impl_fn.sig.inputs);
        let param_types = self.extract_parameter_types(&impl_fn.sig.inputs);

        MethodSignature {
            name,
            return_type,
            generic_params,
            is_async,
            takes_self,
            takes_mut_self,
            visibility,
            param_types,
        }
    }

    /// Analyze the self parameter of a method
    fn analyze_self_parameter(
        &self,
        inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
    ) -> (bool, bool) {
        if let Some(FnArg::Receiver(receiver)) = inputs.first() {
            let takes_self = true;
            let takes_mut_self = receiver.mutability.is_some();
            (takes_self, takes_mut_self)
        } else {
            (false, false)
        }
    }

    /// Extract parameter types from function inputs
    fn extract_parameter_types(
        &self,
        inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
    ) -> Vec<String> {
        inputs
            .iter()
            .filter_map(|arg| match arg {
                FnArg::Receiver(_) => None, // Skip self parameter
                FnArg::Typed(pat_type) => Self::get_type_name(&pat_type.ty),
            })
            .collect()
    }

    /// Detect builder patterns in registered methods
    fn detect_builder_patterns(&mut self) {
        let mut builders = Vec::new();

        // Find types that have methods returning Self and a build() method
        for (type_name, methods) in self.registry.get_all_methods() {
            let chain_methods: Vec<String> = methods
                .iter()
                .filter(|m| {
                    m.return_type.type_name == *type_name || m.return_type.type_name == "Self"
                })
                .map(|m| m.name.clone())
                .collect();

            let build_method = methods
                .iter()
                .find(|m| m.name == "build" || m.name == "finish" || m.name == "complete");

            if !chain_methods.is_empty() {
                if let Some(build_method) = build_method {
                    let target_type = build_method.return_type.type_name.clone();

                    if target_type != *type_name && target_type != "Self" {
                        builders.push(BuilderInfo {
                            builder_type: type_name.clone(),
                            target_type,
                            build_method: build_method.name.clone(),
                            chain_methods,
                        });
                    }
                }
            }
        }

        // Register detected builders
        for builder in builders {
            self.registry.register_builder(builder);
        }
    }

    /// Get the type name from a Type
    fn get_type_name(ty: &Type) -> Option<String> {
        match ty {
            Type::Path(type_path) => {
                let segments = &type_path.path.segments;
                if segments.is_empty() {
                    None
                } else {
                    Some(
                        segments
                            .iter()
                            .map(|seg| seg.ident.to_string())
                            .collect::<Vec<_>>()
                            .join("::"),
                    )
                }
            }
            _ => None,
        }
    }
}

impl<'ast> Visit<'ast> for SignatureExtractor {
    fn visit_item(&mut self, item: &'ast Item) {
        match item {
            Item::Fn(item_fn) => {
                let signature = self.extract_function_signature(item_fn);
                self.registry.register_function(signature);
                syn::visit::visit_item_fn(self, item_fn);
            }
            Item::Impl(item_impl) => {
                self.visit_item_impl(item_impl);
            }
            Item::Mod(item_mod) => {
                if let Some((_, items)) = &item_mod.content {
                    let mod_name = item_mod.ident.to_string();
                    self.current_module_path.push(mod_name);
                    for item in items {
                        self.visit_item(item);
                    }
                    self.current_module_path.pop();
                }
            }
            _ => {
                syn::visit::visit_item(self, item);
            }
        }
    }

    fn visit_item_impl(&mut self, item_impl: &'ast ItemImpl) {
        // Extract the type being implemented
        let impl_type = if let Type::Path(type_path) = &*item_impl.self_ty {
            Self::get_type_name(&Type::Path(type_path.clone()))
        } else {
            None
        };

        if let Some(type_name) = impl_type {
            self.current_impl_type = Some(type_name.clone());

            // Process all methods in the impl block
            for item in &item_impl.items {
                if let ImplItem::Fn(impl_fn) = item {
                    let method = self.extract_method_signature(impl_fn);
                    self.registry.register_method(type_name.clone(), method);
                }
            }

            self.current_impl_type = None;
        }

        syn::visit::visit_item_impl(self, item_impl);
    }
}
