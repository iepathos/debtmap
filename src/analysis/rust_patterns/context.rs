use crate::core::FunctionMetrics;
use std::path::Path;
use syn::ItemFn;

/// Context for Rust pattern detection combining AST and metadata
pub struct RustFunctionContext<'a> {
    /// Parsed function AST from syn
    pub item_fn: &'a ItemFn,

    /// Computed metrics (may be None during initial analysis)
    pub metrics: Option<&'a FunctionMetrics>,

    /// Parent impl block context if this is a method
    pub impl_context: Option<ImplContext>,

    /// File path for error reporting
    pub file_path: &'a Path,
}

#[derive(Clone, Debug)]
pub struct ImplContext {
    pub impl_type: String,
    pub is_trait_impl: bool,
    pub trait_name: Option<String>,
}

impl<'a> RustFunctionContext<'a> {
    pub fn from_item_fn(item_fn: &'a ItemFn, file_path: &'a Path) -> Self {
        Self {
            item_fn,
            metrics: None,
            impl_context: None,
            file_path,
        }
    }

    pub fn with_impl_context(mut self, ctx: ImplContext) -> Self {
        self.impl_context = Some(ctx);
        self
    }

    /// Check if function is async (leverages existing capability)
    pub fn is_async(&self) -> bool {
        self.item_fn.sig.asyncness.is_some()
    }

    /// Get function body for AST traversal
    pub fn body(&self) -> &syn::Block {
        &self.item_fn.block
    }

    /// Get function body as string for reporting (NOT for pattern matching)
    #[allow(dead_code, unused_imports)]
    pub fn body_text(&self) -> String {
        use quote::ToTokens;
        quote::quote!(#(self.item_fn.block)).to_string()
    }

    /// Check if this is a trait method implementation
    pub fn is_trait_impl(&self) -> bool {
        self.impl_context
            .as_ref()
            .map(|ctx| ctx.is_trait_impl)
            .unwrap_or(false)
    }

    /// Get trait name if this is a trait implementation
    pub fn trait_name(&self) -> Option<&str> {
        self.impl_context
            .as_ref()
            .and_then(|ctx| ctx.trait_name.as_deref())
    }
}
