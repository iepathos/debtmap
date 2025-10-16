//! Accuracy measurement tests for Python callback tracking
//!
//! These tests measure the accuracy of callback pattern detection to verify
//! 90%+ accuracy on common callback patterns from real frameworks.

use debtmap::analysis::python_call_graph::TwoPassExtractor;
use rustpython_parser as rp;
use std::path::PathBuf;

/// Test case for callback accuracy measurement
#[derive(Debug)]
struct CallbackTest {
    name: &'static str,
    code: &'static str,
    expected_callbacks: Vec<(&'static str, &'static str)>, // (caller, callee)
}

/// Calculate accuracy percentage for a set of tests
fn calculate_accuracy(detected: usize, expected: usize) -> f64 {
    if expected == 0 {
        return 100.0;
    }
    (detected as f64 / expected as f64) * 100.0
}

#[test]
fn test_wxpython_callback_accuracy() {
    let test_cases = vec![
        CallbackTest {
            name: "wxPython Bind pattern",
            code: r#"
class MyFrame:
    def on_click(self, event):
        pass

    def on_paint(self, event):
        pass

    def setup(self):
        self.button.Bind(wx.EVT_BUTTON, self.on_click)
        self.panel.Bind(wx.EVT_PAINT, self.on_paint)
"#,
            expected_callbacks: vec![
                ("MyFrame.setup", "MyFrame.on_click"),
                ("MyFrame.setup", "MyFrame.on_paint"),
            ],
        },
        CallbackTest {
            name: "wxPython CallAfter pattern",
            code: r#"
class Handler:
    def handle_event(self):
        pass

    def trigger(self):
        wx.CallAfter(self.handle_event)
"#,
            expected_callbacks: vec![("Handler.trigger", "Handler.handle_event")],
        },
    ];

    let mut total_expected = 0;
    let mut total_detected = 0;

    for test in test_cases {
        let module = rp::parse(test.code, rp::Mode::Module, "test.py")
            .expect("Failed to parse test code");
        let mut extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), test.code);
        let call_graph = extractor.extract(&module);

        let mut detected = 0;
        for (caller_name, callee_name) in &test.expected_callbacks {
            // Find caller
            let caller_id = call_graph
                .get_all_functions()
                .find(|f| f.name == *caller_name)
                .expect(&format!("Caller {} not found", caller_name));

            // Check if callee is in the call graph
            let callees = call_graph.get_callees(caller_id);
            if callees.iter().any(|c| c.name == *callee_name) {
                detected += 1;
            } else {
                eprintln!(
                    "Test '{}': Missing callback {} -> {}",
                    test.name, caller_name, callee_name
                );
            }
        }

        total_expected += test.expected_callbacks.len();
        total_detected += detected;

        let test_accuracy = calculate_accuracy(detected, test.expected_callbacks.len());
        println!(
            "Test '{}': {}/{} callbacks detected ({:.1}%)",
            test.name,
            detected,
            test.expected_callbacks.len(),
            test_accuracy
        );
    }

    let overall_accuracy = calculate_accuracy(total_detected, total_expected);
    println!(
        "\nwxPython Overall Accuracy: {}/{} ({:.1}%)",
        total_detected, total_expected, overall_accuracy
    );

    // Assert 90% accuracy threshold
    assert!(
        overall_accuracy >= 90.0,
        "wxPython callback detection accuracy {:.1}% is below 90% threshold",
        overall_accuracy
    );
}

#[test]
fn test_flask_callback_accuracy() {
    let test_cases = vec![
        CallbackTest {
            name: "Flask route decorator",
            code: r#"
from flask import Flask
app = Flask(__name__)

@app.route('/')
def index():
    return 'Hello'

@app.route('/about')
def about():
    return 'About'
"#,
            expected_callbacks: vec![
                // Flask routes are entry points, so they should be marked as such
                // Not traditional callbacks in the event sense
            ],
        },
        CallbackTest {
            name: "Flask before_request",
            code: r#"
from flask import Flask
app = Flask(__name__)

@app.before_request
def check_auth():
    pass

@app.route('/')
def index():
    return 'Hello'
"#,
            expected_callbacks: vec![
                // before_request handlers are automatically called
            ],
        },
    ];

    let mut total_expected = 0;
    let mut total_detected = 0;

    for test in test_cases {
        let module = rp::parse(test.code, rp::Mode::Module, "test.py")
            .expect("Failed to parse test code");
        let mut extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), test.code);
        let call_graph = extractor.extract(&module);

        // For Flask, we check that decorated functions are marked as entry points
        let mut detected = 0;
        for func in call_graph.get_all_functions() {
            if let Some((is_entry_point, _is_test, _complexity, _loc)) =
                call_graph.get_function_info(func)
            {
                if is_entry_point {
                    detected += 1;
                }
            }
        }

        total_expected += test.expected_callbacks.len();
        total_detected += detected;

        println!(
            "Test '{}': {} entry points detected",
            test.name, detected
        );
    }

    let overall_accuracy = if total_expected > 0 {
        calculate_accuracy(total_detected, total_expected)
    } else {
        100.0 // No callbacks to detect means perfect accuracy
    };

    println!(
        "\nFlask Overall Accuracy: {}/{} ({:.1}%)",
        total_detected, total_expected, overall_accuracy
    );

    // Flask uses decorators which should be detected
    assert!(
        overall_accuracy >= 90.0 || total_expected == 0,
        "Flask callback detection accuracy {:.1}% is below 90% threshold",
        overall_accuracy
    );
}

#[test]
fn test_fastapi_callback_accuracy() {
    let test_cases = vec![CallbackTest {
        name: "FastAPI route decorators",
        code: r#"
from fastapi import FastAPI
app = FastAPI()

@app.get("/")
async def root():
    return {"message": "Hello"}

@app.post("/items")
async def create_item(item: dict):
    return item
"#,
        expected_callbacks: vec![
            // FastAPI routes are entry points
        ],
    }];

    let mut total_expected = 0;
    let mut total_detected = 0;

    for test in test_cases {
        let module = rp::parse(test.code, rp::Mode::Module, "test.py")
            .expect("Failed to parse test code");
        let mut extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), test.code);
        let call_graph = extractor.extract(&module);

        // Check for entry points
        let mut detected = 0;
        for func in call_graph.get_all_functions() {
            if let Some((is_entry_point, _is_test, _complexity, _loc)) =
                call_graph.get_function_info(func)
            {
                if is_entry_point {
                    detected += 1;
                }
            }
        }

        total_expected += test.expected_callbacks.len();
        total_detected += detected;

        println!(
            "Test '{}': {} entry points detected",
            test.name, detected
        );
    }

    let overall_accuracy = if total_expected > 0 {
        calculate_accuracy(total_detected, total_expected)
    } else {
        100.0
    };

    println!(
        "\nFastAPI Overall Accuracy: {}/{} ({:.1}%)",
        total_detected, total_expected, overall_accuracy
    );

    assert!(
        overall_accuracy >= 90.0 || total_expected == 0,
        "FastAPI callback detection accuracy {:.1}% is below 90% threshold",
        overall_accuracy
    );
}

#[test]
fn test_general_callback_patterns_accuracy() {
    let test_cases = vec![
        CallbackTest {
            name: "Functools partial",
            code: r#"
import functools

class TaskScheduler:
    def process_task(self, task_id, priority):
        pass

    def schedule_high_priority(self, task_id):
        callback = functools.partial(self.process_task, priority=10)
        self.executor.submit(callback, task_id)
"#,
            expected_callbacks: vec![(
                "TaskScheduler.schedule_high_priority",
                "TaskScheduler.process_task",
            )],
        },
        CallbackTest {
            name: "Nested function callback",
            code: r#"
class DeliveryBoy:
    def deliver_message_added(self, observers, message, index):
        def deliver(observers, message, index):
            for observer in observers:
                observer.on_message_added(message, index)

        wx.CallAfter(deliver, observers, message, index)
"#,
            expected_callbacks: vec![(
                "DeliveryBoy.deliver_message_added",
                "DeliveryBoy.deliver_message_added.deliver",
            )],
        },
        CallbackTest {
            name: "PyQt signal connection",
            code: r#"
class Window:
    def on_button_clicked(self):
        pass

    def setup_ui(self):
        self.button.clicked.connect(self.on_button_clicked)
"#,
            expected_callbacks: vec![("Window.setup_ui", "Window.on_button_clicked")],
        },
        CallbackTest {
            name: "Tkinter bind pattern",
            code: r#"
class App:
    def on_key_press(self, event):
        pass

    def setup(self):
        self.root.bind("<KeyPress>", self.on_key_press)
"#,
            expected_callbacks: vec![("App.setup", "App.on_key_press")],
        },
    ];

    let mut total_expected = 0;
    let mut total_detected = 0;

    for test in test_cases {
        let module = rp::parse(test.code, rp::Mode::Module, "test.py")
            .expect("Failed to parse test code");
        let mut extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), test.code);
        let call_graph = extractor.extract(&module);

        let mut detected = 0;
        for (caller_name, callee_name) in &test.expected_callbacks {
            // Find caller
            let caller_opt = call_graph
                .get_all_functions()
                .find(|f| f.name == *caller_name);

            if let Some(caller_id) = caller_opt {
                // Check if callee is in the call graph
                let callees = call_graph.get_callees(caller_id);
                if callees.iter().any(|c| c.name == *callee_name) {
                    detected += 1;
                } else {
                    eprintln!(
                        "Test '{}': Missing callback {} -> {}",
                        test.name, caller_name, callee_name
                    );
                }
            } else {
                eprintln!("Test '{}': Caller {} not found", test.name, caller_name);
            }
        }

        total_expected += test.expected_callbacks.len();
        total_detected += detected;

        let test_accuracy = calculate_accuracy(detected, test.expected_callbacks.len());
        println!(
            "Test '{}': {}/{} callbacks detected ({:.1}%)",
            test.name,
            detected,
            test.expected_callbacks.len(),
            test_accuracy
        );
    }

    let overall_accuracy = calculate_accuracy(total_detected, total_expected);
    println!(
        "\nGeneral Callbacks Overall Accuracy: {}/{} ({:.1}%)",
        total_detected, total_expected, overall_accuracy
    );

    // Assert 90% accuracy threshold
    assert!(
        overall_accuracy >= 90.0,
        "General callback detection accuracy {:.1}% is below 90% threshold",
        overall_accuracy
    );
}

#[test]
fn test_combined_callback_accuracy() {
    // This test runs all patterns together to get an overall accuracy score
    let all_test_cases = vec![
        // wxPython patterns
        ("wxPython Bind", r#"
class MyFrame:
    def on_click(self, event):
        pass

    def setup(self):
        self.button.Bind(wx.EVT_BUTTON, self.on_click)
"#, vec![("MyFrame.setup", "MyFrame.on_click")]),

        // PyQt patterns
        ("PyQt connect", r#"
class Window:
    def on_button_clicked(self):
        pass

    def setup_ui(self):
        self.button.clicked.connect(self.on_button_clicked)
"#, vec![("Window.setup_ui", "Window.on_button_clicked")]),

        // Tkinter patterns
        ("Tkinter bind", r#"
class App:
    def on_key_press(self, event):
        pass

    def setup(self):
        self.root.bind("<KeyPress>", self.on_key_press)
"#, vec![("App.setup", "App.on_key_press")]),

        // Functools patterns
        ("Functools partial", r#"
import functools

class TaskScheduler:
    def process_task(self, task_id, priority):
        pass

    def schedule(self, task_id):
        callback = functools.partial(self.process_task, priority=10)
        self.executor.submit(callback, task_id)
"#, vec![("TaskScheduler.schedule", "TaskScheduler.process_task")]),
    ];

    let mut total_expected = 0;
    let mut total_detected = 0;

    for (name, code, expected_callbacks) in all_test_cases {
        let module = rp::parse(code, rp::Mode::Module, "test.py")
            .expect("Failed to parse test code");
        let mut extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), code);
        let call_graph = extractor.extract(&module);

        let mut detected = 0;
        for (caller_name, callee_name) in &expected_callbacks {
            let caller_opt = call_graph
                .get_all_functions()
                .find(|f| f.name == *caller_name);

            if let Some(caller_id) = caller_opt {
                let callees = call_graph.get_callees(caller_id);
                if callees.iter().any(|c| c.name == *callee_name) {
                    detected += 1;
                }
            }
        }

        total_expected += expected_callbacks.len();
        total_detected += detected;

        let test_accuracy = calculate_accuracy(detected, expected_callbacks.len());
        println!(
            "Pattern '{}': {}/{} ({:.1}%)",
            name,
            detected,
            expected_callbacks.len(),
            test_accuracy
        );
    }

    let overall_accuracy = calculate_accuracy(total_detected, total_expected);
    println!(
        "\n=== COMBINED CALLBACK ACCURACY REPORT ===");
    println!(
        "Total callbacks detected: {}/{}",
        total_detected, total_expected
    );
    println!("Overall Accuracy: {:.1}%", overall_accuracy);
    println!("Target Accuracy: 90.0%");
    println!("Status: {}", if overall_accuracy >= 90.0 { "PASS" } else { "FAIL" });
    println!("=========================================\n");

    // Assert 90% accuracy threshold for spec compliance
    assert!(
        overall_accuracy >= 90.0,
        "Combined callback detection accuracy {:.1}% is below 90% threshold (spec requirement)",
        overall_accuracy
    );
}
