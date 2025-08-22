//! AST-based visitor pattern detection for complexity reduction
//!
//! This module detects visitor patterns and other trait-based patterns
//! that should use logarithmic complexity scaling instead of linear.

use im::{HashMap, HashSet};
use std::time::SystemTime;
use syn::{File, Item, ItemFn, ItemImpl, Path as SynPath};

/// Information about a detected visitor pattern
#[derive(Debug, Clone)]
pub struct VisitorInfo {
    pub trait_name: String,
    pub method_name: String,
    pub arm_count: usize,
    pub is_exhaustive: bool,
    pub confidence: f32,
}

/// Pattern detection results
#[derive(Debug, Clone)]
pub struct PatternInfo {
    pub pattern_type: PatternType,
    pub base_complexity: u32,
    pub adjusted_complexity: u32,
    pub confidence: f32,
}

/// Types of patterns detected
#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    Visitor,
    ExhaustiveMatch,
    SimpleMapping,
    Standard,
}

/// Cache for pattern detection results
#[derive(Debug, Clone)]
pub struct PatternCache {
    pub file_hash: u64,
    pub patterns: HashMap<String, PatternInfo>,
    pub timestamp: SystemTime,
}

/// Main visitor pattern detector
pub struct VisitorPatternDetector {
    visitor_traits: HashSet<String>,
}

impl Default for VisitorPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl VisitorPatternDetector {
    /// Create a new visitor pattern detector with default trait names
    pub fn new() -> Self {
        let mut visitor_traits = HashSet::new();
        // Common visitor trait names
        visitor_traits.insert("Visit".to_string());
        visitor_traits.insert("Visitor".to_string());
        visitor_traits.insert("Fold".to_string());
        visitor_traits.insert("VisitMut".to_string());
        visitor_traits.insert("Walker".to_string());
        visitor_traits.insert("Traverser".to_string());

        Self { visitor_traits }
    }

    /// Add a custom visitor trait name
    pub fn add_visitor_trait(&mut self, trait_name: String) {
        self.visitor_traits.insert(trait_name);
    }

    /// Detect if a function implements a visitor pattern
    pub fn detect_visitor_pattern(&mut self, file: &File, func: &ItemFn) -> Option<VisitorInfo> {
        // Check trait implementations
        for item in &file.items {
            if let Item::Impl(impl_block) = item {
                if self.is_visitor_trait(impl_block) && self.contains_function(impl_block, func) {
                    return Some(self.analyze_visitor(func));
                }
            }
        }
        None
    }

    /// Check if an impl block is for a visitor trait
    fn is_visitor_trait(&self, impl_block: &ItemImpl) -> bool {
        if let Some((_, path, _)) = &impl_block.trait_ {
            if let Some(trait_name) = self.extract_trait_name(path) {
                // Check exact matches
                if self.visitor_traits.contains(&trait_name) {
                    return true;
                }

                // Check for generic Visit traits like Visit<'ast>
                if trait_name.starts_with("Visit")
                    || trait_name.starts_with("Visitor")
                    || trait_name.starts_with("Fold")
                {
                    return true;
                }
            }
        }
        false
    }

    /// Extract trait name from a path
    fn extract_trait_name(&self, path: &SynPath) -> Option<String> {
        path.segments.last().map(|seg| seg.ident.to_string())
    }

    /// Check if an impl block contains a specific function
    fn contains_function(&self, impl_block: &ItemImpl, func: &ItemFn) -> bool {
        let func_name = func.sig.ident.to_string();

        impl_block.items.iter().any(|item| {
            if let syn::ImplItem::Fn(method) = item {
                method.sig.ident == func_name
            } else {
                false
            }
        })
    }

    /// Analyze a visitor function to extract characteristics
    fn analyze_visitor(&self, func: &ItemFn) -> VisitorInfo {
        use syn::visit::Visit;

        let mut visitor = MatchArmCounter::default();
        visitor.visit_block(&func.block);

        VisitorInfo {
            trait_name: "Visit".to_string(), // We'll enhance this later
            method_name: func.sig.ident.to_string(),
            arm_count: visitor.max_arms,
            is_exhaustive: visitor.has_wildcard,
            confidence: if visitor.max_arms > 0 { 0.9 } else { 0.5 },
        }
    }

    /// Detect if a function is a visitor by its name and structure
    pub fn detect_visitor_by_pattern(&self, func: &ItemFn) -> Option<VisitorInfo> {
        let name = func.sig.ident.to_string();

        // Check if name matches visitor patterns
        if name.starts_with("visit_")
            || name.starts_with("walk_")
            || name.starts_with("traverse_")
            || name.starts_with("fold_")
        {
            // Analyze the function structure
            use syn::visit::Visit;
            let mut visitor = MatchArmCounter::default();
            visitor.visit_block(&func.block);

            // If it has a large match statement, it's likely a visitor
            if visitor.max_arms >= 3 {
                return Some(VisitorInfo {
                    trait_name: "Visitor".to_string(),
                    method_name: name,
                    arm_count: visitor.max_arms,
                    is_exhaustive: visitor.has_wildcard,
                    confidence: 0.8,
                });
            }
        }

        None
    }
}

/// Helper visitor to count match arms
#[derive(Default)]
struct MatchArmCounter {
    max_arms: usize,
    has_wildcard: bool,
}

impl<'ast> syn::visit::Visit<'ast> for MatchArmCounter {
    fn visit_expr_match(&mut self, match_expr: &'ast syn::ExprMatch) {
        self.max_arms = self.max_arms.max(match_expr.arms.len());

        // Check for wildcard pattern
        for arm in &match_expr.arms {
            if matches!(&arm.pat, syn::Pat::Wild(_) | syn::Pat::Ident(_)) {
                self.has_wildcard = true;
            }
        }

        syn::visit::visit_expr_match(self, match_expr);
    }
}

/// Match characteristics for exhaustive match detection
#[derive(Debug, Clone)]
pub struct MatchCharacteristics {
    pub pattern_type: PatternType,
    pub arm_count: usize,
    pub max_arm_complexity: u32,
    pub is_simple_mapping: bool,
    pub has_default: bool,
}

impl Default for MatchCharacteristics {
    fn default() -> Self {
        Self {
            pattern_type: PatternType::Standard,
            arm_count: 0,
            max_arm_complexity: 0,
            is_simple_mapping: false,
            has_default: false,
        }
    }
}

/// Analyzer for match expressions
pub struct MatchAnalyzer;

impl MatchAnalyzer {
    /// Analyze a function to detect match patterns
    pub fn analyze_match_pattern(&self, func: &ItemFn) -> MatchCharacteristics {
        use syn::visit::Visit;

        let mut visitor = MatchPatternVisitor::default();
        visitor.visit_block(&func.block);

        if visitor.match_count == 1 && visitor.is_primary_match {
            MatchCharacteristics {
                pattern_type: if visitor.is_simple_mapping {
                    PatternType::SimpleMapping
                } else {
                    PatternType::ExhaustiveMatch
                },
                arm_count: visitor.max_arms,
                max_arm_complexity: visitor.max_arm_complexity,
                is_simple_mapping: visitor.is_simple_mapping,
                has_default: visitor.has_wildcard,
            }
        } else {
            MatchCharacteristics::default()
        }
    }
}

/// Visitor to analyze match patterns
#[derive(Default)]
struct MatchPatternVisitor {
    match_count: usize,
    max_arms: usize,
    max_arm_complexity: u32,
    is_simple_mapping: bool,
    is_primary_match: bool,
    has_wildcard: bool,
}

impl<'ast> syn::visit::Visit<'ast> for MatchPatternVisitor {
    fn visit_expr_match(&mut self, match_expr: &'ast syn::ExprMatch) {
        self.match_count += 1;
        self.max_arms = self.max_arms.max(match_expr.arms.len());

        // Check if all arms are simple
        let all_simple = match_expr.arms.iter().all(|arm| {
            matches!(
                &*arm.body,
                syn::Expr::Lit(_)
                    | syn::Expr::Path(_)
                    | syn::Expr::Return(_)
                    | syn::Expr::Break(_)
                    | syn::Expr::Continue(_)
            )
        });

        if all_simple {
            self.is_simple_mapping = true;
        }

        // Check if this is the primary logic (more than 50% of arms)
        if match_expr.arms.len() >= 3 {
            self.is_primary_match = true;
        }

        // Check for wildcard
        for arm in &match_expr.arms {
            if matches!(&arm.pat, syn::Pat::Wild(_)) {
                self.has_wildcard = true;
            }
        }

        syn::visit::visit_expr_match(self, match_expr);
    }
}

/// Apply pattern-based scaling to complexity
pub fn apply_pattern_scaling(base_complexity: u32, pattern: &PatternInfo) -> u32 {
    match pattern.pattern_type {
        PatternType::Visitor => {
            // log2 scaling for visitors
            let log_complexity = (base_complexity as f32).log2().ceil();
            log_complexity.max(1.0) as u32
        }
        PatternType::ExhaustiveMatch => {
            // sqrt scaling for exhaustive matches
            let sqrt_complexity = (base_complexity as f32).sqrt().ceil();
            sqrt_complexity.max(2.0) as u32
        }
        PatternType::SimpleMapping => {
            // 80% reduction for simple mappings
            ((base_complexity as f32) * 0.2).max(1.0) as u32
        }
        PatternType::Standard => base_complexity,
    }
}

/// Detect visitor pattern for a function
pub fn detect_visitor_pattern(file: &File, func: &ItemFn) -> Option<PatternInfo> {
    let mut detector = VisitorPatternDetector::new();

    // Try to detect by trait implementation
    if let Some(visitor_info) = detector.detect_visitor_pattern(file, func) {
        let base = visitor_info.arm_count as u32;
        let adjusted = apply_pattern_scaling(
            base,
            &PatternInfo {
                pattern_type: PatternType::Visitor,
                base_complexity: base,
                adjusted_complexity: 0, // Will be calculated
                confidence: visitor_info.confidence,
            },
        );

        return Some(PatternInfo {
            pattern_type: PatternType::Visitor,
            base_complexity: base,
            adjusted_complexity: adjusted,
            confidence: visitor_info.confidence,
        });
    }

    // Try to detect by pattern
    if let Some(visitor_info) = detector.detect_visitor_by_pattern(func) {
        let base = visitor_info.arm_count as u32;
        let adjusted = apply_pattern_scaling(
            base,
            &PatternInfo {
                pattern_type: PatternType::Visitor,
                base_complexity: base,
                adjusted_complexity: 0,
                confidence: visitor_info.confidence,
            },
        );

        return Some(PatternInfo {
            pattern_type: PatternType::Visitor,
            base_complexity: base,
            adjusted_complexity: adjusted,
            confidence: visitor_info.confidence,
        });
    }

    // Check for exhaustive match patterns
    let analyzer = MatchAnalyzer;
    let match_info = analyzer.analyze_match_pattern(func);

    if match_info.arm_count >= 3 {
        let base = match_info.arm_count as u32;
        let pattern_type = match_info.pattern_type;
        let adjusted = apply_pattern_scaling(
            base,
            &PatternInfo {
                pattern_type: pattern_type.clone(),
                base_complexity: base,
                adjusted_complexity: 0,
                confidence: 0.7,
            },
        );

        return Some(PatternInfo {
            pattern_type,
            base_complexity: base,
            adjusted_complexity: adjusted,
            confidence: 0.7,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_visitor_trait_detection() {
        let mut detector = VisitorPatternDetector::new();

        let impl_block: ItemImpl = parse_quote! {
            impl Visit for MyVisitor {
                fn visit_expr(&mut self, expr: &Expr) {
                    match expr {
                        Expr::Binary(b) => self.visit_binary(b),
                        Expr::Unary(u) => self.visit_unary(u),
                        Expr::Call(c) => self.visit_call(c),
                        _ => {}
                    }
                }
            }
        };

        assert!(detector.is_visitor_trait(&impl_block));
    }

    #[test]
    fn test_visitor_by_pattern() {
        let detector = VisitorPatternDetector::new();

        let func: ItemFn = parse_quote! {
            fn visit_expr(&mut self, expr: &Expr) {
                match expr {
                    Expr::Binary(b) => {},
                    Expr::Unary(u) => {},
                    Expr::Call(c) => {},
                    Expr::Method(m) => {},
                    _ => {}
                }
            }
        };

        let result = detector.detect_visitor_by_pattern(&func);
        assert!(result.is_some());

        let info = result.unwrap();
        assert_eq!(info.method_name, "visit_expr");
        assert_eq!(info.arm_count, 5);
    }

    #[test]
    fn test_logarithmic_scaling() {
        let pattern = PatternInfo {
            pattern_type: PatternType::Visitor,
            base_complexity: 34,
            adjusted_complexity: 0,
            confidence: 0.9,
        };

        let adjusted = apply_pattern_scaling(34, &pattern);
        // log2(34) ≈ 5.09, ceil = 6
        assert_eq!(adjusted, 6);
    }

    #[test]
    fn test_sqrt_scaling() {
        let pattern = PatternInfo {
            pattern_type: PatternType::ExhaustiveMatch,
            base_complexity: 16,
            adjusted_complexity: 0,
            confidence: 0.7,
        };

        let adjusted = apply_pattern_scaling(16, &pattern);
        // sqrt(16) = 4
        assert_eq!(adjusted, 4);
    }

    #[test]
    fn test_simple_mapping_scaling() {
        let pattern = PatternInfo {
            pattern_type: PatternType::SimpleMapping,
            base_complexity: 10,
            adjusted_complexity: 0,
            confidence: 0.8,
        };

        let adjusted = apply_pattern_scaling(10, &pattern);
        // 10 * 0.2 = 2
        assert_eq!(adjusted, 2);
    }
}
