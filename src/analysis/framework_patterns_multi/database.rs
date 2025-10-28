//! Database Framework Pattern Detection

use super::detector::{FileContext, FunctionAst};

/// Detect Diesel query (Rust)
pub fn is_diesel_query(function: &FunctionAst, file_context: &FileContext) -> bool {
    let has_diesel_import = file_context.imports.iter().any(|i| i.contains("diesel"));

    let has_diesel_calls = function.calls.iter().any(|call| {
        call.name.contains("execute")
            || call.name.contains("load")
            || call.name.contains("get_result")
            || call.name.contains("get_results")
    });

    let has_diesel_derives = function
        .derives
        .iter()
        .any(|d| d == "Queryable" || d == "Insertable");

    has_diesel_import && (has_diesel_calls || has_diesel_derives)
}

/// Detect SQLAlchemy model (Python)
pub fn is_sqlalchemy_model(function: &FunctionAst, file_context: &FileContext) -> bool {
    let has_sqlalchemy_import = file_context
        .imports
        .iter()
        .any(|i| i.contains("sqlalchemy") || i.contains("SQLAlchemy"));

    let has_sqlalchemy_decorators = function.decorators.iter().any(|d| {
        d.name.contains("validates")
            || d.name.contains("hybrid_property")
            || d.name.contains("event.listens_for")
    });

    has_sqlalchemy_import && has_sqlalchemy_decorators
}

#[cfg(test)]
mod tests {
    use super::super::detector::{Decorator, FileContext, FunctionAst, FunctionCall};
    use super::super::patterns::Language;
    use super::*;

    #[test]
    fn test_diesel_query_detection() {
        let mut function = FunctionAst::new("get_users".to_string());
        function.calls.push(FunctionCall {
            name: "load".to_string(),
        });

        let mut file_context = FileContext::new(Language::Rust, "models.rs".into());
        file_context.add_import("use diesel::prelude::*;".to_string());

        assert!(is_diesel_query(&function, &file_context));
    }

    #[test]
    fn test_sqlalchemy_model_detection() {
        let mut function = FunctionAst::new("validate_email".to_string());
        function.decorators.push(Decorator {
            name: "@validates('email')".to_string(),
        });

        let mut file_context = FileContext::new(Language::Python, "models.py".into());
        file_context.add_import("from sqlalchemy import validates".to_string());

        assert!(is_sqlalchemy_model(&function, &file_context));
    }
}
