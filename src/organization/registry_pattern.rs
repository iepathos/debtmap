/// Registry/Catalog Pattern Detection
///
/// Detects intentional registry/catalog patterns where many small trait implementations
/// are centralized in one file for discoverability and consistency.
use std::collections::HashMap;
use syn::{visit::Visit, File, Item, ItemImpl};

/// Information about a trait implementation
#[derive(Debug, Clone)]
pub struct TraitImplInfo {
    pub trait_name: String,
    pub type_name: String,
    pub line_count: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub is_unit_struct: bool,
}

/// Detected registry pattern
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RegistryPattern {
    /// Name of the trait being implemented repeatedly
    pub trait_name: String,

    /// Number of implementations found
    pub impl_count: usize,

    /// Average lines per implementation
    pub avg_impl_size: f64,

    /// Standard deviation of impl sizes
    pub impl_size_stddev: f64,

    /// Total lines in file
    pub total_lines: usize,

    /// Percentage of implementations that are unit structs
    pub unit_struct_ratio: f64,

    /// Whether file contains static registry array
    pub has_static_registry: bool,

    /// Coverage: trait impl lines / total lines
    pub trait_impl_coverage: f64,
}

/// Registry pattern detector configuration
pub struct RegistryPatternDetector {
    pub min_impl_count: usize,
    pub max_avg_impl_size: usize,
    pub min_coverage: f64,
}

impl Default for RegistryPatternDetector {
    fn default() -> Self {
        Self {
            min_impl_count: 20,
            max_avg_impl_size: 15,
            min_coverage: 0.80,
        }
    }
}

impl RegistryPatternDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect registry pattern in a Rust file
    pub fn detect(&self, file: &File, file_content: &str) -> Option<RegistryPattern> {
        let trait_impls = extract_trait_impls(file, file_content);
        let total_lines = file_content.lines().count();

        // Group impls by trait name
        let mut impls_by_trait: HashMap<String, Vec<&TraitImplInfo>> = HashMap::new();
        for impl_info in &trait_impls {
            impls_by_trait
                .entry(impl_info.trait_name.clone())
                .or_default()
                .push(impl_info);
        }

        // Find trait with most implementations
        let (dominant_trait, dominant_impls) =
            impls_by_trait.iter().max_by_key(|(_, impls)| impls.len())?;

        let impl_count = dominant_impls.len();

        // Check minimum implementation count threshold
        if impl_count < self.min_impl_count {
            return None;
        }

        // Calculate average implementation size
        let total_impl_lines: usize = dominant_impls.iter().map(|i| i.line_count).sum();
        let avg_impl_size = total_impl_lines as f64 / impl_count as f64;

        // Check average size threshold
        if avg_impl_size >= self.max_avg_impl_size as f64 {
            return None;
        }

        // Calculate standard deviation
        let variance: f64 = dominant_impls
            .iter()
            .map(|i| {
                let diff = i.line_count as f64 - avg_impl_size;
                diff * diff
            })
            .sum::<f64>()
            / impl_count as f64;
        let impl_size_stddev = variance.sqrt();

        // Calculate unit struct ratio
        let unit_struct_count = dominant_impls.iter().filter(|i| i.is_unit_struct).count();
        let unit_struct_ratio = unit_struct_count as f64 / impl_count as f64;

        // Calculate coverage
        let trait_impl_coverage = total_impl_lines as f64 / total_lines as f64;

        // Check coverage threshold
        if trait_impl_coverage < self.min_coverage {
            return None;
        }

        // Check for static registry array (simplified detection)
        let has_static_registry = file_content.contains("const") && file_content.contains("&[");

        Some(RegistryPattern {
            trait_name: dominant_trait.clone(),
            impl_count,
            avg_impl_size,
            impl_size_stddev,
            total_lines,
            unit_struct_ratio,
            has_static_registry,
            trait_impl_coverage,
        })
    }

    /// Calculate confidence score (0.0 to 1.0)
    pub fn confidence(&self, pattern: &RegistryPattern) -> f64 {
        let mut confidence = 0.0;

        // Base confidence from impl count
        confidence += (pattern.impl_count as f64 / 100.0).min(0.3);

        // Boost from small avg size
        if pattern.avg_impl_size < 10.0 {
            confidence += 0.3;
        } else if pattern.avg_impl_size < 15.0 {
            confidence += 0.2;
        }

        // Boost from high coverage
        if pattern.trait_impl_coverage > 0.9 {
            confidence += 0.2;
        } else if pattern.trait_impl_coverage > 0.8 {
            confidence += 0.1;
        }

        // Boost from unit structs
        if pattern.unit_struct_ratio > 0.8 {
            confidence += 0.15;
        } else if pattern.unit_struct_ratio > 0.5 {
            confidence += 0.1;
        }

        // Boost from static registry
        if pattern.has_static_registry {
            confidence += 0.05;
        }

        confidence.min(1.0)
    }
}

/// Extract trait implementation info from AST
fn extract_trait_impls(file: &File, file_content: &str) -> Vec<TraitImplInfo> {
    let mut visitor = TraitImplVisitor {
        impls: Vec::new(),
        unit_structs: std::collections::HashSet::new(),
        file_content,
    };

    visitor.visit_file(file);
    visitor.impls
}

/// AST visitor for extracting trait implementations
struct TraitImplVisitor<'a> {
    impls: Vec<TraitImplInfo>,
    unit_structs: std::collections::HashSet<String>,
    file_content: &'a str,
}

impl<'a, 'ast> Visit<'ast> for TraitImplVisitor<'a> {
    fn visit_item(&mut self, item: &'ast Item) {
        match item {
            Item::Struct(item_struct) => {
                // Check for unit struct (no fields)
                if matches!(item_struct.fields, syn::Fields::Unit) {
                    self.unit_structs.insert(item_struct.ident.to_string());
                }
            }
            Item::Impl(item_impl) => {
                if let Some(impl_info) = extract_impl_info(item_impl, self.file_content) {
                    // Check if implementing type is a unit struct
                    let is_unit_struct = self.unit_structs.contains(&impl_info.type_name);
                    self.impls.push(TraitImplInfo {
                        is_unit_struct,
                        ..impl_info
                    });
                }
            }
            _ => {}
        }

        syn::visit::visit_item(self, item);
    }
}

/// Extract implementation info from an impl block
fn extract_impl_info(item_impl: &ItemImpl, file_content: &str) -> Option<TraitImplInfo> {
    use syn::spanned::Spanned;

    // Only process trait implementations (not inherent impls)
    let (_, trait_path, _) = item_impl.trait_.as_ref()?;

    let trait_name = trait_path.segments.last()?.ident.to_string();

    let type_name = match &*item_impl.self_ty {
        syn::Type::Path(type_path) => type_path.path.segments.last()?.ident.to_string(),
        _ => return None,
    };

    let span = item_impl.span();
    let start_line = span.start().line;
    let end_line = span.end().line;

    // Count non-empty lines
    let line_count = count_lines_in_span(file_content, start_line, end_line);

    Some(TraitImplInfo {
        trait_name,
        type_name,
        line_count,
        start_line,
        end_line,
        is_unit_struct: false, // Will be updated by visitor
    })
}

/// Count non-empty, non-comment lines in a span
fn count_lines_in_span(content: &str, start_line: usize, end_line: usize) -> usize {
    content
        .lines()
        .enumerate()
        .skip(start_line.saturating_sub(1))
        .take(end_line.saturating_sub(start_line) + 1)
        .filter(|(_, line)| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//")
        })
        .count()
}

/// Adjust god object score based on registry pattern
pub fn adjust_registry_score(base_score: f64, pattern: &RegistryPattern) -> f64 {
    let reduction_factor = if pattern.avg_impl_size < 10.0 {
        0.2 // 80% reduction for very small impls
    } else if pattern.avg_impl_size < 15.0 {
        0.3 // 70% reduction for small impls
    } else {
        0.5 // 50% reduction for moderate impls
    };

    base_score * reduction_factor
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_rust_code(code: &str) -> File {
        syn::parse_str(code).expect("Failed to parse Rust code")
    }

    #[test]
    fn test_detect_registry_pattern_basic() {
        let code = r#"
            struct Flag1;
            struct Flag2;
            struct Flag3;

            trait Flag {
                fn name(&self) -> &str;
            }

            impl Flag for Flag1 { fn name(&self) -> &str { "flag1" } }
            impl Flag for Flag2 { fn name(&self) -> &str { "flag2" } }
            impl Flag for Flag3 { fn name(&self) -> &str { "flag3" } }
        "#;

        let file = parse_rust_code(code);
        let detector = RegistryPatternDetector {
            min_impl_count: 3,
            max_avg_impl_size: 15,
            min_coverage: 0.1,
        };

        let pattern = detector.detect(&file, code);
        // With only 3 impls, this shouldn't trigger with default thresholds
        // but with adjusted thresholds it should
        assert!(pattern.is_some());

        let pattern = pattern.unwrap();
        assert_eq!(pattern.trait_name, "Flag");
        assert_eq!(pattern.impl_count, 3);
    }

    #[test]
    fn test_registry_score_reduction() {
        let pattern = RegistryPattern {
            trait_name: "Flag".into(),
            impl_count: 150,
            avg_impl_size: 8.0,
            total_lines: 7775,
            unit_struct_ratio: 0.95,
            has_static_registry: true,
            trait_impl_coverage: 0.90,
            impl_size_stddev: 2.5,
        };

        let base_score = 1000.0;
        let adjusted = adjust_registry_score(base_score, &pattern);

        // 80% reduction for avg_impl_size < 10
        assert!((adjusted - 200.0).abs() < 1.0);
    }

    #[test]
    fn test_not_registry_large_impls() {
        // File with few large implementations should not be registry
        let code = r#"
            trait Processor {
                fn process(&self, data: &str) -> String;
                fn validate(&self, data: &str) -> bool;
                fn transform(&self, data: &str) -> String;
            }

            impl Processor for TypeA {
                fn process(&self, data: &str) -> String {
                    // Many lines of complex logic
                    let mut result = String::new();
                    for line in data.lines() {
                        result.push_str(&line.to_uppercase());
                        result.push('\n');
                    }
                    result
                }
                fn validate(&self, data: &str) -> bool { true }
                fn transform(&self, data: &str) -> String { data.to_string() }
            }
        "#;

        let file = parse_rust_code(code);
        let detector = RegistryPatternDetector::default();

        let pattern = detector.detect(&file, code);
        assert!(
            pattern.is_none(),
            "Large implementations should not be registry"
        );
    }

    #[test]
    fn test_confidence_calculation() {
        let detector = RegistryPatternDetector::default();

        let high_confidence_pattern = RegistryPattern {
            trait_name: "Flag".into(),
            impl_count: 100,
            avg_impl_size: 8.0,
            total_lines: 1000,
            unit_struct_ratio: 0.9,
            has_static_registry: true,
            trait_impl_coverage: 0.95,
            impl_size_stddev: 2.0,
        };

        let confidence = detector.confidence(&high_confidence_pattern);
        assert!(
            confidence > 0.8,
            "High confidence pattern should score > 0.8"
        );

        let low_confidence_pattern = RegistryPattern {
            trait_name: "Trait".into(),
            impl_count: 20,
            avg_impl_size: 14.0,
            total_lines: 500,
            unit_struct_ratio: 0.3,
            has_static_registry: false,
            trait_impl_coverage: 0.80,
            impl_size_stddev: 5.0,
        };

        let confidence = detector.confidence(&low_confidence_pattern);
        assert!(
            confidence < 0.6,
            "Low confidence pattern should score < 0.6"
        );
    }

    #[test]
    fn test_unit_struct_detection() {
        let code = r#"
            struct UnitStruct;
            struct RegularStruct { field: i32 }

            trait Trait {
                fn method(&self);
            }

            impl Trait for UnitStruct {
                fn method(&self) {}
            }

            impl Trait for RegularStruct {
                fn method(&self) {}
            }
        "#;

        let file = parse_rust_code(code);
        let impls = extract_trait_impls(&file, code);

        let unit_impl = impls.iter().find(|i| i.type_name == "UnitStruct");
        let regular_impl = impls.iter().find(|i| i.type_name == "RegularStruct");

        assert!(unit_impl.is_some());
        assert!(unit_impl.unwrap().is_unit_struct);
        assert!(regular_impl.is_some());
        assert!(!regular_impl.unwrap().is_unit_struct);
    }
}
