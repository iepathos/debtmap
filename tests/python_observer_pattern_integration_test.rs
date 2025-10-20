//! Integration tests for observer pattern call graph detection
//!
//! Tests the complete flow from observer pattern recognition through call graph construction.

use debtmap::analysis::python_type_tracker::TwoPassExtractor;
use debtmap::priority::call_graph::CallType;
use rustpython_parser::parse;
use std::path::PathBuf;

#[test]
fn test_full_observer_pattern_call_graph() {
    let python_code = r#"
from abc import ABC, abstractmethod

class Observer(ABC):
    @abstractmethod
    def on_event(self):
        pass

class Subject:
    def __init__(self):
        self.observers = []

    def notify(self):
        for observer in self.observers:
            observer.on_event()

class ConcreteObserver(Observer):
    def on_event(self):
        print("Event received")
"#;

    let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let mut extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), python_code);
    let call_graph = extractor.extract(&module);

    // Find ConcreteObserver.on_event
    let on_event_functions: Vec<_> = call_graph
        .get_all_functions()
        .into_iter()
        .filter(|f| f.name.contains("ConcreteObserver.on_event"))
        .collect();

    assert!(
        !on_event_functions.is_empty(),
        "ConcreteObserver.on_event should be registered in call graph"
    );

    // Check that on_event has callers
    let on_event_id = &on_event_functions[0];
    let callers = call_graph.get_callers(on_event_id);

    assert!(
        !callers.is_empty(),
        "ConcreteObserver.on_event should have callers from Subject.notify"
    );

    // Verify the caller is Subject.notify
    let caller_names: Vec<_> = callers.iter().map(|c| c.name.as_str()).collect();
    assert!(
        caller_names
            .iter()
            .any(|name| name.contains("Subject.notify")),
        "Caller should be Subject.notify, got: {:?}",
        caller_names
    );
}

#[test]
fn test_observer_dispatch_call_type() {
    let python_code = r#"
from abc import ABC, abstractmethod

class Listener(ABC):
    @abstractmethod
    def on_change(self):
        pass

class Model:
    def __init__(self):
        self.listeners = []

    def update(self):
        for listener in self.listeners:
            listener.on_change()

class ViewListener(Listener):
    def on_change(self):
        pass
"#;

    let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let mut extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), python_code);
    let call_graph = extractor.extract(&module);

    // Find the call edge
    let view_listener_funcs: Vec<_> = call_graph
        .get_all_functions()
        .into_iter()
        .filter(|f| f.name.contains("ViewListener.on_change"))
        .collect();

    if let Some(on_change_id) = view_listener_funcs.first() {
        let all_calls = call_graph.get_all_calls();
        let incoming_edges: Vec<_> = all_calls
            .into_iter()
            .filter(|call| &call.callee == *on_change_id)
            .collect();

        // Check that at least one edge has CallType::ObserverDispatch
        let has_observer_dispatch = incoming_edges
            .iter()
            .any(|edge| matches!(edge.call_type, CallType::ObserverDispatch));

        assert!(
            has_observer_dispatch,
            "Should have at least one ObserverDispatch edge"
        );
    }
}

#[test]
fn test_multiple_observer_implementations() {
    let python_code = r#"
from abc import ABC, abstractmethod

class Handler(ABC):
    @abstractmethod
    def handle(self):
        pass

class EventSource:
    def __init__(self):
        self.handlers = []

    def fire_event(self):
        for handler in self.handlers:
            handler.handle()

class HandlerA(Handler):
    def handle(self):
        print("A")

class HandlerB(Handler):
    def handle(self):
        print("B")
"#;

    let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let mut extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), python_code);
    let call_graph = extractor.extract(&module);

    // Both HandlerA.handle and HandlerB.handle should have callers
    let handler_a: Vec<_> = call_graph
        .get_all_functions()
        .into_iter()
        .filter(|f| f.name.contains("HandlerA.handle"))
        .collect();

    let handler_b: Vec<_> = call_graph
        .get_all_functions()
        .into_iter()
        .filter(|f| f.name.contains("HandlerB.handle"))
        .collect();

    if let Some(handler_a_id) = handler_a.first() {
        let callers_a = call_graph.get_callers(handler_a_id);
        assert!(!callers_a.is_empty(), "HandlerA.handle should have callers");
    }

    if let Some(handler_b_id) = handler_b.first() {
        let callers_b = call_graph.get_callers(handler_b_id);
        assert!(!callers_b.is_empty(), "HandlerB.handle should have callers");
    }
}

#[test]
fn test_inline_observer_notification() {
    let python_code = r#"
from abc import ABC, abstractmethod

class Callback(ABC):
    @abstractmethod
    def on_complete(self):
        pass

class Task:
    def __init__(self):
        self.callbacks = []

    def run(self):
        # Do work
        result = 42
        # Inline notification
        for callback in self.callbacks:
            callback.on_complete()

class LogCallback(Callback):
    def on_complete(self):
        print("Done")
"#;

    let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let mut extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), python_code);
    let call_graph = extractor.extract(&module);

    // LogCallback.on_complete should have callers even though notification is inline
    let log_callback: Vec<_> = call_graph
        .get_all_functions()
        .into_iter()
        .filter(|f| f.name.contains("LogCallback.on_complete"))
        .collect();

    if let Some(on_complete_id) = log_callback.first() {
        let callers = call_graph.get_callers(on_complete_id);
        assert!(
            !callers.is_empty(),
            "LogCallback.on_complete should have callers from Task.run (inline notification)"
        );
    }
}

#[test]
fn test_no_false_positives_for_non_observer_loops() {
    let python_code = r#"
class DataProcessor:
    def __init__(self):
        self.items = []

    def process_all(self):
        for item in self.items:
            item.process()

class DataItem:
    def process(self):
        pass
"#;

    let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let mut extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), python_code);
    let call_graph = extractor.extract(&module);

    // DataItem.process should NOT have ObserverDispatch edges
    // (because "items" is not a recognized observer collection name)
    let data_item_funcs: Vec<_> = call_graph
        .get_all_functions()
        .into_iter()
        .filter(|f| f.name.contains("DataItem.process"))
        .collect();

    if let Some(process_id) = data_item_funcs.first() {
        let all_calls = call_graph.get_all_calls();
        let incoming_edges: Vec<_> = all_calls
            .into_iter()
            .filter(|call| &call.callee == *process_id)
            .collect();

        let has_observer_dispatch = incoming_edges
            .iter()
            .any(|edge| matches!(edge.call_type, CallType::ObserverDispatch));

        assert!(
            !has_observer_dispatch,
            "Should not have ObserverDispatch edges for non-observer collections"
        );
    }
}
