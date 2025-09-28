use debtmap::analysis::python_call_graph::{analyze_python_project, build_cross_module_context};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_cross_module_function_calls() {
    let temp_dir = TempDir::new().unwrap();

    // Create module1.py with a function
    let module1_path = temp_dir.path().join("module1.py");
    fs::write(
        &module1_path,
        r#"
def process_data(data):
    """Process the input data."""
    return data * 2

def helper_function():
    """Helper function that should be called."""
    return 42
"#,
    )
    .unwrap();

    // Create module2.py that imports and uses module1
    let module2_path = temp_dir.path().join("module2.py");
    fs::write(
        &module2_path,
        r#"
import module1

def main():
    result = module1.process_data(10)
    helper = module1.helper_function()
    return result + helper
"#,
    )
    .unwrap();

    // Analyze the project
    let files = vec![module1_path.clone(), module2_path.clone()];
    let call_graph = analyze_python_project(&files).unwrap();

    // Check that cross-module calls are detected
    let functions: Vec<_> = call_graph.get_all_functions().collect();
    assert!(functions.len() >= 3); // process_data, helper_function, main

    // Verify that process_data and helper_function have callers
    let process_data_id = functions
        .iter()
        .find(|f| f.name.contains("process_data"))
        .expect("process_data should be found");

    let helper_id = functions
        .iter()
        .find(|f| f.name.contains("helper_function"))
        .expect("helper_function should be found");

    assert!(
        !call_graph.get_callers(process_data_id).is_empty(),
        "process_data should have callers from module2"
    );
    assert!(
        !call_graph.get_callers(helper_id).is_empty(),
        "helper_function should have callers from module2"
    );
}

#[test]
fn test_from_import_resolution() {
    let temp_dir = TempDir::new().unwrap();

    // Create utils.py with utility functions
    let utils_path = temp_dir.path().join("utils.py");
    fs::write(
        &utils_path,
        r#"
def format_string(s):
    """Format a string."""
    return s.upper()

def calculate_sum(a, b):
    """Calculate sum of two numbers."""
    return a + b
"#,
    )
    .unwrap();

    // Create main.py that uses from import
    let main_path = temp_dir.path().join("main.py");
    fs::write(
        &main_path,
        r#"
from utils import format_string, calculate_sum

def process():
    result = format_string("hello")
    total = calculate_sum(10, 20)
    return result, total
"#,
    )
    .unwrap();

    // Analyze the project
    let files = vec![utils_path, main_path];
    let call_graph = analyze_python_project(&files).unwrap();

    // Check that imported functions are properly resolved
    let functions: Vec<_> = call_graph.get_all_functions().collect();

    let format_string_id = functions
        .iter()
        .find(|f| f.name.contains("format_string"))
        .expect("format_string should be found");

    let calculate_sum_id = functions
        .iter()
        .find(|f| f.name.contains("calculate_sum"))
        .expect("calculate_sum should be found");

    assert!(
        !call_graph.get_callers(format_string_id).is_empty(),
        "format_string should have callers"
    );
    assert!(
        !call_graph.get_callers(calculate_sum_id).is_empty(),
        "calculate_sum should have callers"
    );
}

#[test]
fn test_class_method_cross_module() {
    let temp_dir = TempDir::new().unwrap();

    // Create manager.py with a class
    let manager_path = temp_dir.path().join("manager.py");
    fs::write(
        &manager_path,
        r#"
class DataManager:
    def __init__(self):
        self.data = []

    def add_item(self, item):
        """Add an item to the manager."""
        self.data.append(item)

    def process_all(self):
        """Process all items."""
        return [self.process_item(item) for item in self.data]

    def process_item(self, item):
        """Process a single item."""
        return item * 2
"#,
    )
    .unwrap();

    // Create app.py that uses the DataManager
    let app_path = temp_dir.path().join("app.py");
    fs::write(
        &app_path,
        r#"
from manager import DataManager

def run_application():
    mgr = DataManager()
    mgr.add_item(10)
    mgr.add_item(20)
    results = mgr.process_all()
    return results
"#,
    )
    .unwrap();

    // Analyze the project
    let files = vec![manager_path, app_path];
    let call_graph = analyze_python_project(&files).unwrap();

    // Check that class methods are detected across modules
    let functions: Vec<_> = call_graph.get_all_functions().collect();

    let add_item_id = functions
        .iter()
        .find(|f| f.name.contains("add_item"))
        .expect("add_item should be found");

    let process_all_id = functions
        .iter()
        .find(|f| f.name.contains("process_all"))
        .expect("process_all should be found");

    assert!(
        !call_graph.get_callers(add_item_id).is_empty(),
        "add_item should have callers from app.py"
    );
    assert!(
        !call_graph.get_callers(process_all_id).is_empty(),
        "process_all should have callers from app.py"
    );
}

#[test]
fn test_aliased_imports() {
    let temp_dir = TempDir::new().unwrap();

    // Create helpers.py
    let helpers_path = temp_dir.path().join("helpers.py");
    fs::write(
        &helpers_path,
        r#"
def validate_input(value):
    """Validate input value."""
    return value is not None and value > 0

def transform_data(data):
    """Transform the data."""
    return [x * 2 for x in data]
"#,
    )
    .unwrap();

    // Create processor.py with aliased imports
    let processor_path = temp_dir.path().join("processor.py");
    fs::write(
        &processor_path,
        r#"
import helpers as h
from helpers import validate_input as validate

def process_batch(batch):
    valid_items = [x for x in batch if validate(x)]
    transformed = h.transform_data(valid_items)
    return transformed
"#,
    )
    .unwrap();

    // Analyze the project
    let files = vec![helpers_path, processor_path];
    let call_graph = analyze_python_project(&files).unwrap();

    // Check that aliased functions are properly resolved
    let functions: Vec<_> = call_graph.get_all_functions().collect();

    let validate_id = functions
        .iter()
        .find(|f| f.name.contains("validate_input"))
        .expect("validate_input should be found");

    let transform_id = functions
        .iter()
        .find(|f| f.name.contains("transform_data"))
        .expect("transform_data should be found");

    assert!(
        !call_graph.get_callers(validate_id).is_empty(),
        "validate_input should have callers through alias"
    );
    assert!(
        !call_graph.get_callers(transform_id).is_empty(),
        "transform_data should have callers through module alias"
    );
}

#[test]
fn test_observer_pattern_cross_module() {
    let temp_dir = TempDir::new().unwrap();

    // Create observer.py with observer base class
    let observer_path = temp_dir.path().join("observer.py");
    fs::write(
        &observer_path,
        r#"
class Observer:
    def update(self, event):
        """Called when an event occurs."""
        pass

class Subject:
    def __init__(self):
        self.observers = []

    def attach(self, observer):
        """Attach an observer."""
        self.observers.append(observer)

    def notify(self, event):
        """Notify all observers."""
        for obs in self.observers:
            obs.update(event)
"#,
    )
    .unwrap();

    // Create concrete_observer.py
    let concrete_path = temp_dir.path().join("concrete_observer.py");
    fs::write(
        &concrete_path,
        r#"
from observer import Observer, Subject

class ConcreteObserver(Observer):
    def update(self, event):
        """Handle the update event."""
        print(f"Received event: {event}")
        self.handle_event(event)

    def handle_event(self, event):
        """Process the event."""
        return event.upper() if isinstance(event, str) else event

def setup_observers():
    subject = Subject()
    observer = ConcreteObserver()
    subject.attach(observer)
    subject.notify("test_event")
"#,
    )
    .unwrap();

    // Analyze the project
    let files = vec![observer_path, concrete_path];
    let call_graph = analyze_python_project(&files).unwrap();

    // Check that observer pattern methods are tracked across modules
    let functions: Vec<_> = call_graph.get_all_functions().collect();

    let update_id = functions
        .iter()
        .find(|f| f.name.contains("ConcreteObserver") && f.name.contains("update"))
        .expect("ConcreteObserver.update should be found");

    let handle_event_id = functions
        .iter()
        .find(|f| f.name.contains("handle_event"))
        .expect("handle_event should be found");

    // The update method should have callers (from notify)
    // The handle_event should have callers (from update)
    assert!(
        !call_graph.get_callers(update_id).is_empty(),
        "ConcreteObserver.update should be called through observer pattern"
    );
    assert!(
        !call_graph.get_callers(handle_event_id).is_empty(),
        "handle_event should be called from update"
    );
}

#[test]
fn test_framework_methods_recognition() {
    let temp_dir = TempDir::new().unwrap();

    // Create base_widget.py
    let base_path = temp_dir.path().join("base_widget.py");
    fs::write(
        &base_path,
        r#"
class BaseWidget:
    def __init__(self):
        self.initialized = False

    def OnInit(self):
        """Framework initialization method."""
        pass

    def setUp(self):
        """Test setup method."""
        pass
"#,
    )
    .unwrap();

    // Create custom_widget.py
    let custom_path = temp_dir.path().join("custom_widget.py");
    fs::write(
        &custom_path,
        r#"
from base_widget import BaseWidget

class CustomWidget(BaseWidget):
    def OnInit(self):
        """Override framework init."""
        super().OnInit()
        self.custom_init()

    def custom_init(self):
        """Custom initialization."""
        self.initialized = True

    def setUp(self):
        """Override test setup."""
        self.test_data = []
"#,
    )
    .unwrap();

    // Analyze the project
    let files = vec![base_path, custom_path];
    let call_graph = analyze_python_project(&files).unwrap();

    // Framework methods should be recognized as entry points
    let functions: Vec<_> = call_graph.get_all_functions().collect();

    let on_init_id = functions
        .iter()
        .find(|f| f.name.contains("CustomWidget") && f.name.contains("OnInit"))
        .expect("CustomWidget.OnInit should be found");

    let set_up_id = functions
        .iter()
        .find(|f| f.name.contains("CustomWidget") && f.name.contains("setUp"))
        .expect("CustomWidget.setUp should be found");

    let custom_init_id = functions
        .iter()
        .find(|f| f.name.contains("custom_init"))
        .expect("custom_init should be found");

    // Framework methods should be treated as entry points
    assert!(
        call_graph.is_entry_point(on_init_id),
        "OnInit should be recognized as an entry point"
    );
    assert!(
        call_graph.is_entry_point(set_up_id),
        "setUp should be recognized as an entry point"
    );

    // custom_init should have callers (from OnInit)
    assert!(
        !call_graph.get_callers(custom_init_id).is_empty(),
        "custom_init should be called from OnInit"
    );
}

#[test]
fn test_build_cross_module_context() {
    let temp_dir = TempDir::new().unwrap();

    // Create two simple modules
    let mod1_path = temp_dir.path().join("mod1.py");
    fs::write(
        &mod1_path,
        r#"
def func_a():
    return 1

class ClassA:
    def method_a(self):
        return 2
"#,
    )
    .unwrap();

    let mod2_path = temp_dir.path().join("mod2.py");
    fs::write(
        &mod2_path,
        r#"
from mod1 import func_a, ClassA

def func_b():
    return func_a() + ClassA().method_a()
"#,
    )
    .unwrap();

    // Build cross-module context
    let files = vec![mod1_path, mod2_path];
    let context = build_cross_module_context(&files).unwrap();

    // Check that symbols are registered
    assert!(
        !context.symbols.is_empty(),
        "Context should contain symbols"
    );

    // Check that imports are tracked
    assert!(!context.imports.is_empty(), "Context should track imports");

    // Check that exports are tracked
    assert!(!context.exports.is_empty(), "Context should track exports");
}
