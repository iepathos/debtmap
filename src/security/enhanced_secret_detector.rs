// debtmap:ignore-start -- This file contains test patterns for security detection, not real secrets
use crate::security::types::{SecretType, SecurityDetector, SecurityVulnerability};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use syn::visit::Visit;
use syn::{Expr, ExprLit, File, Lit};

pub struct EnhancedSecretDetector {
    patterns: HashMap<SecretType, Vec<Regex>>,
    entropy_threshold: f64,
    allowlist: Vec<String>,
}

impl Default for EnhancedSecretDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl EnhancedSecretDetector {
    pub fn new() -> Self {
        let mut patterns = HashMap::new();

        // debtmap:ignore - These are test patterns for security detection, not real secrets
        // API Keys
        patterns.insert(
            SecretType::ApiKey,
            vec![
                Regex::new(r"sk-[a-zA-Z0-9]{32,}").unwrap(), // OpenAI/Stripe style
                Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),    // AWS Access Key
                Regex::new(r"ghp_[a-zA-Z0-9]{36}").unwrap(), // GitHub personal access token
                Regex::new(r"gho_[a-zA-Z0-9]{36}").unwrap(), // GitHub OAuth token
                Regex::new(r"github_pat_[a-zA-Z0-9_]{82}").unwrap(), // GitHub fine-grained PAT
                Regex::new(r#"(?i)api[_-]?key['"]?\s*[:=]\s*['"]([a-zA-Z0-9_\-]{20,})['"]"#)
                    .unwrap(),
            ],
        );

        // Passwords
        patterns.insert(
            SecretType::Password,
            vec![
                Regex::new(r#"(?i)(password|passwd|pwd)['\"]?\s*[:=]\s*['"]([^'"]{8,})['"]"#)
                    .unwrap(),
                Regex::new(r#"(?i)db[_-]?pass(word)?['\"]?\s*[:=]\s*['"]([^'"]{8,})['"]"#).unwrap(),
            ],
        );

        // Private Keys
        patterns.insert(
            SecretType::PrivateKey,
            vec![
                Regex::new(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap(),
                Regex::new(r"-----BEGIN PGP PRIVATE KEY BLOCK-----").unwrap(),
            ],
        );

        // Auth Tokens
        patterns.insert(
            SecretType::AuthToken,
            vec![
                Regex::new(
                    r#"(?i)(auth|bearer)[_-]?token['"]?\s*[:=]\s*['"]([a-zA-Z0-9_\-\.]{20,})['"]"#,
                )
                .unwrap(),
                Regex::new(r"xox[baprs]-[0-9]{10,13}-[a-zA-Z0-9]{24,32}").unwrap(), // Slack token
            ],
        );

        // JWT Secrets
        patterns.insert(
            SecretType::JwtSecret,
            vec![
                Regex::new(r"eyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+").unwrap(),
                Regex::new(r#"(?i)jwt[_-]?secret['\"]?\s*[:=]\s*['"]([^'"]{16,})['"]"#).unwrap(),
            ],
        );

        // Database Credentials
        patterns.insert(
            SecretType::DatabaseCredential,
            vec![
                Regex::new(r"(?i)postgres://[^:]+:[^@]+@").unwrap(),
                Regex::new(r"(?i)mysql://[^:]+:[^@]+@").unwrap(),
                Regex::new(r"(?i)mongodb(\+srv)?://[^:]+:[^@]+@").unwrap(),
            ],
        );

        // Webhook Secrets
        patterns.insert(
            SecretType::WebhookSecret,
            vec![
                Regex::new(r#"(?i)webhook[_-]?secret['\"]?\s*[:=]\s*['"]([^'"]{16,})['"]"#)
                    .unwrap(),
            ],
        );

        // Encryption Keys
        patterns.insert(
            SecretType::EncryptionKey,
            vec![
                Regex::new(r#"(?i)encrypt(ion)?[_-]?key['\"]?\s*[:=]\s*['"]([^'"]{16,})['"]"#)
                    .unwrap(),
                Regex::new(r#"(?i)aes[_-]?key['\"]?\s*[:=]\s*['"]([^'"]{16,})['"]"#).unwrap(),
            ],
        );

        let allowlist = vec![
            "example".to_string(),
            "test".to_string(),
            "mock".to_string(),
            "demo".to_string(),
            "sample".to_string(),
            "placeholder".to_string(),
            "your-api-key-here".to_string(),
            "xxx".to_string(),
        ];

        Self {
            patterns,
            entropy_threshold: 4.5,
            allowlist,
        }
    }

    fn calculate_confidence(&self, value: &str, secret_type: &SecretType) -> f64 {
        let mut confidence: f64 = 0.5; // Base confidence

        // Increase confidence based on string characteristics
        if value.len() > 30 {
            confidence += 0.1;
        }

        // Check for known patterns
        match secret_type {
            SecretType::ApiKey if value.starts_with("sk-") || value.starts_with("pk-") => {
                confidence += 0.3;
            }
            SecretType::AuthToken if value.starts_with("Bearer ") => {
                confidence += 0.2;
            }
            _ => {}
        }

        // Check entropy
        let entropy = self.calculate_entropy(value);
        if entropy > 4.0 {
            confidence += 0.1;
        }
        if entropy > 5.0 {
            confidence += 0.1;
        }

        // Decrease confidence if it matches allowlist patterns
        let lower_value = value.to_lowercase();
        for allowed in &self.allowlist {
            if lower_value.contains(allowed) {
                confidence -= 0.4;
            }
        }

        confidence.min(1.0).max(0.0)
    }

    fn calculate_entropy(&self, s: &str) -> f64 {
        if s.is_empty() {
            return 0.0;
        }

        let mut char_counts = HashMap::new();
        for c in s.chars() {
            *char_counts.entry(c).or_insert(0) += 1;
        }

        let length = s.len() as f64;
        char_counts
            .values()
            .map(|&count| {
                let probability = count as f64 / length;
                -probability * probability.log2()
            })
            .sum()
    }

    fn create_preview(&self, value: &str) -> String {
        let preview_length = 8.min(value.len());
        let visible = &value[..preview_length];
        let hidden_count = value.len() - preview_length;

        if hidden_count > 0 {
            format!("{}***({} chars hidden)", visible, hidden_count)
        } else {
            visible.to_string()
        }
    }

    fn is_in_allowlist(&self, value: &str) -> bool {
        let lower_value = value.to_lowercase();
        self.allowlist
            .iter()
            .any(|allowed| lower_value.contains(allowed))
    }
}

impl SecurityDetector for EnhancedSecretDetector {
    fn detect_vulnerabilities(&self, file: &File, path: &Path) -> Vec<SecurityVulnerability> {
        let mut visitor = SecretVisitor {
            vulnerabilities: Vec::new(),
            path: path.to_path_buf(),
            detector: self,
        };
        visitor.visit_file(file);
        visitor.vulnerabilities
    }

    fn detector_name(&self) -> &'static str {
        "EnhancedSecretDetector"
    }
}

struct SecretVisitor<'a> {
    vulnerabilities: Vec<SecurityVulnerability>,
    path: std::path::PathBuf,
    detector: &'a EnhancedSecretDetector,
}

impl<'a, 'ast> Visit<'ast> for SecretVisitor<'a> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if let Expr::Lit(ExprLit { lit, .. }) = expr {
            if let Lit::Str(lit_str) = lit {
                let value = lit_str.value();

                // Skip if in allowlist
                if self.detector.is_in_allowlist(&value) {
                    syn::visit::visit_expr(self, expr);
                    return;
                }

                // Check against known patterns
                for (secret_type, patterns) in &self.detector.patterns {
                    for pattern in patterns {
                        if pattern.is_match(&value) {
                            let confidence =
                                self.detector.calculate_confidence(&value, secret_type);

                            if confidence > 0.6 {
                                let entropy = self.detector.calculate_entropy(&value);
                                self.vulnerabilities
                                    .push(SecurityVulnerability::HardcodedSecret {
                                        secret_type: *secret_type,
                                        confidence,
                                        value_preview: self.detector.create_preview(&value),
                                        entropy,
                                        line: 0, // Would need span info for accurate line
                                        file: self.path.clone(),
                                    });
                                syn::visit::visit_expr(self, expr);
                                return;
                            }
                        }
                    }
                }

                // Entropy-based detection for unknown patterns
                let entropy = self.detector.calculate_entropy(&value);
                if entropy > self.detector.entropy_threshold
                    && value.len() > 20
                    && !value.contains(' ')
                    && value.chars().all(|c| c.is_ascii_graphic())
                {
                    self.vulnerabilities
                        .push(SecurityVulnerability::HardcodedSecret {
                            secret_type: SecretType::Unknown,
                            confidence: 0.6,
                            value_preview: self.detector.create_preview(&value),
                            entropy,
                            line: 0,
                            file: self.path.clone(),
                        });
                }
            }
        }
        syn::visit::visit_expr(self, expr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_calculation() {
        let detector = EnhancedSecretDetector::new();

        // Low entropy (repeated chars)
        assert!(detector.calculate_entropy("aaaaaaa") < 2.0);

        // High entropy (random-looking)
        assert!(detector.calculate_entropy("a8B3x9Q2mN5pL7") > 3.5);

        // Very high entropy
        assert!(detector.calculate_entropy("sk-proj-1234567890abcdefghijklmnop") > 4.0);
    }

    #[test]
    fn test_confidence_calculation() {
        let detector = EnhancedSecretDetector::new();

        // High confidence for real-looking API key
        let confidence = detector
            .calculate_confidence("sk-live-1234567890abcdefghijklmnop", &SecretType::ApiKey);
        assert!(confidence > 0.7);

        // Low confidence for test key
        let confidence = detector.calculate_confidence("test-api-key", &SecretType::ApiKey);
        assert!(confidence < 0.5);
    }

    #[test]
    fn test_preview_creation() {
        let detector = EnhancedSecretDetector::new();

        assert_eq!(detector.create_preview("short"), "short");
        assert_eq!(
            detector.create_preview("verylongsecretkey123456"),
            "verylong***(15 chars hidden)"
        );
    }
}
// debtmap:ignore-end
