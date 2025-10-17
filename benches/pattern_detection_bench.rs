//! Performance benchmarks for pattern detection
//!
//! Measures the overhead of pattern detection to verify < 5% impact requirement

use criterion::{criterion_group, criterion_main, Criterion};
use debtmap::analysis::patterns::PatternDetector;
use debtmap::analyzers::{python::PythonAnalyzer, Analyzer};
use std::hint::black_box;
use std::path::PathBuf;

/// Generate Python code with various design patterns
fn generate_pattern_rich_code(num_classes: usize) -> String {
    let mut code = String::new();

    code.push_str("from abc import ABC, abstractmethod\n");
    code.push_str("from dataclasses import dataclass\n");
    code.push_str("from typing import List\n\n");

    // Observer pattern
    for i in 0..num_classes {
        code.push_str(&format!("class Observer{i}(ABC):\n"));
        code.push_str("    @abstractmethod\n");
        code.push_str("    def on_event(self, event: str) -> None:\n");
        code.push_str("        pass\n\n");

        code.push_str(&format!("@dataclass\nclass ConcreteObserver{i}(Observer{i}):\n"));
        code.push_str("    name: str\n\n");
        code.push_str("    def on_event(self, event: str) -> None:\n");
        code.push_str("        print(f'{self.name} received {event}')\n\n");

        code.push_str(&format!("class Subject{i}:\n"));
        code.push_str("    def __init__(self):\n");
        code.push_str("        self._observers = []\n\n");
        code.push_str("    def attach(self, observer):\n");
        code.push_str("        self._observers.append(observer)\n\n");
        code.push_str("    def notify(self):\n");
        code.push_str("        for observer in self._observers:\n");
        code.push_str("            observer.on_event('update')\n\n");

        // Factory pattern
        code.push_str(&format!("def create_observer_{i}(name: str):\n"));
        code.push_str(&format!("    return ConcreteObserver{i}(name=name)\n\n"));
    }

    code
}

/// Generate Python code with minimal patterns
fn generate_minimal_pattern_code(num_classes: usize) -> String {
    let mut code = String::new();

    for i in 0..num_classes {
        code.push_str(&format!("class SimpleClass{i}:\n"));
        code.push_str("    def __init__(self):\n");
        code.push_str("        self.value = 0\n\n");
        code.push_str("    def method(self):\n");
        code.push_str("        self.value += 1\n\n");
    }

    code
}

/// Benchmark pattern detection with pattern-rich code
fn bench_with_pattern_detection(c: &mut Criterion) {
    let code = generate_pattern_rich_code(10);
    let path = PathBuf::from("test.py");

    c.bench_function("pattern_detection_enabled", |b| {
        b.iter(|| {
            let analyzer = PythonAnalyzer::new();
            let ast = analyzer.parse(&code, path.clone()).expect("Parse failed");
            let metrics = analyzer.analyze(&ast);
            let detector = PatternDetector::new();
            black_box(detector.detect_all_patterns(&metrics))
        })
    });
}

/// Benchmark baseline analysis without pattern detection
fn bench_baseline_no_detection(c: &mut Criterion) {
    let code = generate_minimal_pattern_code(10);
    let path = PathBuf::from("test.py");

    c.bench_function("pattern_detection_baseline", |b| {
        b.iter(|| {
            let analyzer = PythonAnalyzer::new();
            let ast = analyzer.parse(&code, path.clone()).expect("Parse failed");
            black_box(analyzer.analyze(&ast))
        })
    });
}

/// Benchmark overhead calculation (with vs without pattern detection)
fn bench_overhead_comparison(c: &mut Criterion) {
    let code = generate_pattern_rich_code(20);
    let path = PathBuf::from("test.py");

    c.bench_function("analysis_only", |b| {
        b.iter(|| {
            let analyzer = PythonAnalyzer::new();
            let ast = analyzer.parse(&code, path.clone()).expect("Parse failed");
            black_box(analyzer.analyze(&ast))
        })
    });

    c.bench_function("analysis_with_patterns", |b| {
        b.iter(|| {
            let analyzer = PythonAnalyzer::new();
            let ast = analyzer.parse(&code, path.clone()).expect("Parse failed");
            let metrics = analyzer.analyze(&ast);
            let detector = PatternDetector::new();
            black_box(detector.detect_all_patterns(&metrics))
        })
    });
}

/// Benchmark scalability with varying codebase sizes
fn bench_scalability(c: &mut Criterion) {
    let sizes = vec![("small", 5), ("medium", 20), ("large", 50)];

    for (name, num_classes) in sizes {
        let code = generate_pattern_rich_code(num_classes);
        let path = PathBuf::from("test.py");

        c.bench_function(&format!("pattern_detection_{name}"), |b| {
            b.iter(|| {
                let analyzer = PythonAnalyzer::new();
                let ast = analyzer.parse(&code, path.clone()).expect("Parse failed");
                let metrics = analyzer.analyze(&ast);
                let detector = PatternDetector::new();
                black_box(detector.detect_all_patterns(&metrics))
            })
        });
    }
}

/// Benchmark with real observer.py fixture
fn bench_realistic_patterns(c: &mut Criterion) {
    let code = r#"
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import List

class Observer(ABC):
    @abstractmethod
    def on_event(self, event: str) -> None:
        pass

class Subject(ABC):
    def __init__(self):
        self._observers: List[Observer] = []

    def attach(self, observer: Observer) -> None:
        if observer not in self._observers:
            self._observers.append(observer)

    def notify(self) -> None:
        for observer in self._observers:
            observer.on_event('update')

@dataclass
class ConcreteObserver(Observer):
    name: str

    def on_event(self, event: str) -> None:
        print(f"{self.name} received event: {event}")

class EventManager(Subject):
    def __init__(self):
        super().__init__()
        self._event_queue: List[str] = []

    def notify(self) -> None:
        for event in self._event_queue:
            for observer in self._observers:
                observer.on_event(event)
        self._event_queue.clear()

def create_observer(name: str) -> ConcreteObserver:
    return ConcreteObserver(name=name)

class Configuration:
    def __init__(self):
        self.debug_mode = False

config = Configuration()
event_manager = EventManager()
"#;

    let path = PathBuf::from("observer.py");

    c.bench_function("realistic_observer_pattern", |b| {
        b.iter(|| {
            let analyzer = PythonAnalyzer::new();
            let ast = analyzer.parse(code, path.clone()).expect("Parse failed");
            let metrics = analyzer.analyze(&ast);
            let detector = PatternDetector::new();
            black_box(detector.detect_all_patterns(&metrics))
        })
    });
}

criterion_group!(
    pattern_benches,
    bench_with_pattern_detection,
    bench_baseline_no_detection,
    bench_overhead_comparison,
    bench_scalability,
    bench_realistic_patterns
);

criterion_main!(pattern_benches);
