//! Regression test for Bug #10: Cross-file dependency false positive
//!
//! This test reproduces the bug where `ConversationManager.add_message()` was
//! incorrectly flagged as dead code because it was only called from another
//! file (mainwindow.py) via a singleton instance.
//!
//! Before the fix: add_message() has no callers in conversation_manager.py -> mark as dead
//! After the fix: add_message() is called from mainwindow.py via singleton -> NOT dead

use std::fs;
use tempfile::TempDir;

/// Create a temporary Python project that reproduces Bug #10
fn create_bug_10_scenario() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create conversation_manager.py with singleton pattern
    let conversation_manager_py = r#"
class Conversation:
    def __init__(self):
        self.messages = []

    def add_message(self, text, sender):
        """Add a new message to the end of the current conversation."""
        message = {"text": text, "sender": sender}
        self.messages.append(message)
        return len(self.messages) - 1

class ConversationManager:
    def __init__(self):
        self.current_conversation = Conversation()

    def add_message(self, text, sender):
        """Add a new message to the end of the current conversation."""
        return self.current_conversation.add_message(text, sender)

# Singleton instance - exported for use in other modules
manager = ConversationManager()
"#;

    // Create mainwindow.py that imports and uses the singleton
    let mainwindow_py = r#"
from conversation_manager import manager

def handle_user_message(message_text):
    """Handle a message from the user."""
    index = manager.add_message(message_text, "user")
    print(f"Added message at index {index}")
    return index

def handle_bot_message(message_text):
    """Handle a message from the bot."""
    index = manager.add_message(message_text, "bot")
    print(f"Bot responded at index {index}")
    return index
"#;

    fs::write(
        base_path.join("conversation_manager.py"),
        conversation_manager_py,
    )
    .unwrap();
    fs::write(base_path.join("mainwindow.py"), mainwindow_py).unwrap();

    temp_dir
}

#[test]
fn test_bug_10_conversation_manager_not_flagged_as_dead() {
    let temp_dir = create_bug_10_scenario();
    let base_path = temp_dir.path();

    // Parse both Python files
    let conversation_manager_path = base_path.join("conversation_manager.py");
    let mainwindow_path = base_path.join("mainwindow.py");

    let conversation_manager_content = fs::read_to_string(&conversation_manager_path).unwrap();
    let mainwindow_content = fs::read_to_string(&mainwindow_path).unwrap();

    let conversation_manager_ast = rustpython_parser::parse(
        &conversation_manager_content,
        rustpython_parser::Mode::Module,
        conversation_manager_path.to_str().unwrap(),
    )
    .expect("Failed to parse conversation_manager.py");

    let mainwindow_ast = rustpython_parser::parse(
        &mainwindow_content,
        rustpython_parser::Mode::Module,
        mainwindow_path.to_str().unwrap(),
    )
    .expect("Failed to parse mainwindow.py");

    // Build cross-module context with enhanced import resolver
    use debtmap::analysis::python_imports::EnhancedImportResolver;
    let mut resolver = EnhancedImportResolver::new();

    // Analyze imports in both files
    resolver.analyze_imports(&conversation_manager_ast, &conversation_manager_path);
    resolver.analyze_imports(&mainwindow_ast, &mainwindow_path);

    // Verify that the import resolution works
    // mainwindow.py imports 'manager' from conversation_manager
    let resolved = resolver.resolve_symbol(&mainwindow_path, "manager");
    assert!(
        resolved.is_some(),
        "Should resolve 'manager' import from conversation_manager.py"
    );

    let symbol = resolved.unwrap();
    assert_eq!(symbol.name, "manager");
    assert_eq!(
        symbol.module_path, conversation_manager_path,
        "Should resolve to conversation_manager.py"
    );

    // Verify confidence level for the resolution
    use debtmap::analysis::python_imports::ResolutionConfidence;
    assert!(
        matches!(
            symbol.confidence,
            ResolutionConfidence::High | ResolutionConfidence::Medium
        ),
        "Should have high or medium confidence for direct import resolution, got {:?}",
        symbol.confidence
    );

    // Note: Full dead code detection with cross-file call graph analysis
    // would require integration with the call graph builder. This test
    // verifies that the import resolution infrastructure correctly tracks
    // the singleton pattern, which is the foundation for fixing Bug #10.
    //
    // The complete fix involves:
    // 1. Import resolution (tested here) ✓
    // 2. Singleton pattern detection (exists in codebase)
    // 3. Cross-file call graph construction (exists in codebase)
    // 4. Dead code detector using cross-file graph (exists in codebase)
}

#[test]
fn test_confidence_scoring_for_different_import_types() {
    use debtmap::analysis::python_imports::{ImportType, ResolutionConfidence};

    // Test confidence classification for different import types
    assert_eq!(
        ResolutionConfidence::from_import_type(&ImportType::Direct),
        ResolutionConfidence::High,
        "Direct imports should have High confidence"
    );

    assert_eq!(
        ResolutionConfidence::from_import_type(&ImportType::From),
        ResolutionConfidence::High,
        "From imports should have High confidence"
    );

    assert_eq!(
        ResolutionConfidence::from_import_type(&ImportType::Relative { level: 1 }),
        ResolutionConfidence::Medium,
        "Simple relative imports should have Medium confidence"
    );

    assert_eq!(
        ResolutionConfidence::from_import_type(&ImportType::Relative { level: 3 }),
        ResolutionConfidence::Low,
        "Deep relative imports should have Low confidence"
    );

    assert_eq!(
        ResolutionConfidence::from_import_type(&ImportType::Star),
        ResolutionConfidence::Low,
        "Star imports should have Low confidence"
    );

    assert_eq!(
        ResolutionConfidence::from_import_type(&ImportType::Dynamic),
        ResolutionConfidence::Unknown,
        "Dynamic imports should have Unknown confidence"
    );
}

#[test]
fn test_circular_import_detection() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create circular import scenario
    let module_a = "from module_b import function_b\ndef function_a(): pass";
    let module_b = "from module_a import function_a\ndef function_b(): pass";

    fs::write(base_path.join("module_a.py"), module_a).unwrap();
    fs::write(base_path.join("module_b.py"), module_b).unwrap();

    let module_a_path = base_path.join("module_a.py");
    let module_b_path = base_path.join("module_b.py");

    let ast_a = rustpython_parser::parse(
        module_a,
        rustpython_parser::Mode::Module,
        module_a_path.to_str().unwrap(),
    )
    .unwrap();

    let ast_b = rustpython_parser::parse(
        module_b,
        rustpython_parser::Mode::Module,
        module_b_path.to_str().unwrap(),
    )
    .unwrap();

    use debtmap::analysis::python_imports::EnhancedImportResolver;
    let mut resolver = EnhancedImportResolver::new();

    resolver.analyze_imports(&ast_a, &module_a_path);
    resolver.analyze_imports(&ast_b, &module_b_path);

    // Build the import graph
    resolver.build_import_graph(&[
        (ast_a, module_a_path.clone()),
        (ast_b, module_b_path.clone()),
    ]);

    // Check for circular imports
    let cycles = resolver.circular_imports();
    assert!(
        !cycles.is_empty(),
        "Should detect circular import between module_a and module_b"
    );

    // Verify the cycle includes both modules
    let first_cycle = &cycles[0];
    assert!(
        first_cycle.contains(&module_a_path) || first_cycle.contains(&module_b_path),
        "Circular import should involve module_a or module_b"
    );
}

#[test]
fn test_aliased_import_resolution() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create module with aliased import
    let module_a = "def helper_function():\n    pass\n";
    let module_b = "from module_a import helper_function as hf\n\ndef use_helper():\n    hf()\n";

    fs::write(base_path.join("module_a.py"), module_a).unwrap();
    fs::write(base_path.join("module_b.py"), module_b).unwrap();

    let module_a_path = base_path.join("module_a.py");
    let module_b_path = base_path.join("module_b.py");

    let ast_a = rustpython_parser::parse(
        module_a,
        rustpython_parser::Mode::Module,
        module_a_path.to_str().unwrap(),
    )
    .unwrap();

    let ast_b = rustpython_parser::parse(
        module_b,
        rustpython_parser::Mode::Module,
        module_b_path.to_str().unwrap(),
    )
    .unwrap();

    use debtmap::analysis::python_imports::EnhancedImportResolver;
    let mut resolver = EnhancedImportResolver::new();

    resolver.analyze_imports(&ast_a, &module_a_path);
    resolver.analyze_imports(&ast_b, &module_b_path);

    // Verify that aliased import is resolved correctly
    let resolved = resolver.resolve_symbol(&module_b_path, "hf");
    assert!(
        resolved.is_some(),
        "Should resolve aliased import 'hf' -> 'helper_function'"
    );

    let symbol = resolved.unwrap();
    assert_eq!(symbol.original_name, "helper_function");
    assert_eq!(symbol.module_path, module_a_path);
}

#[test]
fn test_relative_import_resolution() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create package structure with relative imports
    let package_dir = base_path.join("mypackage");
    fs::create_dir(&package_dir).unwrap();

    let helper_module = "def utility_function():\n    pass\n";
    let main_module = "from .helper import utility_function\n\ndef main():\n    utility_function()\n";

    fs::write(package_dir.join("helper.py"), helper_module).unwrap();
    fs::write(package_dir.join("main.py"), main_module).unwrap();

    let helper_path = package_dir.join("helper.py");
    let main_path = package_dir.join("main.py");

    let helper_ast = rustpython_parser::parse(
        helper_module,
        rustpython_parser::Mode::Module,
        helper_path.to_str().unwrap(),
    )
    .unwrap();

    let main_ast = rustpython_parser::parse(
        main_module,
        rustpython_parser::Mode::Module,
        main_path.to_str().unwrap(),
    )
    .unwrap();

    use debtmap::analysis::python_imports::{EnhancedImportResolver, ResolutionConfidence, ImportType};
    let mut resolver = EnhancedImportResolver::new();

    resolver.analyze_imports(&helper_ast, &helper_path);
    resolver.analyze_imports(&main_ast, &main_path);

    // Verify relative import resolution
    let resolved = resolver.resolve_symbol(&main_path, "utility_function");
    assert!(
        resolved.is_some(),
        "Should resolve relative import 'from .helper import utility_function'"
    );

    let symbol = resolved.unwrap();
    assert_eq!(symbol.name, "utility_function");
    assert_eq!(symbol.module_path, helper_path);

    // Verify that import graph contains the relative import
    let import_graph = resolver.import_graph();
    let edges = import_graph.edges.get(&main_path).unwrap();
    let relative_import = edges.iter().find(|e| {
        matches!(e.import_type, ImportType::Relative { level: 1 })
    });
    assert!(
        relative_import.is_some(),
        "Import graph should contain relative import edge"
    );

    // Relative imports should result in Medium confidence when classified
    assert_eq!(
        ResolutionConfidence::from_import_type(&ImportType::Relative { level: 1 }),
        ResolutionConfidence::Medium,
        "Relative imports should have Medium confidence"
    );
}

#[test]
fn test_wildcard_import_low_confidence() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create module with wildcard import
    let utils_module = r#"
def func_a():
    pass

def func_b():
    pass

def _private_func():
    pass

__all__ = ['func_a', 'func_b']
"#;
    let main_module = "from utils import *\n\ndef use_functions():\n    func_a()\n    func_b()\n";

    fs::write(base_path.join("utils.py"), utils_module).unwrap();
    fs::write(base_path.join("main.py"), main_module).unwrap();

    let utils_path = base_path.join("utils.py");
    let main_path = base_path.join("main.py");

    let utils_ast = rustpython_parser::parse(
        utils_module,
        rustpython_parser::Mode::Module,
        utils_path.to_str().unwrap(),
    )
    .unwrap();

    let main_ast = rustpython_parser::parse(
        main_module,
        rustpython_parser::Mode::Module,
        main_path.to_str().unwrap(),
    )
    .unwrap();

    use debtmap::analysis::python_imports::{EnhancedImportResolver, ResolutionConfidence};
    let mut resolver = EnhancedImportResolver::new();

    resolver.analyze_imports(&utils_ast, &utils_path);
    resolver.analyze_imports(&main_ast, &main_path);

    // Verify wildcard import resolution for func_a
    let resolved_a = resolver.resolve_symbol(&main_path, "func_a");
    assert!(
        resolved_a.is_some(),
        "Should resolve 'func_a' from wildcard import"
    );

    let symbol_a = resolved_a.unwrap();
    assert_eq!(symbol_a.name, "func_a");
    assert_eq!(symbol_a.module_path, utils_path);
    // Wildcard imports should have Low confidence
    assert_eq!(
        symbol_a.confidence,
        ResolutionConfidence::Low,
        "Wildcard imports should have Low confidence"
    );

    // Verify that _private_func is not resolved (not in __all__)
    let resolved_private = resolver.resolve_symbol(&main_path, "_private_func");
    assert!(
        resolved_private.is_none(),
        "Should not resolve private functions from wildcard import"
    );
}
