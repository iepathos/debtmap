/// Unit tests for Python resource management pattern detection (spec 73)
use debtmap::resource::python::{
    PythonAsyncResourceDetector, PythonCircularRefDetector, PythonContextManagerDetector,
    PythonResourceAnalyzer, PythonResourceDetector, PythonResourceIssueType, PythonResourceTracker,
    PythonUnboundedCollectionDetector, ResourceSeverity,
};
use rustpython_parser::parse;
use std::path::Path;

#[test]
fn test_context_manager_detection_missing() {
    let code = r#"
def process_file():
    f = open('data.txt', 'r')
    content = f.read()
    return content
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let detector = PythonContextManagerDetector::new();
    let issues = detector.detect_issues(&module, Path::new("test.py"));

    assert_eq!(issues.len(), 1);
    match &issues[0].issue_type {
        PythonResourceIssueType::MissingContextManager {
            resource_type,
            variable_name,
        } => {
            assert_eq!(resource_type, "file");
            assert_eq!(variable_name, "f");
        }
        _ => panic!("Expected MissingContextManager issue"),
    }
    assert_eq!(issues[0].severity, ResourceSeverity::High);
}

#[test]
fn test_context_manager_detection_proper_usage() {
    let code = r#"
def process_file():
    with open('data.txt', 'r') as f:
        content = f.read()
        return content
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let detector = PythonContextManagerDetector::new();
    let issues = detector.detect_issues(&module, Path::new("test.py"));

    assert_eq!(issues.len(), 0);
}

#[test]
fn test_socket_resource_detection() {
    let code = r#"
import socket

def connect_to_server():
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect(('localhost', 8080))
    data = s.recv(1024)
    return data
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let detector = PythonContextManagerDetector::new();
    let issues = detector.detect_issues(&module, Path::new("test.py"));

    assert_eq!(issues.len(), 1);
    match &issues[0].issue_type {
        PythonResourceIssueType::MissingContextManager {
            resource_type,
            variable_name,
        } => {
            assert_eq!(resource_type, "socket");
            assert_eq!(variable_name, "s");
        }
        _ => panic!("Expected MissingContextManager issue"),
    }
}

#[test]
fn test_circular_reference_self_ref() {
    let code = r#"
class Node:
    def __init__(self):
        self.parent = None
        self.children = []
    
    def add_child(self, child):
        child.parent = self
        self.children.append(child)
        child.children.append(self)  # Circular reference
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let detector = PythonCircularRefDetector::new();
    let issues = detector.detect_issues(&module, Path::new("test.py"));

    assert!(!issues.is_empty());
    let has_circular_ref = issues.iter().any(|issue| {
        matches!(
            &issue.issue_type,
            PythonResourceIssueType::CircularReference { .. }
        )
    });
    assert!(has_circular_ref, "Should detect circular reference pattern");
}

#[test]
fn test_unbounded_collection_detection() {
    let code = r#"
class Cache:
    def __init__(self):
        self.data = {}
    
    def add(self, key, value):
        self.data[key] = value  # No size limit or eviction
    
    def get(self, key):
        return self.data.get(key)
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let detector = PythonUnboundedCollectionDetector::new();
    let issues = detector.detect_issues(&module, Path::new("test.py"));

    assert!(!issues.is_empty());
    let has_unbounded = issues.iter().any(|issue| {
        matches!(
            &issue.issue_type,
            PythonResourceIssueType::UnboundedCollection { .. }
        )
    });
    assert!(has_unbounded, "Should detect unbounded collection");
}

#[test]
fn test_async_resource_leak_detection() {
    let code = r#"
import asyncio
import aiohttp

async def fetch_data(url):
    session = aiohttp.ClientSession()
    response = await session.get(url)
    data = await response.text()
    return data  # Session not closed
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let detector = PythonAsyncResourceDetector::new();
    let issues = detector.detect_issues(&module, Path::new("test.py"));

    assert!(!issues.is_empty());
    let has_async_leak = issues.iter().any(|issue| {
        matches!(
            &issue.issue_type,
            PythonResourceIssueType::AsyncResourceLeak { .. }
        )
    });
    assert!(has_async_leak, "Should detect async resource leak");
}

#[test]
fn test_thread_resource_leak() {
    let code = r#"
import threading

def background_task():
    while True:
        process_data()

def start_worker():
    t = threading.Thread(target=background_task)
    t.start()
    # Thread never joined or stopped
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let tracker = PythonResourceTracker::new();
    let issues = tracker.detect_issues(&module, Path::new("test.py"));

    let has_thread_leak = issues.iter().any(|issue| {
        matches!(
            &issue.issue_type,
            PythonResourceIssueType::ThreadOrProcessLeak { .. }
        )
    });
    assert!(has_thread_leak, "Should detect thread resource leak");
}

#[test]
fn test_process_resource_leak() {
    let code = r#"
import multiprocessing

def worker():
    while True:
        process_data()

def start_workers():
    for i in range(10):
        p = multiprocessing.Process(target=worker)
        p.start()
        # Process never joined or terminated
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let tracker = PythonResourceTracker::new();
    let issues = tracker.detect_issues(&module, Path::new("test.py"));

    let has_process_leak = issues.iter().any(|issue| {
        matches!(
            &issue.issue_type,
            PythonResourceIssueType::ThreadOrProcessLeak { .. }
        )
    });
    assert!(has_process_leak, "Should detect process resource leak");
}

#[test]
fn test_missing_cleanup_detection() {
    let code = r#"
class DatabaseManager:
    def __init__(self):
        self.connection = connect_to_db()
        self.cache = {}
        self.threads = []
    
    def add_worker(self):
        t = threading.Thread(target=self.work)
        t.start()
        self.threads.append(t)
    
    # No __del__ or cleanup method
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let tracker = PythonResourceTracker::new();
    let issues = tracker.detect_issues(&module, Path::new("test.py"));

    let has_missing_cleanup = issues.iter().any(|issue| {
        matches!(
            &issue.issue_type,
            PythonResourceIssueType::MissingCleanup { .. }
        )
    });
    assert!(has_missing_cleanup, "Should detect missing cleanup method");
}

#[test]
fn test_resource_analyzer_integration() {
    let code = r#"
class ResourceHeavyClass:
    def __init__(self):
        self.file = open('data.txt')
        self.socket = socket.socket()
        self.lock = threading.Lock()
        self.cache = {}  # Unbounded
        
    def process(self):
        with self.lock:
            self.cache[id] = data  # Grows unbounded
            
    def circular_ref(self):
        self.ref = self  # Circular
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let analyzer = PythonResourceAnalyzer::new();
    let debt_items = analyzer.analyze(&module, Path::new("test.py"));

    // Should detect multiple issues
    assert!(
        debt_items.len() >= 3,
        "Should detect multiple resource issues"
    );

    // Verify different issue types are detected
    let has_context_manager_issue = debt_items
        .iter()
        .any(|item| item.message.contains("context manager") || item.message.contains("Unclosed"));
    let has_unbounded_issue = debt_items
        .iter()
        .any(|item| item.message.contains("Unbounded") || item.message.contains("unbounded"));

    assert!(
        has_context_manager_issue,
        "Should detect context manager issues"
    );
    assert!(
        has_unbounded_issue,
        "Should detect unbounded collection issues"
    );
}

#[test]
fn test_resource_severity_levels() {
    let code = r#"
def critical_issue():
    conn = database.connect()  # Critical - database connection
    return conn.execute(query)

def high_issue():
    f = open('important.txt')  # High - file resource
    return f.read()

def medium_issue():
    cache = {}
    for item in large_list:
        cache[item.id] = item  # Medium - unbounded growth
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let analyzer = PythonResourceAnalyzer::new();
    let debt_items = analyzer.analyze(&module, Path::new("test.py"));

    // Verify different severity levels
    let has_critical = debt_items
        .iter()
        .any(|item| matches!(item.priority, debtmap::core::Priority::Critical));
    let has_high = debt_items
        .iter()
        .any(|item| matches!(item.priority, debtmap::core::Priority::High));

    assert!(
        has_high || has_critical,
        "Should detect high priority issues"
    );
}

#[test]
fn test_lock_resource_detection() {
    let code = r#"
import threading

def unsafe_operation():
    lock = threading.Lock()
    lock.acquire()
    # Do work
    # Never released - missing lock.release()
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let detector = PythonContextManagerDetector::new();
    let issues = detector.detect_issues(&module, Path::new("test.py"));

    assert!(!issues.is_empty(), "Should detect unreleased lock");
}

#[test]
fn test_connection_pool_detection() {
    let code = r#"
from sqlalchemy import create_engine

def setup_database():
    engine = create_engine('postgresql://...')
    pool = engine.pool
    # Pool never closed
    return pool
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let tracker = PythonResourceTracker::new();
    let issues = tracker.detect_issues(&module, Path::new("test.py"));

    let has_pool_issue = issues.iter().any(|issue| match &issue.issue_type {
        PythonResourceIssueType::UnclosedResource { resource_type, .. }
        | PythonResourceIssueType::MissingContextManager { resource_type, .. } => {
            resource_type.contains("pool") || resource_type.contains("Pool")
        }
        _ => false,
    });

    assert!(has_pool_issue, "Should detect unclosed connection pool");
}

#[test]
fn test_temporary_file_detection() {
    let code = r#"
import tempfile

def process_temp():
    temp = tempfile.NamedTemporaryFile(delete=False)
    temp.write(b'data')
    # File handle not closed, file not deleted
    return temp.name
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let detector = PythonContextManagerDetector::new();
    let issues = detector.detect_issues(&module, Path::new("test.py"));

    assert!(!issues.is_empty(), "Should detect unclosed temporary file");
}

#[test]
fn test_nested_resource_detection() {
    let code = r#"
def nested_resources():
    file1 = open('file1.txt')
    try:
        file2 = open('file2.txt')
        try:
            file3 = open('file3.txt')
            # All three files opened but not properly closed
            return file1.read() + file2.read() + file3.read()
        except:
            pass
    except:
        pass
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let detector = PythonContextManagerDetector::new();
    let issues = detector.detect_issues(&module, Path::new("test.py"));

    assert!(
        issues.len() >= 2,
        "Should detect multiple unclosed resources"
    );
}

#[test]
fn test_generator_resource_leak() {
    let code = r#"
def file_generator():
    for filename in filenames:
        f = open(filename)
        yield f.read()
        # File never closed in generator
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let detector = PythonContextManagerDetector::new();
    let issues = detector.detect_issues(&module, Path::new("test.py"));

    assert!(
        !issues.is_empty(),
        "Should detect resource leak in generator"
    );
}

#[test]
fn test_class_with_proper_cleanup() {
    let code = r#"
class ProperResourceManager:
    def __init__(self):
        self.resources = []
        
    def __enter__(self):
        return self
        
    def __exit__(self, exc_type, exc_val, exc_tb):
        for resource in self.resources:
            resource.close()
            
    def __del__(self):
        self.cleanup()
        
    def cleanup(self):
        # Proper cleanup implementation
        pass
"#;

    let module = parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let tracker = PythonResourceTracker::new();
    let issues = tracker.detect_issues(&module, Path::new("test.py"));

    let has_cleanup_issue = issues.iter().any(|issue| {
        matches!(
            &issue.issue_type,
            PythonResourceIssueType::MissingCleanup { .. }
        )
    });

    assert!(
        !has_cleanup_issue,
        "Should not flag class with proper cleanup"
    );
}
