/// Integration test for Python event binding detection
/// This test reproduces the issue where wxPython Bind() calls aren't being detected
/// and event handlers are incorrectly showing as having no callers

use debtmap::analysis::python_type_tracker::TwoPassExtractor;
use debtmap::priority::call_graph::FunctionId;
use rustpython_parser;
use std::path::PathBuf;

#[test]
fn test_wxpython_bind_event_handler_detection() {
    // Create a test Python file that mimics the structure from promptconstruct-frontend
    let python_code = r#"
import wx

class MainFrame(wx.Frame):
    def __init__(self, parent=None):
        super().__init__(parent, title="Test Frame")
        self.init_ui()

    def init_ui(self):
        """Initialize the UI components."""
        self.input_box = wx.TextCtrl(self, style=wx.TE_MULTILINE)
        # This Bind() call should create a call from init_ui to on_key_down
        self.input_box.Bind(wx.EVT_KEY_DOWN, self.on_key_down)

    def on_key_down(self, event):
        """Handle key down events - should NOT be dead code."""
        key_code = event.GetKeyCode()
        if key_code == wx.WXK_RETURN:
            self.on_send(event)
        else:
            event.Skip()

    def on_send(self, event):
        """Send the message."""
        pass

    def unused_method(self):
        """This method is truly unused and SHOULD be dead code."""
        return 42

class TestApp(wx.App):
    def OnInit(self):
        """Initialize the application - wxPython framework method."""
        frame = MainFrame()
        frame.Show()
        return True
"#;

    // Parse the Python code
    let module = rustpython_parser::parse(
        python_code,
        rustpython_parser::Mode::Module,
        "test_wxpython.py"
    ).expect("Failed to parse Python code");

    // Extract the call graph using TwoPassExtractor with source
    let file_path = PathBuf::from("test_wxpython.py");
    let mut extractor = TwoPassExtractor::new_with_source(file_path.clone(), python_code);
    let call_graph = extractor.extract(&module);

    // The key test: Check if on_key_down has any callers
    // We'll iterate through all function IDs to find on_key_down
    let mut on_key_down_found = false;
    let mut on_key_down_has_callers = false;
    let mut unused_method_found = false;
    let mut unused_method_has_callers = false;

    // Get all function IDs from the call graph
    let all_functions: Vec<FunctionId> = call_graph.get_all_functions().cloned().collect();

    println!("\n=== Functions found in call graph ===");
    for func_id in &all_functions {
        println!("  - {} at line {}", func_id.name, func_id.line);

        if func_id.name.contains("on_key_down") {
            on_key_down_found = true;
            let callers = call_graph.get_callers(func_id);
            on_key_down_has_callers = !callers.is_empty();
            println!("    on_key_down has {} callers", callers.len());
            for caller in callers {
                println!("      <- called by {} at line {}", caller.name, caller.line);
            }
        }

        if func_id.name.contains("unused_method") {
            unused_method_found = true;
            let callers = call_graph.get_callers(func_id);
            unused_method_has_callers = !callers.is_empty();
            println!("    unused_method has {} callers", callers.len());
        }
    }

    // Print all calls for debugging
    println!("\n=== All function calls detected ===");
    for call in call_graph.get_all_calls() {
        println!("  {} -> {}", call.caller.name, call.callee.name);
    }

    // Assertions
    assert!(on_key_down_found, "on_key_down function not found in call graph");
    assert!(unused_method_found, "unused_method function not found in call graph");

    // THE KEY ASSERTION: on_key_down should have callers from the Bind() call
    assert!(
        on_key_down_has_callers,
        "FAILURE: on_key_down should have callers from Bind() call, but has no callers. This reproduces the bug!"
    );

    // unused_method should have no callers
    assert!(
        !unused_method_has_callers,
        "unused_method should have no callers"
    );
}