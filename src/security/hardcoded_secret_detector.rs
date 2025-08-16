use crate::core::{DebtItem, DebtType, Priority};
use regex::Regex;
use std::path::Path;
use syn::visit::Visit;
use syn::{Expr, ExprLit, File, Lit};

pub fn detect_hardcoded_secrets(file: &File, path: &Path) -> Vec<DebtItem> {
    let mut visitor = SecretVisitor::new(path);
    visitor.visit_file(file);
    visitor.debt_items
}

struct SecretVisitor {
    path: std::path::PathBuf,
    debt_items: Vec<DebtItem>,
    secret_patterns: Vec<(Regex, String)>,
}

impl SecretVisitor {
    fn new(path: &Path) -> Self {
        let patterns = vec![
            (
                Regex::new(r#"(?i)(api[_-]?key|apikey)[\s]*[:=][\s]*['"][\w\-]{20,}['"]"#).unwrap(),
                "API key".to_string(),
            ),
            (
                Regex::new(r#"(?i)(secret|password|passwd|pwd)[\s]*[:=][\s]*['"][^'"]{8,}['"]"#).unwrap(),
                "Password or secret".to_string(),
            ),
            (
                Regex::new(r#"(?i)(token|bearer)[\s]*[:=][\s]*['"][\w\-\.]{20,}['"]"#).unwrap(),
                "Authentication token".to_string(),
            ),
            (
                Regex::new(r#"(?i)aws[_-]?access[_-]?key[_-]?id[\s]*[:=][\s]*['"][A-Z0-9]{20}['"]"#).unwrap(),
                "AWS Access Key".to_string(),
            ),
            (
                Regex::new(r#"(?i)aws[_-]?secret[_-]?access[_-]?key[\s]*[:=][\s]*['"][A-Za-z0-9/+=]{40}['"]"#).unwrap(),
                "AWS Secret Key".to_string(),
            ),
            (
                Regex::new(r"sk[_-]live[_-][0-9a-zA-Z]{24,}").unwrap(),
                "Stripe API key".to_string(),
            ),
            (
                Regex::new(r#"(?i)private[_-]?key[\s]*[:=][\s]*['"]-----BEGIN"#).unwrap(),
                "Private key".to_string(),
            ),
        ];

        Self {
            path: path.to_path_buf(),
            debt_items: Vec::new(),
            secret_patterns: patterns,
        }
    }

    fn check_string_for_secrets(&mut self, s: &str, line: usize) {
        // Calculate entropy as an additional check
        let entropy = calculate_shannon_entropy(s);

        for (pattern, secret_type) in &self.secret_patterns {
            if pattern.is_match(s) {
                self.debt_items.push(DebtItem {
                    id: format!("security-secret-{}-{}", self.path.display(), line),
                    debt_type: DebtType::Security,
                    priority: Priority::Critical,
                    file: self.path.clone(),
                    line,
                    column: None,
                    message: format!("Critical: Hardcoded {} detected", secret_type),
                    context: Some(format!(
                        "Entropy: {:.2} - Move to environment variable or secure configuration",
                        entropy
                    )),
                });
                return; // Only report the first match
            }
        }

        // Check for high-entropy strings that might be secrets
        if entropy > 4.5 && s.len() > 20 && !s.contains(' ') {
            // Check if it looks like a key/token
            if s.chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                self.debt_items.push(DebtItem {
                    id: format!("security-entropy-{}-{}", self.path.display(), line),
                    debt_type: DebtType::Security,
                    priority: Priority::High,
                    file: self.path.clone(),
                    line,
                    column: None,
                    message: "Potential hardcoded secret detected (high entropy)".to_string(),
                    context: Some(format!(
                        "Entropy: {:.2} - Review if this is a secret",
                        entropy
                    )),
                });
            }
        }
    }
}

impl<'ast> Visit<'ast> for SecretVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if let Expr::Lit(ExprLit {
            lit: Lit::Str(lit_str),
            ..
        }) = expr
        {
            let value = lit_str.value();
            // Use a placeholder line number since syn doesn't provide it directly
            self.check_string_for_secrets(&value, 0);
        }
        syn::visit::visit_expr(self, expr);
    }
}

fn calculate_shannon_entropy(s: &str) -> f64 {
    let mut char_counts = std::collections::HashMap::new();
    let len = s.len() as f64;

    for c in s.chars() {
        *char_counts.entry(c).or_insert(0) += 1;
    }

    char_counts
        .values()
        .map(|&count| {
            let probability = count as f64 / len;
            -probability * probability.log2()
        })
        .sum()
}
