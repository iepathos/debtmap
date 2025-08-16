use syn::visit::Visit;

#[test]
fn test_how_vec_macro_is_parsed() {
    let code = r#"
fn test() {
    let x = vec![MyStruct {
        field: some_function(),
    }];
}

fn some_function() -> i32 {
    42
}
"#;

    let syntax = syn::parse_file(code).expect("Failed to parse");

    struct DebugVisitor;

    impl<'ast> Visit<'ast> for DebugVisitor {
        fn visit_expr(&mut self, expr: &'ast syn::Expr) {
            match expr {
                syn::Expr::Macro(m) => {
                    println!("Found macro: {}", quote::quote!(#m));
                }
                syn::Expr::Struct(_s) => {
                    println!("Found struct literal");
                }
                syn::Expr::Call(c) => {
                    if let syn::Expr::Path(p) = &*c.func {
                        let name = p
                            .path
                            .segments
                            .last()
                            .map(|s| s.ident.to_string())
                            .unwrap_or_default();
                        println!("Found function call: {}", name);
                    }
                }
                _ => {}
            }
            syn::visit::visit_expr(self, expr);
        }
    }

    let mut visitor = DebugVisitor;
    visitor.visit_file(&syntax);
}
