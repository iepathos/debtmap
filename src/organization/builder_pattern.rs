/// Builder Pattern Detection
///
/// Detects builder patterns where many fluent setter methods are intentionally
/// present for configuration purposes. Prevents false positives where god object
/// detection flags builders for having many methods.
use std::collections::HashMap;
use syn::{spanned::Spanned, visit::Visit, File, ImplItem, Item, ItemImpl, ReturnType, Type};

/// Information about a method signature
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub return_type: MethodReturnType,
    pub param_count: usize,
    pub line_count: usize,
    pub is_mutable_self: bool,
    pub is_consuming_self: bool,
    pub start_line: usize,
    pub end_line: usize,
}

/// Categorized return types for methods
#[derive(Debug, Clone, PartialEq)]
pub enum MethodReturnType {
    MutableSelfRef,       // &mut Self
    SelfValue,            // Self
    BuildProduct(String), // Named type (likely the configured type)
    Other(String),        // Other return types
    Unit,                 // ()
}

/// Detected builder pattern
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BuilderPattern {
    /// Name of the builder struct
    pub builder_struct: String,

    /// Number of fluent setter methods
    pub setter_count: usize,

    /// Total methods in builder impl
    pub total_method_count: usize,

    /// Ratio of setters to total methods (0.0 - 1.0)
    pub setter_ratio: f64,

    /// Average lines per setter
    pub avg_setter_size: f64,

    /// Standard deviation of setter sizes
    pub setter_size_stddev: f64,

    /// Names of build methods (build, finish, etc.)
    pub build_methods: Vec<String>,

    /// Type produced by builder (if detected)
    pub product_type: Option<String>,

    /// Whether builder uses separate config struct
    pub has_config_struct: bool,

    /// Total lines in file containing builder
    pub total_file_lines: usize,

    /// Lines in non-setter implementation code
    pub implementation_lines: usize,
}

/// Builder pattern detector configuration
pub struct BuilderPatternDetector {
    pub min_setter_count: usize,
    pub min_setter_ratio: f64,
    pub max_avg_setter_size: usize,
}

impl Default for BuilderPatternDetector {
    fn default() -> Self {
        Self {
            min_setter_count: 10,
            min_setter_ratio: 0.50,
            max_avg_setter_size: 10,
        }
    }
}

impl BuilderPatternDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect builder pattern in a Rust file
    pub fn detect(&self, file: &File, file_content: &str) -> Option<BuilderPattern> {
        let method_infos = extract_method_infos(file, file_content);
        let total_lines = file_content.lines().count();

        // Group methods by struct/impl block
        let mut methods_by_type: HashMap<String, Vec<&MethodInfo>> = HashMap::new();
        for method in &method_infos {
            // Infer struct name from method context (we'll need to enhance this)
            // For now, we'll group all methods together
            methods_by_type
                .entry("Builder".to_string())
                .or_default()
                .push(method);
        }

        // Find struct with builder characteristics
        let (builder_struct, builder_methods) = methods_by_type
            .iter()
            .max_by_key(|(_, methods)| methods.len())?;

        let total_method_count = builder_methods.len();

        // Count fluent setter methods
        let setter_methods: Vec<&&MethodInfo> = builder_methods
            .iter()
            .filter(|m| {
                matches!(
                    m.return_type,
                    MethodReturnType::MutableSelfRef | MethodReturnType::SelfValue
                )
            })
            .collect();

        let setter_count = setter_methods.len();

        // Check minimum setter count
        if setter_count < self.min_setter_count {
            return None;
        }

        // Calculate setter ratio
        let setter_ratio = setter_count as f64 / total_method_count as f64;

        // Check setter ratio threshold
        if setter_ratio < self.min_setter_ratio {
            return None;
        }

        // Calculate average setter size
        let total_setter_lines: usize = setter_methods.iter().map(|m| m.line_count).sum();
        let avg_setter_size = total_setter_lines as f64 / setter_count as f64;

        // Check average size threshold
        if avg_setter_size >= self.max_avg_setter_size as f64 {
            return None;
        }

        // Calculate standard deviation
        let variance: f64 = setter_methods
            .iter()
            .map(|m| {
                let diff = m.line_count as f64 - avg_setter_size;
                diff * diff
            })
            .sum::<f64>()
            / setter_count as f64;
        let setter_size_stddev = variance.sqrt();

        // Find build methods
        let build_methods: Vec<String> = builder_methods
            .iter()
            .filter(|m| {
                let name_lower = m.name.to_lowercase();
                name_lower == "build" || name_lower == "finish" || name_lower.contains("build")
            })
            .map(|m| m.name.clone())
            .collect();

        // Must have at least one build method
        if build_methods.is_empty() {
            return None;
        }

        // Try to determine product type from build methods
        let product_type = builder_methods
            .iter()
            .find(|m| m.name == "build" || m.name == "finish")
            .and_then(|m| match &m.return_type {
                MethodReturnType::BuildProduct(type_name) => Some(type_name.clone()),
                _ => None,
            });

        // Check if there's a separate config struct (heuristic)
        let has_config_struct = file_content.contains("struct")
            && (file_content.contains("Config") || file_content.contains("Settings"));

        // Calculate implementation lines (non-setter code)
        let implementation_lines = total_lines - total_setter_lines;

        Some(BuilderPattern {
            builder_struct: builder_struct.clone(),
            setter_count,
            total_method_count,
            setter_ratio,
            avg_setter_size,
            setter_size_stddev,
            build_methods,
            product_type,
            has_config_struct,
            total_file_lines: total_lines,
            implementation_lines,
        })
    }

    /// Calculate confidence score (0.0 to 1.0)
    pub fn confidence(&self, pattern: &BuilderPattern) -> f64 {
        let mut confidence = 0.0;

        // Base confidence from setter ratio - most important signal
        if pattern.setter_ratio > 0.85 {
            confidence += 0.35;
        } else if pattern.setter_ratio > 0.70 {
            confidence += 0.25;
        } else if pattern.setter_ratio > 0.55 {
            confidence += 0.15;
        } else {
            confidence += 0.05; // Very low confidence if < 55% setters
        }

        // Boost from setter count - but not too much weight
        confidence += (pattern.setter_count as f64 / 50.0).min(0.15);

        // Boost from small setter size - indicates simple setters
        if pattern.avg_setter_size < 5.0 {
            confidence += 0.18;
        } else if pattern.avg_setter_size < 7.0 {
            confidence += 0.10;
        } else if pattern.avg_setter_size < 10.0 {
            confidence += 0.03;
        }
        // No boost for larger setters - reduces confidence

        // Boost from presence of build method - critical signal
        // But only give full boost if other signals are strong
        if !pattern.build_methods.is_empty() {
            if pattern.setter_ratio > 0.70 && pattern.avg_setter_size < 7.0 {
                confidence += 0.18; // Strong builder signals
            } else if pattern.setter_ratio > 0.55 {
                confidence += 0.10; // Moderate builder signals
            } else {
                confidence += 0.05; // Weak builder signals
            }
        }

        // Boost from consistent setter sizes (low stddev)
        if pattern.setter_size_stddev < 2.0 {
            confidence += 0.08;
        } else if pattern.setter_size_stddev < 3.5 {
            confidence += 0.04;
        }
        // High stddev reduces builder confidence

        // Boost from builder naming patterns
        if pattern.builder_struct.contains("Builder")
            || pattern.builder_struct.contains("Config")
            || pattern.builder_struct.contains("Options")
        {
            confidence += 0.07;
        }

        confidence.min(1.0)
    }
}

/// Extract method information from AST
fn extract_method_infos(file: &File, file_content: &str) -> Vec<MethodInfo> {
    let mut visitor = MethodVisitor {
        methods: Vec::new(),
        file_content,
    };

    visitor.visit_file(file);
    visitor.methods
}

/// AST visitor for extracting method signatures
struct MethodVisitor<'a> {
    methods: Vec<MethodInfo>,
    file_content: &'a str,
}

impl<'a, 'ast> Visit<'ast> for MethodVisitor<'a> {
    fn visit_item(&mut self, item: &'ast Item) {
        if let Item::Impl(item_impl) = item {
            for impl_item in &item_impl.items {
                if let ImplItem::Fn(method) = impl_item {
                    if let Some(method_info) =
                        extract_method_info(method, item_impl, self.file_content)
                    {
                        self.methods.push(method_info);
                    }
                }
            }
        }

        syn::visit::visit_item(self, item);
    }
}

/// Extract method information from a method in an impl block
fn extract_method_info(
    method: &syn::ImplItemFn,
    impl_block: &ItemImpl,
    file_content: &str,
) -> Option<MethodInfo> {
    let name = method.sig.ident.to_string();

    // Determine return type
    let return_type = classify_return_type(&method.sig.output, impl_block);

    // Count parameters
    let param_count = method.sig.inputs.len();

    // Check for mutable self and consuming self
    let (is_mutable_self, is_consuming_self) = check_self_params(&method.sig.inputs);

    // Calculate line count
    let span = method.span();
    let start_line = span.start().line;
    let end_line = span.end().line;
    let line_count = count_lines_in_span(file_content, start_line, end_line);

    Some(MethodInfo {
        name,
        return_type,
        param_count,
        line_count,
        is_mutable_self,
        is_consuming_self,
        start_line,
        end_line,
    })
}

/// Classify the return type of a method
fn classify_return_type(output: &ReturnType, impl_block: &ItemImpl) -> MethodReturnType {
    match output {
        ReturnType::Default => MethodReturnType::Unit,
        ReturnType::Type(_, ty) => classify_type(ty, impl_block),
    }
}

/// Classify a type to determine if it's Self, &mut Self, or other
fn classify_type(ty: &Type, _impl_block: &ItemImpl) -> MethodReturnType {
    match ty {
        Type::Reference(type_ref) => {
            if type_ref.mutability.is_some() {
                if let Type::Path(type_path) = &*type_ref.elem {
                    if type_path.path.is_ident("Self") {
                        return MethodReturnType::MutableSelfRef;
                    }
                }
            }
            MethodReturnType::Other(quote::quote!(#ty).to_string())
        }
        Type::Path(type_path) => {
            if type_path.path.is_ident("Self") {
                MethodReturnType::SelfValue
            } else {
                // Check if this might be the build product
                let type_name = type_path
                    .path
                    .segments
                    .last()
                    .map(|seg| seg.ident.to_string())
                    .unwrap_or_default();

                // If it's not Self and not a common generic, it might be the build product
                if !type_name.is_empty()
                    && !matches!(
                        type_name.as_str(),
                        "Option" | "Result" | "Vec" | "Box" | "Rc" | "Arc"
                    )
                {
                    MethodReturnType::BuildProduct(type_name)
                } else {
                    MethodReturnType::Other(type_name)
                }
            }
        }
        _ => MethodReturnType::Other(quote::quote!(#ty).to_string()),
    }
}

/// Check if method has mutable self or consuming self parameter
fn check_self_params(
    inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
) -> (bool, bool) {
    for input in inputs {
        if let syn::FnArg::Receiver(receiver) = input {
            let is_mutable = receiver.mutability.is_some();
            let is_consuming = receiver.reference.is_none();
            return (is_mutable, is_consuming);
        }
    }
    (false, false)
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

/// Adjust god object score based on builder pattern
pub fn adjust_builder_score(base_score: f64, pattern: &BuilderPattern) -> f64 {
    // Size-based factor - larger files still warrant attention even if they're builders
    let size_factor = if pattern.total_file_lines > 5000 {
        2.5 // Very large file - definitely needs review
    } else if pattern.total_file_lines > 3000 {
        2.0 // Large file - warrants attention
    } else if pattern.total_file_lines > 1500 {
        1.0 // Moderate size - acceptable
    } else {
        0.5 // Small builder - reduce score significantly
    };

    // Focus factor based on setter ratio - high setter ratio means it's doing its job
    // But large files still need review even if focused
    let focus_factor = if pattern.setter_ratio > 0.80 {
        if pattern.total_file_lines > 3000 {
            0.6 // Large file with many setters still warrants review
        } else {
            0.4 // Small file with many setters - definitely a builder
        }
    } else if pattern.setter_ratio > 0.60 {
        0.7 // Focused - majority setters
    } else if pattern.setter_ratio > 0.50 {
        0.85 // Some setters - borderline builder
    } else {
        1.0 // Mixed - might actually be a god object
    };

    base_score * size_factor * focus_factor
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_rust_code(code: &str) -> File {
        syn::parse_str(code).expect("Failed to parse Rust code")
    }

    #[test]
    fn test_detect_builder_pattern_basic() {
        let code = r#"
            struct ConfigBuilder {
                timeout: Option<u64>,
                retries: Option<u32>,
            }

            impl ConfigBuilder {
                pub fn new() -> Self {
                    Self { timeout: None, retries: None }
                }

                pub fn timeout(&mut self, value: u64) -> &mut Self {
                    self.timeout = Some(value);
                    self
                }

                pub fn retries(&mut self, value: u32) -> &mut Self {
                    self.retries = Some(value);
                    self
                }

                pub fn max_connections(&mut self, value: u32) -> &mut Self {
                    self
                }

                pub fn buffer_size(&mut self, value: usize) -> &mut Self {
                    self
                }

                pub fn enable_logging(&mut self) -> &mut Self {
                    self
                }

                pub fn enable_metrics(&mut self) -> &mut Self {
                    self
                }

                pub fn set_host(&mut self, host: String) -> &mut Self {
                    self
                }

                pub fn set_port(&mut self, port: u16) -> &mut Self {
                    self
                }

                pub fn set_path(&mut self, path: String) -> &mut Self {
                    self
                }

                pub fn set_headers(&mut self, headers: Vec<String>) -> &mut Self {
                    self
                }

                pub fn build(self) -> Config {
                    Config {}
                }
            }

            struct Config {}
        "#;

        let file = parse_rust_code(code);
        let detector = BuilderPatternDetector::default();

        let pattern = detector.detect(&file, code);
        assert!(pattern.is_some(), "Should detect builder pattern");

        let pattern = pattern.unwrap();
        assert!(pattern.setter_count >= 10);
        assert!(pattern.setter_ratio > 0.50);
        assert!(!pattern.build_methods.is_empty());
    }

    #[test]
    fn test_builder_score_focuses_on_size_not_setters() {
        let small_builder = BuilderPattern {
            builder_struct: "SmallBuilder".into(),
            setter_count: 30,
            total_method_count: 35,
            setter_ratio: 0.86,
            avg_setter_size: 3.0,
            setter_size_stddev: 1.2,
            build_methods: vec!["build".into()],
            product_type: Some("Config".into()),
            has_config_struct: true,
            total_file_lines: 500,
            implementation_lines: 50,
        };

        let large_builder = BuilderPattern {
            builder_struct: "LargeBuilder".into(),
            total_file_lines: 4000,
            ..small_builder.clone()
        };

        let base_score = 1000.0;
        let small_adjusted = adjust_builder_score(base_score, &small_builder);
        let large_adjusted = adjust_builder_score(base_score, &large_builder);

        // Small builder with 30 setters gets score REDUCTION
        assert!(small_adjusted < base_score);

        // Large builder with same 30 setters gets score INCREASE
        assert!(large_adjusted >= base_score);
    }

    #[test]
    fn test_not_builder_low_setter_ratio() {
        let code = r#"
            struct NotBuilder {}

            impl NotBuilder {
                pub fn complex_method1(&self) { }
                pub fn complex_method2(&self) { }
                pub fn complex_method3(&self) { }
                pub fn complex_method4(&self) { }
                pub fn complex_method5(&self) { }
                pub fn setter(&mut self) -> &mut Self { self }
            }
        "#;

        let file = parse_rust_code(code);
        let detector = BuilderPatternDetector::default();

        let pattern = detector.detect(&file, code);
        assert!(
            pattern.is_none(),
            "Low setter ratio should not be detected as builder"
        );
    }

    #[test]
    fn test_fluent_setter_detection() {
        let code = r#"
            impl ConfigBuilder {
                pub fn timeout(&mut self, value: u64) -> &mut Self {
                    self.timeout = value;
                    self
                }

                pub fn retries(mut self, value: u32) -> Self {
                    self.retries = value;
                    self
                }

                pub fn build(self) -> Config {
                    Config {}
                }
            }
            struct Config {}
        "#;

        let file = parse_rust_code(code);
        let methods = extract_method_infos(&file, code);

        let setters: Vec<_> = methods
            .iter()
            .filter(|m| {
                matches!(
                    m.return_type,
                    MethodReturnType::MutableSelfRef | MethodReturnType::SelfValue
                )
            })
            .collect();

        assert_eq!(setters.len(), 2);
        assert!(methods.iter().any(|m| m.name == "build"));
    }

    #[test]
    fn test_confidence_calculation() {
        let detector = BuilderPatternDetector::default();

        let high_confidence = BuilderPattern {
            builder_struct: "HttpClientBuilder".into(),
            setter_count: 25,
            total_method_count: 27,
            setter_ratio: 0.93,
            avg_setter_size: 4.0,
            setter_size_stddev: 1.5,
            build_methods: vec!["build".into()],
            product_type: Some("HttpClient".into()),
            has_config_struct: false,
            total_file_lines: 800,
            implementation_lines: 100,
        };

        let confidence = detector.confidence(&high_confidence);
        assert!(
            confidence > 0.70,
            "High confidence builder should score > 0.70, got {}",
            confidence
        );

        let low_confidence = BuilderPattern {
            builder_struct: "MaybeBuilder".into(),
            setter_count: 10,
            total_method_count: 18,
            setter_ratio: 0.56,
            avg_setter_size: 9.0,
            setter_size_stddev: 5.0,
            build_methods: vec!["finish".into()],
            product_type: None,
            has_config_struct: false,
            total_file_lines: 600,
            implementation_lines: 400,
        };

        let confidence = detector.confidence(&low_confidence);
        assert!(
            confidence < 0.50,
            "Low confidence builder should score < 0.50, got {}",
            confidence
        );
    }
}
