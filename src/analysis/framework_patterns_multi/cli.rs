//! CLI Framework Pattern Detection

use super::detector::{FileContext, FunctionAst};

/// Detect Clap CLI parser (Rust)
pub fn is_clap_parser(function: &FunctionAst, file_context: &FileContext) -> bool {
    let has_clap_import = file_context.imports.iter().any(|i| i.contains("clap"));

    let has_parser_derive = function
        .derives
        .iter()
        .any(|d| d == "Parser" || d == "Args" || d == "Subcommand");

    has_clap_import && has_parser_derive
}

/// Detect Click CLI command (Python)
pub fn is_click_command(function: &FunctionAst) -> bool {
    function.decorators.iter().any(|d| {
        d.name.contains("click.command")
            || d.name.contains("click.group")
            || d.name.contains("@command")
            || d.name.contains("@group")
    })
}

#[cfg(test)]
mod tests {
    use super::super::detector::{Decorator, FileContext, FunctionAst};
    use super::super::patterns::Language;
    use super::*;

    #[test]
    fn test_clap_parser_detection() {
        let mut function = FunctionAst::new("Args".to_string());
        function.derives.push("Parser".to_string());

        let mut file_context = FileContext::new(Language::Rust, "cli.rs".into());
        file_context.add_import("use clap::Parser;".to_string());

        assert!(is_clap_parser(&function, &file_context));
    }

    #[test]
    fn test_click_command_detection() {
        let mut function = FunctionAst::new("deploy".to_string());
        function.decorators.push(Decorator {
            name: "@click.command".to_string(),
        });

        assert!(is_click_command(&function));
    }
}
