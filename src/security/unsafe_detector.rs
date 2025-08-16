use crate::core::{DebtItem, DebtType, Priority};
use std::path::Path;
use syn::visit::Visit;
use syn::{ExprUnsafe, File, ItemFn, ItemImpl};

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

        let (context, risk_level) = if unsafe_content.contains("transmute") {
            (
                "Contains transmute - very dangerous type casting",
                "Critical",
            )
        } else if unsafe_content.contains("raw_pointer")
            || unsafe_content.contains("*const")
            || unsafe_content.contains("*mut")
        {
            (
                "Raw pointer manipulation - potential memory safety issues",
                "High",
            )
        } else if unsafe_content.contains("ffi") || unsafe_content.contains("extern") {
            ("FFI usage - external code interaction", "High")
        } else if unsafe_content.contains("std::ptr::") {
            ("Pointer operations - requires careful review", "High")
        } else if unsafe_content.contains("mem::") {
            ("Memory manipulation - potential undefined behavior", "High")
        } else {
            ("General unsafe code - requires security review", "Medium")
        };

        // Use a placeholder line number since syn doesn't provide it directly
        self.add_unsafe_debt(0, context, risk_level);

        syn::visit::visit_expr_unsafe(self, i);
    }
}
