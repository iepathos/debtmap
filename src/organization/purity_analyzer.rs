/// Three-level purity classification for god object scoring.
///
/// This module provides purity analysis that distinguishes between pure functions
/// (no side effects), probably pure functions (likely no side effects), and impure
/// functions (has side effects). Pure functions receive reduced weighting in god object
/// scoring to reward functional programming patterns.
///
/// # Purity Levels
///
/// - **Pure** (weight 0.3): Guaranteed no side effects - no mut params, no I/O, no mutations
/// - **ProbablyPure** (weight 0.5): Likely no side effects - static/associated functions
/// - **Impure** (weight 1.0): Has side effects - uses &mut, I/O, async, or mutations
use crate::analyzers::purity_detector::PurityDetector;
use serde::{Deserialize, Serialize};
use syn::{ItemFn, Receiver, Signature};

/// Three-level purity classification for functions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PurityLevel {
    /// Guaranteed no side effects (weight 0.3)
    Pure,
    /// Likely no side effects (weight 0.5)
    ProbablyPure,
    /// Has side effects (weight 1.0)
    Impure,
}

impl PurityLevel {
    /// Get the weight multiplier for this purity level
    ///
    /// Pure functions contribute less to god object score than impure functions,
    /// rewarding functional programming patterns with many small pure helpers.
    pub fn weight_multiplier(&self) -> f64 {
        match self {
            PurityLevel::Pure => 0.3,
            PurityLevel::ProbablyPure => 0.5,
            PurityLevel::Impure => 1.0,
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            PurityLevel::Pure => "pure (no side effects)",
            PurityLevel::ProbablyPure => "probably pure (likely no side effects)",
            PurityLevel::Impure => "impure (has side effects)",
        }
    }
}

/// Signature-level purity indicators extracted from function signature
#[derive(Debug, Clone)]
pub struct PurityIndicators {
    pub has_mutable_receiver: bool,
    pub has_mutable_params: bool,
    pub is_async: bool,
    pub is_static: bool,
}

/// Analyzer for determining function purity level
pub struct PurityAnalyzer;

impl PurityAnalyzer {
    /// Analyze a function and determine its purity level
    ///
    /// # Classification Logic
    ///
    /// 1. Check signature for obvious impurity (mut params, async)
    /// 2. Use existing PurityDetector to check for side effects in body
    /// 3. Classify as Pure if static with no side effects
    /// 4. Classify as Impure if has mutations, I/O, or unsafe
    /// 5. Otherwise classify as ProbablyPure
    pub fn analyze(item_fn: &ItemFn) -> PurityLevel {
        let sig_indicators = Self::analyze_signature(&item_fn.sig);

        // Immediate impurity checks from signature
        if sig_indicators.has_mutable_receiver
            || sig_indicators.has_mutable_params
            || sig_indicators.is_async
        {
            return PurityLevel::Impure;
        }

        // Use existing purity detector for body analysis
        let mut detector = PurityDetector::new();
        let analysis = detector.is_pure_function(item_fn);

        // Classify based on purity analysis
        if !analysis.is_pure {
            PurityLevel::Impure
        } else if sig_indicators.is_static {
            // Static functions with no side effects are definitely pure
            PurityLevel::Pure
        } else {
            // Non-static functions with no detected side effects are probably pure
            // (might use &self for reading state, which is acceptable)
            PurityLevel::ProbablyPure
        }
    }

    /// Analyze function signature for purity indicators
    pub fn analyze_signature(sig: &Signature) -> PurityIndicators {
        PurityIndicators {
            has_mutable_receiver: Self::has_mut_receiver(sig),
            has_mutable_params: Self::has_mut_params(sig),
            is_async: sig.asyncness.is_some(),
            is_static: Self::is_static_fn(sig),
        }
    }

    /// Check if function has &mut self or mut self receiver
    fn has_mut_receiver(sig: &Signature) -> bool {
        sig.inputs.iter().any(|arg| {
            if let syn::FnArg::Receiver(Receiver { mutability, .. }) = arg {
                mutability.is_some()
            } else {
                false
            }
        })
    }

    /// Check if function has mutable parameters (excluding self)
    fn has_mut_params(sig: &Signature) -> bool {
        sig.inputs.iter().any(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                // Check if the type is a mutable reference
                matches!(&*pat_type.ty, syn::Type::Reference(type_ref) if type_ref.mutability.is_some())
            } else {
                false
            }
        })
    }

    /// Check if function is static (no self parameter)
    fn is_static_fn(sig: &Signature) -> bool {
        !sig.inputs
            .iter()
            .any(|arg| matches!(arg, syn::FnArg::Receiver(_)))
    }

    /// Check if function signature indicates purity
    ///
    /// A pure signature has:
    /// - No mutable receiver
    /// - No mutable parameters
    /// - Not async
    pub fn is_pure_signature(sig: &Signature) -> bool {
        let indicators = Self::analyze_signature(sig);
        !indicators.has_mutable_receiver && !indicators.has_mutable_params && !indicators.is_async
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_pure_function_simple_calculation() {
        let func: ItemFn = parse_quote! {
            fn add(x: i32, y: i32) -> i32 {
                x + y
            }
        };
        assert_eq!(PurityAnalyzer::analyze(&func), PurityLevel::Pure);
    }

    #[test]
    fn test_pure_function_string_operation() {
        let func: ItemFn = parse_quote! {
            fn capitalize_first(s: &str) -> String {
                s.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()
            }
        };
        assert_eq!(PurityAnalyzer::analyze(&func), PurityLevel::Pure);
    }

    #[test]
    fn test_probably_pure_with_self() {
        let func: ItemFn = parse_quote! {
            fn get_name(&self) -> &str {
                &self.name
            }
        };
        // Has &self so not static, but no side effects
        assert_eq!(PurityAnalyzer::analyze(&func), PurityLevel::ProbablyPure);
    }

    #[test]
    fn test_impure_with_mut_self() {
        let func: ItemFn = parse_quote! {
            fn increment(&mut self) {
                self.count += 1;
            }
        };
        assert_eq!(PurityAnalyzer::analyze(&func), PurityLevel::Impure);
    }

    #[test]
    fn test_impure_with_mut_param() {
        let func: ItemFn = parse_quote! {
            fn modify_vec(v: &mut Vec<i32>) {
                v.push(42);
            }
        };
        assert_eq!(PurityAnalyzer::analyze(&func), PurityLevel::Impure);
    }

    #[test]
    fn test_impure_with_io() {
        let func: ItemFn = parse_quote! {
            fn log_message(msg: &str) {
                println!("{}", msg);
            }
        };
        assert_eq!(PurityAnalyzer::analyze(&func), PurityLevel::Impure);
    }

    #[test]
    fn test_impure_async_function() {
        let func: ItemFn = parse_quote! {
            async fn fetch_data() -> String {
                String::from("data")
            }
        };
        assert_eq!(PurityAnalyzer::analyze(&func), PurityLevel::Impure);
    }

    #[test]
    fn test_weight_multipliers() {
        assert_eq!(PurityLevel::Pure.weight_multiplier(), 0.3);
        assert_eq!(PurityLevel::ProbablyPure.weight_multiplier(), 0.5);
        assert_eq!(PurityLevel::Impure.weight_multiplier(), 1.0);
    }

    #[test]
    fn test_analyze_signature_pure() {
        let sig: Signature = parse_quote! {
            fn pure_fn(x: i32) -> i32
        };
        let indicators = PurityAnalyzer::analyze_signature(&sig);
        assert!(!indicators.has_mutable_receiver);
        assert!(!indicators.has_mutable_params);
        assert!(!indicators.is_async);
        assert!(indicators.is_static);
    }

    #[test]
    fn test_analyze_signature_impure_mut_receiver() {
        let sig: Signature = parse_quote! {
            fn mutate(&mut self, value: i32)
        };
        let indicators = PurityAnalyzer::analyze_signature(&sig);
        assert!(indicators.has_mutable_receiver);
        assert!(!indicators.is_static);
    }

    #[test]
    fn test_analyze_signature_impure_mut_param() {
        let sig: Signature = parse_quote! {
            fn modify(data: &mut Vec<i32>)
        };
        let indicators = PurityAnalyzer::analyze_signature(&sig);
        assert!(indicators.has_mutable_params);
        assert!(indicators.is_static);
    }

    #[test]
    fn test_analyze_signature_async() {
        let sig: Signature = parse_quote! {
            async fn async_fn() -> Result<String>
        };
        let indicators = PurityAnalyzer::analyze_signature(&sig);
        assert!(indicators.is_async);
    }

    #[test]
    fn test_is_pure_signature() {
        let pure_sig: Signature = parse_quote! {
            fn pure(x: i32) -> i32
        };
        assert!(PurityAnalyzer::is_pure_signature(&pure_sig));

        let impure_sig: Signature = parse_quote! {
            fn impure(&mut self, x: i32)
        };
        assert!(!PurityAnalyzer::is_pure_signature(&impure_sig));
    }

    #[test]
    fn test_combined_purity_complexity_weight() {
        // This test demonstrates the combined weighting
        use crate::organization::complexity_weighting::calculate_complexity_weight;

        // Pure simple function: complexity 1
        let complexity_weight_1 = calculate_complexity_weight(1);
        let purity_weight = PurityLevel::Pure.weight_multiplier();
        let total_weight = complexity_weight_1 * purity_weight;
        // Should be ~0.19 * 0.3 = ~0.06
        assert!((total_weight - 0.06).abs() < 0.02);

        // Impure complex function: complexity 17
        let complexity_weight_17 = calculate_complexity_weight(17);
        let purity_weight_impure = PurityLevel::Impure.weight_multiplier();
        let total_weight_impure = complexity_weight_17 * purity_weight_impure;
        // Should be ~13.5 * 1.0 = ~13.5
        assert!((total_weight_impure - 13.5).abs() < 0.5);
    }
}
