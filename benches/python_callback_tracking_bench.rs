//! Performance benchmarks for Python callback tracking
//!
//! Measures the overhead of callback tracking to verify < 10% impact requirement

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use debtmap::analysis::python_call_graph::TwoPassExtractor;
use rustpython_parser as rp;
use std::path::PathBuf;

/// Generate a large Python codebase with various callback patterns
fn generate_large_codebase(num_classes: usize, methods_per_class: usize) -> String {
    let mut code = String::new();

    code.push_str("import wx\nimport functools\n\n");

    for class_idx in 0..num_classes {
        code.push_str(&format!("class MyClass{}:\n", class_idx));

        // Add constructor
        code.push_str("    def __init__(self):\n");
        code.push_str("        self.value = 0\n\n");

        // Add regular methods
        for method_idx in 0..methods_per_class {
            code.push_str(&format!("    def method_{}(self):\n", method_idx));
            code.push_str("        self.value += 1\n");
            code.push_str(&format!(
                "        self.method_{}()\n",
                (method_idx + 1) % methods_per_class
            ));
            code.push_str("\n");
        }

        // Add event handlers
        code.push_str("    def on_click(self, event):\n");
        code.push_str("        self.method_0()\n\n");

        code.push_str("    def on_paint(self, event):\n");
        code.push_str("        self.method_1()\n\n");

        // Add setup method with callbacks
        code.push_str("    def setup(self):\n");
        code.push_str("        self.button.Bind(wx.EVT_BUTTON, self.on_click)\n");
        code.push_str("        self.panel.Bind(wx.EVT_PAINT, self.on_paint)\n");
        code.push_str("        callback = functools.partial(self.method_0, priority=10)\n");
        code.push_str("        wx.CallAfter(self.on_click, None)\n\n");
    }

    code
}

/// Benchmark callback tracking with default two-pass extractor
fn bench_with_callback_tracking(c: &mut Criterion) {
    let code = generate_large_codebase(10, 5);

    c.bench_function("callback_tracking_enabled", |b| {
        b.iter(|| {
            let module = rp::parse(&code, rp::Mode::Module, "test.py")
                .expect("Failed to parse code");
            let mut extractor =
                TwoPassExtractor::new_with_source(PathBuf::from("test.py"), &code);
            black_box(extractor.extract(&module))
        })
    });
}

/// Benchmark baseline performance without callback-intensive patterns
fn bench_baseline_no_callbacks(c: &mut Criterion) {
    // Generate code with no callback patterns
    let mut code = String::new();
    for class_idx in 0..10 {
        code.push_str(&format!("class MyClass{}:\n", class_idx));
        for method_idx in 0..5 {
            code.push_str(&format!("    def method_{}(self):\n", method_idx));
            code.push_str("        pass\n\n");
        }
    }

    c.bench_function("baseline_no_callbacks", |b| {
        b.iter(|| {
            let module = rp::parse(&code, rp::Mode::Module, "test.py")
                .expect("Failed to parse code");
            let mut extractor =
                TwoPassExtractor::new_with_source(PathBuf::from("test.py"), &code);
            black_box(extractor.extract(&module))
        })
    });
}

/// Benchmark with varying codebase sizes
fn bench_scalability(c: &mut Criterion) {
    let sizes = vec![
        ("small", 5, 3),
        ("medium", 20, 8),
        ("large", 50, 15),
    ];

    for (name, num_classes, methods_per_class) in sizes {
        let code = generate_large_codebase(num_classes, methods_per_class);

        c.bench_function(&format!("callback_tracking_{}", name), |b| {
            b.iter(|| {
                let module = rp::parse(&code, rp::Mode::Module, "test.py")
                    .expect("Failed to parse code");
                let mut extractor =
                    TwoPassExtractor::new_with_source(PathBuf::from("test.py"), &code);
                black_box(extractor.extract(&module))
            })
        });
    }
}

/// Benchmark callback resolution phase specifically
fn bench_callback_resolution(c: &mut Criterion) {
    let code = generate_large_codebase(20, 8);

    c.bench_function("callback_resolution_phase", |b| {
        b.iter(|| {
            let module = rp::parse(&code, rp::Mode::Module, "test.py")
                .expect("Failed to parse code");
            let mut extractor =
                TwoPassExtractor::new_with_source(PathBuf::from("test.py"), &code);

            // Extract call graph which includes both passes
            let call_graph = extractor.extract(&module);

            // The resolution happens in phase_two
            black_box(call_graph)
        })
    });
}

/// Benchmark with real-world pattern distribution
fn bench_realistic_patterns(c: &mut Criterion) {
    // Create a more realistic codebase with mixed patterns
    let code = r#"
import wx
import functools
from typing import Callable

class EventManager:
    def __init__(self):
        self.handlers = []
        self.callbacks = {}

    def register_handler(self, event_type: str, handler: Callable):
        self.callbacks[event_type] = handler

    def on_event_a(self, data):
        print(f"Event A: {data}")

    def on_event_b(self, data):
        print(f"Event B: {data}")

    def setup(self):
        self.register_handler("event_a", self.on_event_a)
        self.register_handler("event_b", self.on_event_b)

class UIComponent:
    def __init__(self):
        self.button = None
        self.panel = None

    def on_button_click(self, event):
        self.process_click()

    def on_panel_paint(self, event):
        self.draw_panel()

    def process_click(self):
        print("Click processed")

    def draw_panel(self):
        print("Panel drawn")

    def setup_ui(self):
        self.button.Bind(wx.EVT_BUTTON, self.on_button_click)
        self.panel.Bind(wx.EVT_PAINT, self.on_panel_paint)

class DataProcessor:
    def process_item(self, item_id, priority):
        print(f"Processing {item_id} with priority {priority}")

    def schedule_processing(self, item_id):
        callback = functools.partial(self.process_item, priority=10)
        self.executor.submit(callback, item_id)

    def batch_process(self, items):
        for item in items:
            self.schedule_processing(item)

class Application:
    def __init__(self):
        self.ui = UIComponent()
        self.processor = DataProcessor()
        self.events = EventManager()

    def run(self):
        self.ui.setup_ui()
        self.events.setup()
        items = range(100)
        self.processor.batch_process(items)
"#;

    c.bench_function("realistic_callback_patterns", |b| {
        b.iter(|| {
            let module = rp::parse(code, rp::Mode::Module, "test.py")
                .expect("Failed to parse code");
            let mut extractor =
                TwoPassExtractor::new_with_source(PathBuf::from("test.py"), code);
            black_box(extractor.extract(&module))
        })
    });
}

criterion_group!(
    callback_benches,
    bench_with_callback_tracking,
    bench_baseline_no_callbacks,
    bench_scalability,
    bench_callback_resolution,
    bench_realistic_patterns
);

criterion_main!(callback_benches);
