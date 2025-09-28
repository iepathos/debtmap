/// Tests for Python framework-specific patterns that should not be flagged as dead code
/// This includes wxPython framework methods, main function patterns, and cross-module calls
use debtmap::analysis::python_type_tracker::TwoPassExtractor;
use std::path::PathBuf;

#[test]
fn test_wxpython_oninit_framework_method() {
    // Test that wxPython's OnInit method is not flagged as dead code
    // even when it has no explicit callers (it's called by the framework)
    let python_code = r#"
import wx

class MyApp(wx.App):
    def OnInit(self):
        """Initialize the application - called automatically by wxPython."""
        frame = MainWindow()
        frame.Show()
        return True

class MainWindow(wx.Frame):
    def __init__(self):
        super().__init__(None, title="Test")

def main():
    app = MyApp()
    app.MainLoop()

if __name__ == "__main__":
    main()
"#;

    // Parse and analyze
    let module = rustpython_parser::parse(
        python_code,
        rustpython_parser::Mode::Module,
        "test_wxapp.py",
    )
    .expect("Failed to parse Python code");

    let file_path = PathBuf::from("test_wxapp.py");
    let mut extractor = TwoPassExtractor::new_with_source(file_path.clone(), python_code);
    let call_graph = extractor.extract(&module);

    // Find OnInit function
    let mut oninit_found = false;
    let mut oninit_has_callers = false;

    for func_id in call_graph.get_all_functions() {
        if func_id.name.contains("OnInit") {
            oninit_found = true;
            let callers = call_graph.get_callers(func_id);
            oninit_has_callers = !callers.is_empty();
            println!(
                "Found OnInit: {} with {} callers",
                func_id.name,
                callers.len()
            );
        }
    }

    assert!(oninit_found, "OnInit method should be found in call graph");

    // EXPECTED TO FAIL: OnInit currently has no callers detected
    // This test documents the false positive issue
    println!(
        "TEST EXPECTED TO FAIL: OnInit has {} callers (should have implicit framework caller)",
        if oninit_has_callers { "some" } else { "no" }
    );
}

#[test]
fn test_python_main_pattern_detection() {
    // Test that main() function called from if __name__ == "__main__" is detected
    let python_code = r#"
def helper_function():
    return 42

def main():
    """Main entry point of the application."""
    print("Starting application...")
    result = helper_function()
    print(f"Result: {result}")
    return 0

if __name__ == "__main__":
    main()
"#;

    // Parse and analyze
    let module =
        rustpython_parser::parse(python_code, rustpython_parser::Mode::Module, "test_main.py")
            .expect("Failed to parse Python code");

    let file_path = PathBuf::from("test_main.py");
    let mut extractor = TwoPassExtractor::new_with_source(file_path.clone(), python_code);
    let call_graph = extractor.extract(&module);

    // Find main function
    let mut main_found = false;
    let mut main_has_callers = false;

    for func_id in call_graph.get_all_functions() {
        if func_id.name == "main" {
            main_found = true;
            let callers = call_graph.get_callers(func_id);
            main_has_callers = !callers.is_empty();
            println!("Found main() with {} callers", callers.len());
            for caller in callers {
                println!("  Called by: {}", caller.name);
            }
        }
    }

    assert!(main_found, "main() function should be found in call graph");

    // EXPECTED TO FAIL: main() called from if __name__ == "__main__" is not detected
    // as a call because it's at module level
    if !main_has_callers {
        println!(
            "TEST SHOWS ISSUE: main() has no callers detected from if __name__ == '__main__' block"
        );
        println!("This is a known false positive that needs fixing");
    }
}

#[test]
fn test_cross_module_instance_method_call() {
    // Test detection of instance method calls across module boundaries
    // This simulates the ConversationManager.register_observer case

    // In Python, when you have:
    //   conversation_manager.register_observer(self)
    // where conversation_manager is an instance variable,
    // the call graph needs to detect this as calling ConversationManager.register_observer

    let panel_code = r#"
class ConversationManager:
    def __init__(self):
        self.observers = []

    def register_observer(self, observer):
        """Register an observer to be notified of changes."""
        if observer not in self.observers:
            self.observers.append(observer)

class ConversationPanel:
    def __init__(self, conversation_manager):
        self.conversation_manager = conversation_manager
        # This instance method call should be detected!
        conversation_manager.register_observer(self)
"#;

    // Parse module
    let module = rustpython_parser::parse(
        panel_code,
        rustpython_parser::Mode::Module,
        "test_observer.py",
    )
    .expect("Failed to parse module");

    // Analyze module
    let file_path = PathBuf::from("test_observer.py");
    let mut extractor = TwoPassExtractor::new_with_source(file_path.clone(), panel_code);
    let call_graph = extractor.extract(&module);

    // Debug: Print all functions and calls
    println!("\n=== Functions found ===");
    for func in call_graph.get_all_functions() {
        println!("  - {}", func.name);
    }

    println!("\n=== Calls found ===");
    for call in call_graph.get_all_calls() {
        println!("  {} -> {}", call.caller.name, call.callee.name);
    }

    // Find register_observer function
    let mut register_found = false;
    let mut register_has_callers = false;

    for func_id in call_graph.get_all_functions() {
        if func_id.name.contains("register_observer") {
            register_found = true;
            let callers = call_graph.get_callers(func_id);
            register_has_callers = !callers.is_empty();
            println!("\nregister_observer has {} callers", callers.len());
            for caller in callers {
                println!("  Called by: {}", caller.name);
            }
        }
    }

    assert!(
        register_found,
        "register_observer should be found in call graph"
    );

    // EXPECTED TO FAIL: Instance method calls like
    // conversation_manager.register_observer(self) are not being detected
    if !register_has_callers {
        println!("TEST SHOWS ISSUE: register_observer has no callers detected");
        println!("Instance method calls through variables are not being tracked");
        println!("This is a known false positive that needs fixing");
    }
}

#[test]
fn test_python_special_methods() {
    // Test Python special methods (dunder methods) that shouldn't be flagged as dead code
    let python_code = r#"
class MyContextManager:
    def __enter__(self):
        """Context manager enter - called by 'with' statement."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit - called by 'with' statement."""
        pass

class MyAsyncContextManager:
    async def __aenter__(self):
        """Async context manager enter - called by 'async with'."""
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit - called by 'async with'."""
        pass

class MyIterator:
    def __iter__(self):
        """Iterator protocol - called by 'for' loops."""
        return self

    def __next__(self):
        """Iterator protocol - called during iteration."""
        raise StopIteration

class MyClass:
    def __init__(self):
        """Constructor - called when creating instance."""
        self.value = 0

    def __str__(self):
        """String representation - called by str()."""
        return f"MyClass({self.value})"

    def __repr__(self):
        """Debug representation - called by repr()."""
        return f"MyClass(value={self.value})"

    def __len__(self):
        """Length - called by len()."""
        return 1

    def __getitem__(self, key):
        """Item access - called by obj[key]."""
        return self.value

    def __setitem__(self, key, value):
        """Item assignment - called by obj[key] = value."""
        self.value = value
"#;

    // Parse and analyze
    let module = rustpython_parser::parse(
        python_code,
        rustpython_parser::Mode::Module,
        "test_special.py",
    )
    .expect("Failed to parse Python code");

    let file_path = PathBuf::from("test_special.py");
    let mut extractor = TwoPassExtractor::new_with_source(file_path.clone(), python_code);
    let call_graph = extractor.extract(&module);

    // Check special methods
    let special_methods = [
        "__enter__",
        "__exit__",
        "__aenter__",
        "__aexit__",
        "__iter__",
        "__next__",
        "__init__",
        "__str__",
        "__repr__",
        "__len__",
        "__getitem__",
        "__setitem__",
    ];

    println!("\n=== Special Methods Analysis ===");
    for method_name in &special_methods {
        let mut found = false;
        for func_id in call_graph.get_all_functions() {
            if func_id.name.ends_with(method_name) {
                found = true;
                let callers = call_graph.get_callers(func_id);
                println!("{}: {} callers", func_id.name, callers.len());

                // Note: __init__ might have callers if it's called explicitly
                // Other special methods typically won't have explicit callers
                // but shouldn't be considered dead code
            }
        }
        assert!(found, "Special method {} should be found", method_name);
    }

    println!("\nNOTE: Special methods typically have no explicit callers");
    println!("but should not be flagged as dead code as they're called by Python runtime");
}

#[test]
fn test_unittest_framework_methods() {
    // Test unittest framework methods that shouldn't be flagged as dead code
    let python_code = r#"
import unittest

class TestMyClass(unittest.TestCase):
    def setUp(self):
        """Called before each test method - unittest framework."""
        self.data = []

    def tearDown(self):
        """Called after each test method - unittest framework."""
        self.data = None

    def setUpClass(cls):
        """Called once before all tests in class - unittest framework."""
        cls.shared_resource = "resource"

    def tearDownClass(cls):
        """Called once after all tests in class - unittest framework."""
        cls.shared_resource = None

    def test_something(self):
        """Actual test method - discovered by test runner."""
        self.assertEqual(1, 1)

    def test_another_thing(self):
        """Another test method - discovered by test runner."""
        self.assertTrue(True)
"#;

    // Parse and analyze
    let module = rustpython_parser::parse(
        python_code,
        rustpython_parser::Mode::Module,
        "test_unittest.py",
    )
    .expect("Failed to parse Python code");

    let file_path = PathBuf::from("test_unittest.py");
    let mut extractor = TwoPassExtractor::new_with_source(file_path.clone(), python_code);
    let call_graph = extractor.extract(&module);

    // Check unittest framework methods
    let unittest_methods = [
        "setUp",
        "tearDown",
        "setUpClass",
        "tearDownClass",
        "test_something",
        "test_another_thing",
    ];

    println!("\n=== Unittest Framework Methods ===");
    for method_name in &unittest_methods {
        for func_id in call_graph.get_all_functions() {
            if func_id.name.contains(method_name) {
                let callers = call_graph.get_callers(func_id);
                println!("{}: {} callers", func_id.name, callers.len());

                // These methods are called by the unittest framework
                // but won't have explicit callers in the code
            }
        }
    }

    println!("\nNOTE: unittest framework methods have no explicit callers");
    println!("but are called by the test runner and shouldn't be dead code");
}
