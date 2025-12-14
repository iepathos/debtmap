//! Module Facade Detection (Spec 170)
//!
//! Detects whether a Rust file is a module facade that primarily organizes
//! submodules through #\[path\] declarations and re-exports, with minimal
//! implementation code.

use syn::spanned::Spanned;

use super::types::{ModuleFacadeInfo, OrganizationQuality, PathDeclaration};

/// Detect if a Rust AST represents a module facade
///
/// A module facade is a file that primarily organizes submodules through
/// #\[path\] declarations and re-exports, with minimal implementation code.
pub fn detect_module_facade(ast: &syn::File) -> ModuleFacadeInfo {
    let mut path_declarations = Vec::new();
    let mut inline_modules = 0;
    let mut impl_lines = 0;
    let mut fn_lines = 0;
    let mut total_lines = 0;

    for item in &ast.items {
        let span = item.span();
        total_lines = total_lines.max(span.end().line);

        match item {
            syn::Item::Mod(module) => {
                if let Some(path) = extract_path_attribute(module) {
                    path_declarations.push(PathDeclaration {
                        module_name: module.ident.to_string(),
                        file_path: path,
                        line: span.start().line,
                    });
                } else if module.content.is_some() {
                    inline_modules += 1;
                }
            }
            syn::Item::Impl(_impl_block) => {
                impl_lines += span.end().line.saturating_sub(span.start().line);
            }
            syn::Item::Fn(_func) => {
                fn_lines += span.end().line.saturating_sub(span.start().line);
            }
            _ => {}
        }
    }

    let submodule_count = path_declarations.len() + inline_modules;
    let implementation_lines = impl_lines + fn_lines;

    // Calculate facade score
    let facade_score = calculate_facade_score(total_lines, implementation_lines, submodule_count);

    // Classify organization quality
    let organization_quality = classify_organization_quality(submodule_count, facade_score);

    ModuleFacadeInfo {
        is_facade: submodule_count >= 3 && facade_score >= 0.5,
        submodule_count,
        path_declarations,
        facade_score,
        organization_quality,
    }
}

/// Calculate facade score based on declaration vs implementation ratio
fn calculate_facade_score(
    total_lines: usize,
    implementation_lines: usize,
    submodule_count: usize,
) -> f64 {
    let declaration_ratio = if total_lines > 0 {
        (total_lines.saturating_sub(implementation_lines)) as f64 / total_lines as f64
    } else {
        0.0
    };

    let submodule_factor = (submodule_count as f64 / 5.0).min(1.0);
    (declaration_ratio * 0.7 + submodule_factor * 0.3).clamp(0.0, 1.0)
}

/// Extract #[path = "..."] attribute from module declaration
///
/// Parses module attributes to find path declarations that indicate
/// external submodule files.
pub fn extract_path_attribute(module: &syn::ItemMod) -> Option<String> {
    for attr in &module.attrs {
        if attr.path().is_ident("path") {
            if let syn::Meta::NameValue(meta) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &meta.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        return Some(lit_str.value());
                    }
                }
            }
        }
    }
    None
}

/// Classify organization quality based on submodule count and facade score
///
/// Returns classification from Excellent to Monolithic based on the
/// degree of modular organization in the file.
pub fn classify_organization_quality(
    submodule_count: usize,
    facade_score: f64,
) -> OrganizationQuality {
    match (submodule_count, facade_score) {
        (0..=2, _) => OrganizationQuality::Monolithic,
        (n, s) if n >= 10 && s >= 0.8 => OrganizationQuality::Excellent,
        (n, s) if n >= 5 && s >= 0.6 => OrganizationQuality::Good,
        (n, s) if n >= 3 && s >= 0.5 => OrganizationQuality::Poor,
        _ => OrganizationQuality::Monolithic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_pure_facade_with_path_attributes() {
        let code = r#"
            #[path = "executor/builder.rs"]
            mod builder;
            #[path = "executor/commands.rs"]
            pub mod commands;
            #[path = "executor/pure.rs"]
            pub(crate) mod pure;

            pub use builder::Builder;
            pub use commands::execute;
        "#;

        let ast = syn::parse_file(code).unwrap();
        let facade_info = detect_module_facade(&ast);

        assert!(facade_info.is_facade);
        assert_eq!(facade_info.submodule_count, 3);
        assert_eq!(facade_info.path_declarations.len(), 3);
        assert!(facade_info.facade_score > 0.8);
        assert_eq!(facade_info.organization_quality, OrganizationQuality::Poor);
    }

    #[test]
    fn test_detect_monolithic_file_no_modules() {
        let code = r#"
            struct Foo { x: u32 }

            impl Foo {
                fn method1(&self) -> u32 { self.x }
                fn method2(&self) -> u32 { self.x * 2 }
                fn method3(&self) -> u32 { self.x * 3 }
            }

            fn standalone1() { println!("test"); }
            fn standalone2() { println!("test"); }
            fn standalone3() { println!("test"); }
        "#;

        let ast = syn::parse_file(code).unwrap();
        let facade_info = detect_module_facade(&ast);

        assert!(!facade_info.is_facade);
        assert_eq!(facade_info.submodule_count, 0);
        assert!(
            facade_info.facade_score < 0.5,
            "Expected facade_score < 0.5, got {}",
            facade_info.facade_score
        );
        assert_eq!(
            facade_info.organization_quality,
            OrganizationQuality::Monolithic
        );
    }

    #[test]
    fn test_detect_excellent_facade() {
        let code = r#"
            #[path = "executor/builder.rs"]
            mod builder;
            #[path = "executor/commands.rs"]
            mod commands;
            #[path = "executor/pure.rs"]
            mod pure;
            #[path = "executor/data.rs"]
            mod data;
            #[path = "executor/types.rs"]
            mod types;
            #[path = "executor/errors.rs"]
            mod errors;
            #[path = "executor/config.rs"]
            mod config;
            #[path = "executor/validation.rs"]
            mod validation;
            #[path = "executor/helpers.rs"]
            mod helpers;
            #[path = "executor/hooks.rs"]
            mod hooks;

            pub use builder::Builder;
            pub use commands::*;
            pub use types::{Type1, Type2};
        "#;

        let ast = syn::parse_file(code).unwrap();
        let facade_info = detect_module_facade(&ast);

        assert!(facade_info.is_facade);
        assert_eq!(facade_info.submodule_count, 10);
        assert!(facade_info.facade_score > 0.85);
        assert_eq!(
            facade_info.organization_quality,
            OrganizationQuality::Excellent
        );
    }

    #[test]
    fn test_extract_path_attribute() {
        let code = r#"
            #[path = "foo/bar.rs"]
            mod test_module;
        "#;

        let ast = syn::parse_file(code).unwrap();
        if let syn::Item::Mod(module) = &ast.items[0] {
            let path = extract_path_attribute(module);
            assert_eq!(path, Some("foo/bar.rs".to_string()));
        } else {
            panic!("Expected module item");
        }
    }

    #[test]
    fn test_classify_organization_quality_thresholds() {
        assert_eq!(
            classify_organization_quality(13, 0.92),
            OrganizationQuality::Excellent
        );

        assert_eq!(
            classify_organization_quality(6, 0.65),
            OrganizationQuality::Good
        );

        assert_eq!(
            classify_organization_quality(3, 0.55),
            OrganizationQuality::Poor
        );

        assert_eq!(
            classify_organization_quality(1, 0.2),
            OrganizationQuality::Monolithic
        );

        assert_eq!(
            classify_organization_quality(5, 0.45),
            OrganizationQuality::Monolithic
        );
    }

    #[test]
    fn test_mixed_inline_and_path_modules() {
        let code = r#"
            #[path = "external.rs"]
            mod external;

            mod inline {
                pub fn helper() {}
            }

            #[path = "another.rs"]
            mod another;
        "#;

        let ast = syn::parse_file(code).unwrap();
        let facade_info = detect_module_facade(&ast);

        assert_eq!(facade_info.submodule_count, 3); // 2 path + 1 inline
        assert_eq!(facade_info.path_declarations.len(), 2);
        assert!(facade_info.is_facade);
    }
}
