/// Tests for Python cross-module call graph analysis
use debtmap::analysis::python_call_graph::{analyze_python_project, build_cross_module_context};
use debtmap::analysis::python_type_tracker::TwoPassExtractor;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_cross_module_function_calls() {
    // Create temporary directory with Python modules
    let temp_dir = TempDir::new().unwrap();

    // Module 1: utils.py - defines utility functions
    let utils_path = temp_dir.path().join("utils.py");
    fs::write(
        &utils_path,
        r#"
def format_message(msg):
    """Format a message for display."""
    return f"[INFO] {msg}"

def log_message(msg):
    """Log a message to console."""
    formatted = format_message(msg)
    print(formatted)
"#,
    )
    .unwrap();

    // Module 2: main.py - uses utility functions
    let main_path = temp_dir.path().join("main.py");
    fs::write(
        &main_path,
        r#"
from utils import log_message, format_message

def process_data(data):
    """Process data and log results."""
    log_message(f"Processing {len(data)} items")
    result = format_message("Complete")
    return result

def main():
    data = [1, 2, 3, 4, 5]
    process_data(data)

if __name__ == "__main__":
    main()
"#,
    )
    .unwrap();

    // Analyze the project
    let files = vec![utils_path.clone(), main_path.clone()];
    let call_graph = analyze_python_project(&files).unwrap();

    // Check that cross-module calls are detected
    let all_functions: Vec<_> = call_graph.get_all_functions().collect();

    // Debug: print all functions found
    println!("Found {} functions:", all_functions.len());
    for func in &all_functions {
        println!(
            "  - {} (file: {}, line: {})",
            func.name,
            func.file.display(),
            func.line
        );
    }

    // Should have functions from both modules
    assert!(
        all_functions.iter().any(|f| f.name == "format_message"),
        "format_message not found"
    );
    assert!(all_functions.iter().any(|f| f.name == "log_message"));
    assert!(all_functions.iter().any(|f| f.name == "process_data"));
    assert!(all_functions.iter().any(|f| f.name == "main"));

    // Check that log_message has callers (called from process_data)
    let log_message_func = all_functions
        .iter()
        .find(|f| f.name == "log_message")
        .expect("log_message should exist");

    let callers = call_graph.get_callers(log_message_func);
    assert!(
        !callers.is_empty(),
        "log_message should have callers from process_data"
    );
}

#[test]
fn test_cross_module_class_methods() {
    let temp_dir = TempDir::new().unwrap();

    // Module 1: manager.py - defines a Manager class
    let manager_path = temp_dir.path().join("manager.py");
    fs::write(
        &manager_path,
        r#"
class ConversationManager:
    def __init__(self):
        self.observers = []
        self.conversations = []

    def register_observer(self, observer):
        """Register an observer to be notified of changes."""
        if observer not in self.observers:
            self.observers.append(observer)

    def notify_observers(self, event):
        """Notify all observers of an event."""
        for observer in self.observers:
            observer.on_event(event)

    def add_conversation(self, conversation):
        """Add a new conversation."""
        self.conversations.append(conversation)
        self.notify_observers({"type": "conversation_added", "data": conversation})
"#,
    )
    .unwrap();

    // Module 2: panel.py - uses the Manager class
    let panel_path = temp_dir.path().join("panel.py");
    fs::write(
        &panel_path,
        r#"
from manager import ConversationManager

class ConversationPanel:
    def __init__(self, conversation_manager):
        self.conversation_manager = conversation_manager
        # This should be detected as calling ConversationManager.register_observer
        conversation_manager.register_observer(self)
        self.messages = []

    def on_event(self, event):
        """Handle events from the conversation manager."""
        if event["type"] == "conversation_added":
            self.refresh_display()

    def refresh_display(self):
        """Refresh the display with latest data."""
        pass

    def add_message(self, message):
        """Add a message to the panel."""
        self.messages.append(message)

def setup_application():
    manager = ConversationManager()
    panel = ConversationPanel(manager)
    manager.add_conversation({"id": 1, "title": "Test"})
    return panel
"#,
    )
    .unwrap();

    // Analyze the project
    let files = vec![manager_path.clone(), panel_path.clone()];
    let call_graph = analyze_python_project(&files).unwrap();

    // Check that cross-module method calls are detected
    let all_functions: Vec<_> = call_graph.get_all_functions().collect();

    // Should have methods from both classes
    assert!(all_functions
        .iter()
        .any(|f| f.name.contains("register_observer")));
    assert!(all_functions
        .iter()
        .any(|f| f.name.contains("notify_observers")));
    assert!(all_functions.iter().any(|f| f.name.contains("on_event")));

    // Check that register_observer has callers (called from ConversationPanel.__init__)
    let register_observer = all_functions
        .iter()
        .find(|f| f.name.contains("register_observer"))
        .expect("register_observer should exist");

    let callers = call_graph.get_callers(register_observer);
    assert!(
        !callers.is_empty(),
        "register_observer should have callers from ConversationPanel.__init__"
    );
}

#[test]
fn test_wxpython_oninit_cross_module() {
    let temp_dir = TempDir::new().unwrap();

    // Module 1: app.py - wxPython application
    let app_path = temp_dir.path().join("app.py");
    fs::write(
        &app_path,
        r#"
import wx
from main_window import MainWindow

class MyApp(wx.App):
    def OnInit(self):
        """Initialize the application - called automatically by wxPython."""
        self.frame = MainWindow(None, title="My Application")
        self.frame.Show()
        return True

def main():
    app = MyApp()
    app.MainLoop()

if __name__ == "__main__":
    main()
"#,
    )
    .unwrap();

    // Module 2: main_window.py - Main window class
    let window_path = temp_dir.path().join("main_window.py");
    fs::write(
        &window_path,
        r#"
import wx

class MainWindow(wx.Frame):
    def __init__(self, parent, title):
        super().__init__(parent, title=title)
        self.init_ui()

    def init_ui(self):
        """Initialize the user interface."""
        panel = wx.Panel(self)
        sizer = wx.BoxSizer(wx.VERTICAL)

        self.text_ctrl = wx.TextCtrl(panel)
        sizer.Add(self.text_ctrl, 1, wx.EXPAND)

        panel.SetSizer(sizer)
"#,
    )
    .unwrap();

    // Analyze the project
    let files = vec![app_path.clone(), window_path.clone()];
    let call_graph = analyze_python_project(&files).unwrap();

    // Check that OnInit is detected (even if no explicit callers due to framework)
    let all_functions: Vec<_> = call_graph.get_all_functions().collect();
    assert!(all_functions.iter().any(|f| f.name.contains("OnInit")));

    // Check that MainWindow.__init__ has callers from OnInit
    let main_window_init = all_functions
        .iter()
        .find(|f| f.name.contains("MainWindow") && f.name.contains("__init__"))
        .expect("MainWindow.__init__ should exist");

    let callers = call_graph.get_callers(main_window_init);
    assert!(
        !callers.is_empty(),
        "MainWindow.__init__ should have callers from OnInit"
    );
}

#[test]
fn test_relative_imports() {
    let temp_dir = TempDir::new().unwrap();

    // Create a package structure
    let package_dir = temp_dir.path().join("mypackage");
    fs::create_dir(&package_dir).unwrap();

    // __init__.py
    let init_path = package_dir.join("__init__.py");
    fs::write(&init_path, "").unwrap();

    // core.py
    let core_path = package_dir.join("core.py");
    fs::write(
        &core_path,
        r#"
class Core:
    def process(self, data):
        return data * 2

    def validate(self, data):
        return data > 0
"#,
    )
    .unwrap();

    // utils.py with relative import
    let utils_path = package_dir.join("utils.py");
    fs::write(
        &utils_path,
        r#"
from .core import Core

class Processor:
    def __init__(self):
        self.core = Core()

    def run(self, data):
        if self.core.validate(data):
            return self.core.process(data)
        return None
"#,
    )
    .unwrap();

    // Analyze the package
    let files = vec![core_path.clone(), utils_path.clone()];
    let call_graph = analyze_python_project(&files).unwrap();

    // Check that methods are detected across relative imports
    let all_functions: Vec<_> = call_graph.get_all_functions().collect();

    // Check Core methods exist
    assert!(all_functions
        .iter()
        .any(|f| f.name.contains("Core.process")));
    assert!(all_functions
        .iter()
        .any(|f| f.name.contains("Core.validate")));

    // Check that Core.process has callers from Processor.run
    let core_process = all_functions
        .iter()
        .find(|f| f.name.contains("Core.process"))
        .expect("Core.process should exist");

    let callers = call_graph.get_callers(core_process);
    assert!(
        !callers.is_empty(),
        "Core.process should have callers from Processor.run"
    );
}

#[test]
fn test_wildcard_imports() {
    let temp_dir = TempDir::new().unwrap();

    // Module with functions
    let helpers_path = temp_dir.path().join("helpers.py");
    fs::write(
        &helpers_path,
        r#"
def helper_one():
    return 1

def helper_two():
    return 2

def helper_three():
    return helper_one() + helper_two()
"#,
    )
    .unwrap();

    // Module using wildcard import
    let main_path = temp_dir.path().join("main.py");
    fs::write(
        &main_path,
        r#"
from helpers import *

def main():
    result = helper_three()
    print(f"Result: {result}")
    return result
"#,
    )
    .unwrap();

    // Analyze the project
    let files = vec![helpers_path.clone(), main_path.clone()];
    let call_graph = analyze_python_project(&files).unwrap();

    // Check that wildcard imported functions are tracked
    let all_functions: Vec<_> = call_graph.get_all_functions().collect();

    // Check helper_three has callers from main
    let helper_three = all_functions
        .iter()
        .find(|f| f.name == "helper_three")
        .expect("helper_three should exist");

    let callers = call_graph.get_callers(helper_three);
    assert!(
        !callers.is_empty(),
        "helper_three should have callers from main"
    );
}
