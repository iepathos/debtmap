/// Example function visitor for demonstrating tech debt fixes
use syn::{Block, Stmt};

pub struct FunctionVisitor;

impl FunctionVisitor {
    /// Visit a function body to analyze its statements
    /// This function would have score 10.0 due to zero test coverage
    pub fn visit_body(&self, body: &Block) -> Vec<String> {
        let mut results = Vec::new();

        // Simple implementation that would need tests
        for stmt in &body.stmts {
            match stmt {
                Stmt::Local(_) => {
                    results.push("local_variable".to_string());
                }
                Stmt::Expr(_, _) | Stmt::Macro(_) => {
                    results.push("expression".to_string());
                }
                Stmt::Item(_) => {
                    results.push("item".to_string());
                }
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, Block};

    #[test]
    fn test_visit_body_with_local_variable() {
        let visitor = FunctionVisitor;
        let body: Block = parse_quote! {{
            let x = 42;
        }};

        let results = visitor.visit_body(&body);
        assert_eq!(results, vec!["local_variable"]);
    }

    #[test]
    fn test_visit_body_with_expression() {
        let visitor = FunctionVisitor;
        let body: Block = parse_quote! {{
            println!("hello");
        }};

        let results = visitor.visit_body(&body);
        assert_eq!(results, vec!["expression"]);
    }

    #[test]
    fn test_visit_body_with_item() {
        let visitor = FunctionVisitor;
        let body: Block = parse_quote! {{
            fn inner() {}
        }};

        let results = visitor.visit_body(&body);
        assert_eq!(results, vec!["item"]);
    }

    #[test]
    fn test_visit_body_with_multiple_statements() {
        let visitor = FunctionVisitor;
        let body: Block = parse_quote! {{
            let x = 1;
            println!("test");
            fn helper() {}
        }};

        let results = visitor.visit_body(&body);
        assert_eq!(results, vec!["local_variable", "expression", "item"]);
    }

    #[test]
    fn test_visit_body_empty() {
        let visitor = FunctionVisitor;
        let body: Block = parse_quote! {{}};

        let results = visitor.visit_body(&body);
        assert!(results.is_empty());
    }
}
