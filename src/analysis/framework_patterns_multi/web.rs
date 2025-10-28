//! Web Framework Pattern Detection

use super::detector::{FileContext, FunctionAst};

/// Detect Axum web handler (Rust)
pub fn is_axum_handler(function: &FunctionAst, file_context: &FileContext) -> bool {
    let has_axum_imports = file_context.imports.iter().any(|i| i.contains("axum"));

    let has_axum_types = function.parameters.iter().any(|p| {
        p.type_annotation.contains("axum::")
            || p.type_annotation.contains("Path<")
            || p.type_annotation.contains("Query<")
            || p.type_annotation.contains("Json<")
    }) || function
        .return_type
        .as_ref()
        .map(|rt| rt.contains("axum::") || rt.contains("Response"))
        .unwrap_or(false);

    let is_async = function.is_async;

    has_axum_imports && has_axum_types && is_async
}

/// Detect Express route handler (JavaScript)
pub fn is_express_handler(function: &FunctionAst, file_context: &FileContext) -> bool {
    let has_express_import = file_context.imports.iter().any(|i| i.contains("express"));

    let has_req_res_params = function.parameters.len() >= 2
        && (function.parameters[0].name == "req" || function.parameters[0].name == "request")
        && (function.parameters[1].name == "res" || function.parameters[1].name == "response");

    has_express_import && has_req_res_params
}

/// Detect React component (JavaScript/TypeScript)
pub fn is_react_component(function: &FunctionAst, file_context: &FileContext) -> bool {
    let has_react_import = file_context
        .imports
        .iter()
        .any(|i| i.contains("react") || i.contains("React"));

    let returns_jsx = function
        .return_type
        .as_ref()
        .map(|rt| rt.contains("JSX.Element") || rt.contains("ReactElement"))
        .unwrap_or(false)
        || function.body_contains_jsx;

    let has_react_create_element = function
        .calls
        .iter()
        .any(|call| call.name.contains("React.createElement"));

    // React components typically:
    // - Have React imports
    // - Return JSX elements or use createElement
    // - Have PascalCase names (starts with uppercase letter)
    let is_pascal_case = function
        .name
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false);

    has_react_import && (returns_jsx || has_react_create_element || is_pascal_case)
}

#[cfg(test)]
mod tests {
    use super::super::detector::{FileContext, FunctionAst, Parameter};
    use super::super::patterns::Language;
    use super::*;

    #[test]
    fn test_axum_handler_detection() {
        let mut function = FunctionAst::new("get_user".to_string());
        function.is_async = true;
        function.parameters.push(Parameter {
            name: "user_id".to_string(),
            type_annotation: "Path<u32>".to_string(),
        });
        function.return_type = Some("Json<User>".to_string());

        let mut file_context = FileContext::new(Language::Rust, "handler.rs".into());
        file_context.add_import("use axum::{extract::Path, response::Json};".to_string());

        assert!(is_axum_handler(&function, &file_context));
    }

    #[test]
    fn test_express_handler_detection() {
        let mut function = FunctionAst::new("handleRequest".to_string());
        function.parameters.push(Parameter {
            name: "req".to_string(),
            type_annotation: "Request".to_string(),
        });
        function.parameters.push(Parameter {
            name: "res".to_string(),
            type_annotation: "Response".to_string(),
        });

        let mut file_context = FileContext::new(Language::JavaScript, "routes.js".into());
        file_context.add_import("const express = require('express');".to_string());

        assert!(is_express_handler(&function, &file_context));
    }

    #[test]
    fn test_react_component_detection_with_jsx_return() {
        let mut function = FunctionAst::new("UserProfile".to_string());
        function.return_type = Some("JSX.Element".to_string());

        let mut file_context = FileContext::new(Language::JavaScript, "components.jsx".into());
        file_context.add_import("import React from 'react';".to_string());

        assert!(is_react_component(&function, &file_context));
    }

    #[test]
    fn test_react_component_detection_with_body_jsx() {
        let mut function = FunctionAst::new("Header".to_string());
        function.body_contains_jsx = true;

        let mut file_context = FileContext::new(Language::TypeScript, "Header.tsx".into());
        file_context.add_import("import React from 'react';".to_string());

        assert!(is_react_component(&function, &file_context));
    }

    #[test]
    fn test_react_component_detection_with_pascal_case() {
        let function = FunctionAst::new("MyComponent".to_string());

        let mut file_context = FileContext::new(Language::JavaScript, "MyComponent.js".into());
        file_context.add_import("import React from 'react';".to_string());

        assert!(is_react_component(&function, &file_context));
    }

    #[test]
    fn test_not_react_component_without_import() {
        let mut function = FunctionAst::new("Component".to_string());
        function.body_contains_jsx = true;

        let file_context = FileContext::new(Language::JavaScript, "component.js".into());

        assert!(!is_react_component(&function, &file_context));
    }
}
