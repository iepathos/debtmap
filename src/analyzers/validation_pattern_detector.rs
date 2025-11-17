//! Validation Pattern Detector
//!
//! Detects repetitive validation patterns (e.g., multiple if-return checks)
//! to populate ValidationSignals for pattern-based complexity adjustments.

use crate::complexity::pattern_adjustments::{PatternMatchRecognizer, PatternRecognizer};
use crate::priority::complexity_patterns::ValidationSignals;
use syn::{visit::Visit, Block, Expr, ExprReturn, Stmt};

/// Detector for repetitive validation patterns
pub struct ValidationPatternDetector {
    pattern_recognizer: PatternMatchRecognizer,
}

impl ValidationPatternDetector {
    pub fn new() -> Self {
        Self {
            pattern_recognizer: PatternMatchRecognizer::new(),
        }
    }

    /// Detect validation pattern from AST block
    pub fn detect(&self, block: &Block, function_name: &str) -> Option<ValidationSignals> {
        // Count validation checks and early returns
        let mut validator = ValidationVisitor::new();
        validator.visit_block(block);

        let check_count = validator.if_count;
        let early_return_count = validator.early_return_count;

        // Require at least 3 validation checks to consider it a pattern
        if check_count < 3 {
            return None;
        }

        // Check if pattern matcher also detected this as repetitive
        let pattern_info = PatternRecognizer::detect(&self.pattern_recognizer, block)?;

        // Calculate structural similarity based on pattern detection
        // If pattern recognizer detected it, structural similarity is high
        let structural_similarity = if pattern_info.condition_count >= 3 {
            // Strong pattern detected
            0.85
        } else {
            // Weak or no pattern
            0.5
        };

        // Check if function name contains validation keywords
        let has_validation_name = function_name.to_lowercase().contains("validate")
            || function_name.to_lowercase().contains("check")
            || function_name.to_lowercase().contains("verify");

        // Calculate confidence based on signals
        let confidence = calculate_validation_confidence(
            check_count,
            early_return_count,
            structural_similarity,
            has_validation_name,
        );

        // Require minimum confidence to avoid false positives
        if confidence < 0.6 {
            return None;
        }

        Some(ValidationSignals {
            check_count,
            early_return_count,
            structural_similarity,
            has_validation_name,
            confidence,
        })
    }
}

impl Default for ValidationPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate confidence score for validation pattern detection
fn calculate_validation_confidence(
    check_count: u32,
    early_return_count: u32,
    structural_similarity: f64,
    has_validation_name: bool,
) -> f64 {
    let mut confidence = 0.0;

    // More checks = higher confidence (up to 0.4)
    confidence += (check_count as f64 / 20.0).min(0.4);

    // High early return ratio = higher confidence (up to 0.3)
    let early_return_ratio = early_return_count as f64 / check_count.max(1) as f64;
    confidence += (early_return_ratio * 0.3).min(0.3);

    // Structural similarity contributes (up to 0.2)
    confidence += structural_similarity * 0.2;

    // Validation-related name is a strong signal (0.1)
    if has_validation_name {
        confidence += 0.1;
    }

    confidence.min(1.0)
}

/// Visitor to count validation-related patterns
struct ValidationVisitor {
    if_count: u32,
    early_return_count: u32,
    depth: u32,
}

impl ValidationVisitor {
    fn new() -> Self {
        Self {
            if_count: 0,
            early_return_count: 0,
            depth: 0,
        }
    }

    /// Check if a block contains an early return
    fn has_early_return(&self, block: &Block) -> bool {
        block
            .stmts
            .iter()
            .any(|stmt| matches!(stmt, Stmt::Expr(Expr::Return(_), _)))
    }

    /// Check if return expression is Result::Err or similar error pattern
    #[allow(dead_code)]
    fn is_error_return(&self, ret: &ExprReturn) -> bool {
        if let Some(ref expr) = ret.expr {
            match &**expr {
                // Err(...) pattern
                Expr::Call(call) => {
                    if let Expr::Path(path) = &*call.func {
                        if let Some(segment) = path.path.segments.last() {
                            return segment.ident == "Err";
                        }
                    }
                    false
                }
                // bail!(...), anyhow!(...) macros
                Expr::Macro(mac) => {
                    let path = &mac.mac.path;
                    if let Some(segment) = path.segments.last() {
                        let name = segment.ident.to_string();
                        return name == "bail" || name == "anyhow";
                    }
                    false
                }
                _ => false,
            }
        } else {
            false
        }
    }
}

impl<'ast> Visit<'ast> for ValidationVisitor {
    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        // Only count top-level if statements (not nested)
        if self.depth == 0 {
            if let Stmt::Expr(Expr::If(if_expr), _) = stmt {
                self.if_count += 1;

                // Check if then branch has early return
                if self.has_early_return(&if_expr.then_branch) {
                    self.early_return_count += 1;
                }
            }
        }

        syn::visit::visit_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        // Track nesting depth to only count top-level ifs
        match expr {
            Expr::If(_) | Expr::Block(_) => {
                self.depth += 1;
                syn::visit::visit_expr(self, expr);
                self.depth -= 1;
            }
            _ => {
                syn::visit::visit_expr(self, expr);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn detect_repetitive_validation_pattern() {
        let block: Block = parse_quote! {
            {
                if config.field1.is_none() {
                    return Err(anyhow!("field1 required"));
                }
                if config.field2.is_none() {
                    return Err(anyhow!("field2 required"));
                }
                if config.field3.is_none() {
                    return Err(anyhow!("field3 required"));
                }
                Ok(())
            }
        };

        let detector = ValidationPatternDetector::new();
        let signals = detector.detect(&block, "validate_config");

        assert!(signals.is_some());
        let signals = signals.unwrap();
        assert_eq!(signals.check_count, 3);
        assert_eq!(signals.early_return_count, 3);
        assert!(signals.has_validation_name);
        assert!(signals.confidence >= 0.6);
    }

    #[test]
    fn no_detection_for_few_checks() {
        let block: Block = parse_quote! {
            {
                if x > 10 {
                    return true;
                }
                false
            }
        };

        let detector = ValidationPatternDetector::new();
        let signals = detector.detect(&block, "calculate");

        assert!(signals.is_none());
    }

    #[test]
    fn detect_validation_name_signal() {
        let block: Block = parse_quote! {
            {
                if config.a.is_none() { return Err(anyhow!("error")); }
                if config.b.is_none() { return Err(anyhow!("error")); }
                if config.c.is_none() { return Err(anyhow!("error")); }
                if config.d.is_none() { return Err(anyhow!("error")); }
                Ok(())
            }
        };

        let detector = ValidationPatternDetector::new();
        let signals_validate = detector.detect(&block, "validate_input");
        let signals_other = detector.detect(&block, "process_data");

        // Both should be detected (4 checks with similar structure)
        // but validate_input should have has_validation_name=true
        assert!(signals_validate.is_some());
        assert!(signals_validate.unwrap().has_validation_name);

        assert!(signals_other.is_some());
        assert!(!signals_other.unwrap().has_validation_name);
    }
}
