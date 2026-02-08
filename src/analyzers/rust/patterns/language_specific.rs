//! Language-specific data building
//!
//! Builds Rust-specific pattern data for function metrics.

use crate::analysis::rust_patterns::{ImplContext, RustFunctionContext, RustPatternDetector};
use crate::analyzers::rust::types::{FunctionContext, PatternSignals};
use crate::core::LanguageSpecificData;

/// Build language-specific data for Rust (spec 146)
pub fn build_language_specific(
    context: &FunctionContext,
    item_fn: &syn::ItemFn,
    signals: &PatternSignals,
    enable_rust_patterns: bool,
) -> Option<LanguageSpecificData> {
    let impl_context = match (context.is_trait_method, &context.impl_type_name) {
        (true, _) => Some(ImplContext {
            impl_type: context.impl_type_name.clone().unwrap_or_default(),
            is_trait_impl: true,
            trait_name: context.trait_name.clone(),
        }),
        (false, Some(impl_type)) => Some(ImplContext {
            impl_type: impl_type.clone(),
            is_trait_impl: false,
            trait_name: None,
        }),
        _ => None,
    };

    let rust_context = RustFunctionContext {
        item_fn,
        metrics: None,
        impl_context,
        file_path: &context.file,
    };

    if enable_rust_patterns {
        Some(LanguageSpecificData::Rust(
            RustPatternDetector::new().detect_all_patterns(
                &rust_context,
                signals.validation.clone(),
                signals.state_machine.clone(),
                signals.coordinator.clone(),
            ),
        ))
    } else if signals.has_any() {
        Some(LanguageSpecificData::Rust(
            crate::analysis::rust_patterns::RustPatternResult {
                trait_impl: None,
                async_patterns: vec![],
                error_patterns: vec![],
                builder_patterns: vec![],
                validation_signals: signals.validation.clone(),
                state_machine_signals: signals.state_machine.clone(),
                coordinator_signals: signals.coordinator.clone(),
            },
        ))
    } else {
        None
    }
}
