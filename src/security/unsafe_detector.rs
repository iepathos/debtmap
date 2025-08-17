// debtmap:ignore-start -- This file contains test patterns for unsafe code detection, not actual unsafe code
use crate::core::{DebtItem, DebtType, Priority};
use std::path::Path;
use syn::visit::Visit;
use syn::{ExprUnsafe, File, ItemFn, ItemImpl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnsafePattern {
    Transmute,
    RawPointer,
    Ffi,
    PointerOps,
    MemoryOps,
    General,
}

pub fn detect_unsafe_blocks(file: &File, path: &Path) -> Vec<DebtItem> {
    let mut visitor = UnsafeVisitor::new(path);
    visitor.visit_file(file);
    visitor.debt_items
}

struct UnsafeVisitor {
    path: std::path::PathBuf,
    debt_items: Vec<DebtItem>,
    current_function: Option<String>,
}

impl UnsafeVisitor {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            debt_items: Vec::new(),
            current_function: None,
        }
    }

    /// Check if content contains transmute pattern
    fn contains_transmute(content: &str) -> bool {
        content.contains("transmute")
    }

    /// Check if content contains raw pointer patterns
    fn contains_raw_pointer(content: &str) -> bool {
        content.contains("raw_pointer") || content.contains("*const") || content.contains("*mut")
    }

    /// Check if content contains FFI patterns
    fn contains_ffi(content: &str) -> bool {
        content.contains("ffi") || content.contains("extern")
    }

    /// Classify the type of unsafe pattern found in the code
    fn classify_unsafe_pattern(content: &str) -> UnsafePattern {
        match () {
            _ if Self::contains_transmute(content) => UnsafePattern::Transmute,
            _ if Self::contains_raw_pointer(content) => UnsafePattern::RawPointer,
            _ if Self::contains_ffi(content) => UnsafePattern::Ffi,
            _ if content.contains("std::ptr::") => UnsafePattern::PointerOps,
            _ if content.contains("mem::") => UnsafePattern::MemoryOps,
            _ => UnsafePattern::General,
        }
    }

    /// Get the risk level for a given unsafe pattern
    fn get_risk_level(pattern: &UnsafePattern) -> &'static str {
        match pattern {
            UnsafePattern::Transmute => "Critical",
            UnsafePattern::RawPointer
            | UnsafePattern::Ffi
            | UnsafePattern::PointerOps
            | UnsafePattern::MemoryOps => "High",
            UnsafePattern::General => "Medium",
        }
    }

    /// Get the context description for a given unsafe pattern
    fn get_pattern_context(pattern: &UnsafePattern) -> &'static str {
        match pattern {
            UnsafePattern::Transmute => "Contains transmute - very dangerous type casting",
            UnsafePattern::RawPointer => {
                "Raw pointer manipulation - potential memory safety issues"
            }
            UnsafePattern::Ffi => "FFI usage - external code interaction",
            UnsafePattern::PointerOps => "Pointer operations - requires careful review",
            UnsafePattern::MemoryOps => "Memory manipulation - potential undefined behavior",
            UnsafePattern::General => "General unsafe code - requires security review",
        }
    }

    /// Analyze unsafe content and return context and risk level
    fn analyze_unsafe_content(content: &str) -> (&'static str, &'static str) {
        let pattern = Self::classify_unsafe_pattern(content);
        let context = Self::get_pattern_context(&pattern);
        let risk_level = Self::get_risk_level(&pattern);
        (context, risk_level)
    }

    fn add_unsafe_debt(&mut self, line: usize, context: &str, risk_level: &str) {
        let function_context = self
            .current_function
            .as_ref()
            .map(|f| format!(" in function '{}'", f))
            .unwrap_or_default();

        self.debt_items.push(DebtItem {
            id: format!("security-unsafe-{}-{}", self.path.display(), line),
            debt_type: DebtType::Security,
            priority: match risk_level {
                "Critical" => Priority::Critical,
                "High" => Priority::High,
                _ => Priority::Medium,
            },
            file: self.path.clone(),
            line,
            column: None,
            message: format!("Unsafe block detected{}: {}", function_context, context),
            context: Some(format!("Risk level: {}", risk_level)),
        });
    }
}

impl<'ast> Visit<'ast> for UnsafeVisitor {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        let prev_function = self.current_function.clone();
        self.current_function = Some(i.sig.ident.to_string());
        syn::visit::visit_item_fn(self, i);
        self.current_function = prev_function;
    }

    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        syn::visit::visit_item_impl(self, i);
    }

    fn visit_expr_unsafe(&mut self, i: &'ast ExprUnsafe) {
        // Check the content of the unsafe block for specific patterns
        let unsafe_content = quote::quote!(#i).to_string();
        let (context, risk_level) = Self::analyze_unsafe_content(&unsafe_content);

        // Use a placeholder line number since syn doesn't provide it directly
        self.add_unsafe_debt(0, context, risk_level);

        syn::visit::visit_expr_unsafe(self, i);
    }
}
// debtmap:ignore-end

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_transmute() {
        assert!(UnsafeVisitor::contains_transmute("unsafe { transmute(x) }"));
        assert!(UnsafeVisitor::contains_transmute("std::mem::transmute"));
        assert!(!UnsafeVisitor::contains_transmute("unsafe { *ptr }"));
        assert!(!UnsafeVisitor::contains_transmute(""));
    }

    #[test]
    fn test_contains_raw_pointer() {
        assert!(UnsafeVisitor::contains_raw_pointer("*const u8"));
        assert!(UnsafeVisitor::contains_raw_pointer("*mut i32"));
        assert!(UnsafeVisitor::contains_raw_pointer(
            "raw_pointer operations"
        ));
        assert!(!UnsafeVisitor::contains_raw_pointer("safe code"));
        assert!(!UnsafeVisitor::contains_raw_pointer(""));
    }

    #[test]
    fn test_contains_ffi() {
        assert!(UnsafeVisitor::contains_ffi("ffi::CString"));
        assert!(UnsafeVisitor::contains_ffi("extern \"C\" fn"));
        assert!(UnsafeVisitor::contains_ffi("use libc::ffi"));
        assert!(!UnsafeVisitor::contains_ffi("internal function"));
        assert!(!UnsafeVisitor::contains_ffi(""));
    }

    #[test]
    fn test_classify_unsafe_pattern_transmute() {
        let pattern = UnsafeVisitor::classify_unsafe_pattern("unsafe { transmute::<u8, i8>(x) }");
        assert_eq!(pattern, UnsafePattern::Transmute);
    }

    #[test]
    fn test_classify_unsafe_pattern_raw_pointer() {
        let pattern = UnsafeVisitor::classify_unsafe_pattern("unsafe { *const ptr }");
        assert_eq!(pattern, UnsafePattern::RawPointer);

        let pattern = UnsafeVisitor::classify_unsafe_pattern("unsafe { *mut data }");
        assert_eq!(pattern, UnsafePattern::RawPointer);

        let pattern = UnsafeVisitor::classify_unsafe_pattern("raw_pointer manipulation");
        assert_eq!(pattern, UnsafePattern::RawPointer);
    }

    #[test]
    fn test_classify_unsafe_pattern_ffi() {
        let pattern = UnsafeVisitor::classify_unsafe_pattern("unsafe { ffi::call() }");
        assert_eq!(pattern, UnsafePattern::Ffi);

        let pattern = UnsafeVisitor::classify_unsafe_pattern("extern \"C\" { fn foo(); }");
        assert_eq!(pattern, UnsafePattern::Ffi);
    }

    #[test]
    fn test_classify_unsafe_pattern_pointer_ops() {
        let pattern = UnsafeVisitor::classify_unsafe_pattern("unsafe { std::ptr::null() }");
        assert_eq!(pattern, UnsafePattern::PointerOps);
    }

    #[test]
    fn test_classify_unsafe_pattern_memory_ops() {
        let pattern = UnsafeVisitor::classify_unsafe_pattern("unsafe { mem::zeroed() }");
        assert_eq!(pattern, UnsafePattern::MemoryOps);
    }

    #[test]
    fn test_classify_unsafe_pattern_general() {
        let pattern = UnsafeVisitor::classify_unsafe_pattern("unsafe { some_function() }");
        assert_eq!(pattern, UnsafePattern::General);

        let pattern = UnsafeVisitor::classify_unsafe_pattern("");
        assert_eq!(pattern, UnsafePattern::General);
    }

    #[test]
    fn test_classify_unsafe_pattern_priority() {
        // Transmute should take precedence
        let pattern = UnsafeVisitor::classify_unsafe_pattern("transmute and *const ptr");
        assert_eq!(pattern, UnsafePattern::Transmute);
    }

    #[test]
    fn test_get_risk_level() {
        assert_eq!(
            UnsafeVisitor::get_risk_level(&UnsafePattern::Transmute),
            "Critical"
        );
        assert_eq!(
            UnsafeVisitor::get_risk_level(&UnsafePattern::RawPointer),
            "High"
        );
        assert_eq!(UnsafeVisitor::get_risk_level(&UnsafePattern::Ffi), "High");
        assert_eq!(
            UnsafeVisitor::get_risk_level(&UnsafePattern::PointerOps),
            "High"
        );
        assert_eq!(
            UnsafeVisitor::get_risk_level(&UnsafePattern::MemoryOps),
            "High"
        );
        assert_eq!(
            UnsafeVisitor::get_risk_level(&UnsafePattern::General),
            "Medium"
        );
    }

    #[test]
    fn test_get_pattern_context() {
        assert_eq!(
            UnsafeVisitor::get_pattern_context(&UnsafePattern::Transmute),
            "Contains transmute - very dangerous type casting"
        );
        assert_eq!(
            UnsafeVisitor::get_pattern_context(&UnsafePattern::RawPointer),
            "Raw pointer manipulation - potential memory safety issues"
        );
        assert_eq!(
            UnsafeVisitor::get_pattern_context(&UnsafePattern::Ffi),
            "FFI usage - external code interaction"
        );
        assert_eq!(
            UnsafeVisitor::get_pattern_context(&UnsafePattern::PointerOps),
            "Pointer operations - requires careful review"
        );
        assert_eq!(
            UnsafeVisitor::get_pattern_context(&UnsafePattern::MemoryOps),
            "Memory manipulation - potential undefined behavior"
        );
        assert_eq!(
            UnsafeVisitor::get_pattern_context(&UnsafePattern::General),
            "General unsafe code - requires security review"
        );
    }

    #[test]
    fn test_analyze_unsafe_content_transmute() {
        let (context, risk) = UnsafeVisitor::analyze_unsafe_content("unsafe { transmute(x) }");
        assert_eq!(context, "Contains transmute - very dangerous type casting");
        assert_eq!(risk, "Critical");
    }

    #[test]
    fn test_analyze_unsafe_content_raw_pointer() {
        let (context, risk) = UnsafeVisitor::analyze_unsafe_content("*const ptr");
        assert_eq!(
            context,
            "Raw pointer manipulation - potential memory safety issues"
        );
        assert_eq!(risk, "High");
    }

    #[test]
    fn test_analyze_unsafe_content_ffi() {
        let (context, risk) = UnsafeVisitor::analyze_unsafe_content("extern \"C\" fn");
        assert_eq!(context, "FFI usage - external code interaction");
        assert_eq!(risk, "High");
    }

    #[test]
    fn test_analyze_unsafe_content_general() {
        let (context, risk) = UnsafeVisitor::analyze_unsafe_content("unsafe block");
        assert_eq!(context, "General unsafe code - requires security review");
        assert_eq!(risk, "Medium");
    }
}
