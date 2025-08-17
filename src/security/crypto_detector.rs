use crate::core::{DebtItem, DebtType, Priority};
use std::path::Path;
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, ExprPath, File};

pub fn detect_crypto_misuse(file: &File, path: &Path) -> Vec<DebtItem> {
    let mut visitor = CryptoVisitor::new(path);
    visitor.visit_file(file);
    visitor.debt_items
}

struct CryptoVisitor {
    path: std::path::PathBuf,
    debt_items: Vec<DebtItem>,
}

impl CryptoVisitor {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            debt_items: Vec::new(),
        }
    }

    fn check_weak_algorithms(&mut self, name: &str, line: usize) {
        // debtmap:ignore - These are test patterns for crypto vulnerability detection
        let weak_algorithms = [
            (
                "md5",
                "MD5",
                "Cryptographically broken, use SHA-256 or better",
            ),
            (
                "sha1",
                "SHA-1",
                "Weak hash algorithm, use SHA-256 or better",
            ),
            ("des", "DES", "Weak encryption, use AES"),
            ("rc4", "RC4", "Broken cipher, use AES-GCM"),
            (
                "ecb",
                "ECB mode",
                "Insecure block cipher mode, use CBC or GCM",
            ),
        ];

        let name_lower = name.to_lowercase();
        for (pattern, algo_name, recommendation) in weak_algorithms {
            if name_lower.contains(pattern) {
                self.debt_items.push(DebtItem {
                    id: format!("security-crypto-{}-{}", self.path.display(), line),
                    debt_type: DebtType::Security,
                    priority: Priority::Critical,
                    file: self.path.clone(),
                    line,
                    column: None,
                    message: format!("Critical: Weak cryptographic algorithm: {}", algo_name),
                    context: Some(recommendation.to_string()),
                });
                return;
            }
        }
    }

    fn check_insecure_random(&mut self, expr_str: &str, line: usize) {
        if (expr_str.contains("rand()") || expr_str.contains("random()"))
            && !expr_str.contains("cryptographically_secure")
            && !expr_str.contains("OsRng")
        {
            self.debt_items.push(DebtItem {
                id: format!("security-random-{}-{}", self.path.display(), line),
                debt_type: DebtType::Security,
                priority: Priority::High,
                file: self.path.clone(),
                line,
                column: None,
                message: "Insecure random number generation for cryptographic use".to_string(),
                context: Some("Use a cryptographically secure random number generator".to_string()),
            });
        }
    }

    fn check_hardcoded_iv(&mut self, expr_str: &str, line: usize) {
        if (expr_str.contains("iv") || expr_str.contains("nonce"))
            && (expr_str.contains("[0u8") || expr_str.contains("[0x"))
        {
            self.debt_items.push(DebtItem {
                id: format!("security-iv-{}-{}", self.path.display(), line),
                debt_type: DebtType::Security,
                priority: Priority::High,
                file: self.path.clone(),
                line,
                column: None,
                message: "Hardcoded IV/nonce detected".to_string(),
                context: Some(
                    "Use a randomly generated IV for each encryption operation".to_string(),
                ),
            });
        }
    }
}

impl<'ast> Visit<'ast> for CryptoVisitor {
    fn visit_expr_path(&mut self, i: &'ast ExprPath) {
        if let Some(segment) = i.path.segments.last() {
            let name = segment.ident.to_string();
            self.check_weak_algorithms(&name, 0);
        }
        syn::visit::visit_expr_path(self, i);
    }

    fn visit_expr_method_call(&mut self, i: &'ast ExprMethodCall) {
        let method = i.method.to_string();
        self.check_weak_algorithms(&method, 0);

        // Check for specific crypto patterns
        let expr_str = quote::quote!(#i).to_string();
        self.check_insecure_random(&expr_str, 0);
        self.check_hardcoded_iv(&expr_str, 0);

        syn::visit::visit_expr_method_call(self, i);
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        let expr_str = quote::quote!(#expr).to_string();

        // Check for crypto-related patterns
        if expr_str.contains("encrypt") || expr_str.contains("decrypt") || expr_str.contains("hash")
        {
            self.check_insecure_random(&expr_str, 0);
            self.check_hardcoded_iv(&expr_str, 0);
        }

        syn::visit::visit_expr(self, expr);
    }
}
