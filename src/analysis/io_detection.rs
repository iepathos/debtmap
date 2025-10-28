//! I/O and Side Effect Detection Module
//!
//! This module provides static analysis to detect I/O operations and side effects
//! in code, enabling responsibility classification based on actual behavior rather
//! than naming conventions alone.
//!
//! # Supported Languages
//!
//! - Rust: std::fs, std::io, std::net, println!, env::var
//! - Python: open(), pathlib, requests, print(), os.environ
//! - JavaScript/TypeScript: fs, fetch, console, process.env
//!
//! # Example
//!
//! ```ignore
//! use debtmap::analysis::io_detection::{IoDetector, Language};
//!
//! let detector = IoDetector::new();
//! let profile = detector.analyze_function(&function_ast, Language::Rust);
//!
//! if profile.has_file_io() {
//!     println!("Function performs file I/O operations");
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a function
pub type FunctionId = String;

/// Language for I/O pattern detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
}

/// Main I/O detector
pub struct IoDetector {
    /// Language-specific I/O patterns
    patterns: HashMap<Language, IoPatternSet>,
}

impl IoDetector {
    /// Create a new I/O detector with default patterns
    pub fn new() -> Self {
        let mut patterns = HashMap::new();
        patterns.insert(Language::Rust, IoPatternSet::for_rust());
        patterns.insert(Language::Python, IoPatternSet::for_python());
        patterns.insert(Language::JavaScript, IoPatternSet::for_javascript());
        patterns.insert(Language::TypeScript, IoPatternSet::for_typescript());

        Self { patterns }
    }

    /// Detect I/O operations in a function
    pub fn detect_io(&self, code: &str, language: Language) -> IoProfile {
        let pattern_set = self
            .patterns
            .get(&language)
            .expect("Language not supported");

        let mut profile = IoProfile::new();

        // Detect file operations
        for pattern in &pattern_set.file_ops {
            if code.contains(pattern) {
                profile
                    .file_operations
                    .push(IoOperation::FileRead { path_expr: None });
            }
        }

        // Detect network operations
        for pattern in &pattern_set.network_ops {
            if code.contains(pattern) {
                profile
                    .network_operations
                    .push(IoOperation::NetworkRequest { endpoint: None });
            }
        }

        // Detect console operations
        for pattern in &pattern_set.console_ops {
            if code.contains(pattern) {
                profile.console_operations.push(IoOperation::ConsoleOutput {
                    stream: OutputStream::Stdout,
                });
            }
        }

        // Detect database operations
        for pattern in &pattern_set.db_ops {
            if code.contains(pattern) {
                profile
                    .database_operations
                    .push(IoOperation::DatabaseQuery {
                        query_type: QueryType::Select,
                    });
            }
        }

        // Detect environment variable access
        for pattern in &pattern_set.env_ops {
            if code.contains(pattern) {
                profile
                    .environment_operations
                    .push(IoOperation::EnvironmentAccess { var_name: None });
            }
        }

        // Update purity status
        profile.is_pure = profile.file_operations.is_empty()
            && profile.network_operations.is_empty()
            && profile.console_operations.is_empty()
            && profile.database_operations.is_empty()
            && profile.environment_operations.is_empty()
            && profile.side_effects.is_empty();

        profile
    }
}

impl Default for IoDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Set of I/O patterns for a language
#[derive(Debug, Clone)]
pub struct IoPatternSet {
    pub file_ops: Vec<String>,
    pub network_ops: Vec<String>,
    pub console_ops: Vec<String>,
    pub db_ops: Vec<String>,
    pub env_ops: Vec<String>,
}

impl IoPatternSet {
    /// Patterns for Rust
    pub fn for_rust() -> Self {
        Self {
            file_ops: vec![
                "std::fs::read".to_string(),
                "std::fs::write".to_string(),
                "std::fs::File::open".to_string(),
                "std::fs::File::create".to_string(),
                "std::fs::OpenOptions".to_string(),
                "std::fs::remove".to_string(),
                "std::fs::copy".to_string(),
                "std::fs::rename".to_string(),
                "fs::read".to_string(),
                "fs::write".to_string(),
                "File::open".to_string(),
                "File::create".to_string(),
                "read_to_string".to_string(),
                "write_all".to_string(),
            ],
            network_ops: vec![
                "reqwest::".to_string(),
                "hyper::".to_string(),
                "std::net::TcpStream".to_string(),
                "std::net::TcpListener".to_string(),
                "std::net::UdpSocket".to_string(),
                "TcpStream::connect".to_string(),
                "TcpListener::bind".to_string(),
            ],
            console_ops: vec![
                "println!".to_string(),
                "print!".to_string(),
                "eprintln!".to_string(),
                "eprint!".to_string(),
                "dbg!".to_string(),
            ],
            db_ops: vec![
                "diesel::".to_string(),
                "sqlx::".to_string(),
                "rusqlite::".to_string(),
                "execute".to_string(),
                "query".to_string(),
            ],
            env_ops: vec![
                "std::env::var".to_string(),
                "std::env::set_var".to_string(),
                "env::var".to_string(),
                "env::set_var".to_string(),
            ],
        }
    }

    /// Patterns for Python
    pub fn for_python() -> Self {
        Self {
            file_ops: vec![
                "open(".to_string(),
                "pathlib.Path".to_string(),
                ".read_text(".to_string(),
                ".write_text(".to_string(),
                ".read_bytes(".to_string(),
                ".write_bytes(".to_string(),
                "os.path.".to_string(),
                "shutil.".to_string(),
            ],
            network_ops: vec![
                "requests.".to_string(),
                "urllib.".to_string(),
                "http.client.".to_string(),
                "socket.".to_string(),
                "httpx.".to_string(),
            ],
            console_ops: vec![
                "print(".to_string(),
                "input(".to_string(),
                "sys.stdout.".to_string(),
                "sys.stderr.".to_string(),
            ],
            db_ops: vec![
                "sqlite3.".to_string(),
                "psycopg2.".to_string(),
                "pymongo.".to_string(),
                ".execute(".to_string(),
                ".fetchall(".to_string(),
                ".fetchone(".to_string(),
            ],
            env_ops: vec![
                "os.environ".to_string(),
                "os.getenv(".to_string(),
                "os.putenv(".to_string(),
            ],
        }
    }

    /// Patterns for JavaScript
    pub fn for_javascript() -> Self {
        Self {
            file_ops: vec![
                "fs.readFile".to_string(),
                "fs.writeFile".to_string(),
                "fs.readFileSync".to_string(),
                "fs.writeFileSync".to_string(),
                "fs.promises.".to_string(),
                "require('fs')".to_string(),
            ],
            network_ops: vec![
                "fetch(".to_string(),
                "axios.".to_string(),
                "XMLHttpRequest".to_string(),
                "http.request".to_string(),
                "https.request".to_string(),
            ],
            console_ops: vec![
                "console.log".to_string(),
                "console.error".to_string(),
                "console.warn".to_string(),
                "console.debug".to_string(),
            ],
            db_ops: vec![
                "mongoose.".to_string(),
                "sequelize.".to_string(),
                ".query(".to_string(),
                ".find(".to_string(),
                ".findOne(".to_string(),
            ],
            env_ops: vec!["process.env".to_string(), "process.env.".to_string()],
        }
    }

    /// Patterns for TypeScript (same as JavaScript)
    pub fn for_typescript() -> Self {
        Self::for_javascript()
    }
}

/// I/O operation types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IoOperation {
    FileRead { path_expr: Option<String> },
    FileWrite { path_expr: Option<String> },
    NetworkRequest { endpoint: Option<String> },
    ConsoleOutput { stream: OutputStream },
    DatabaseQuery { query_type: QueryType },
    EnvironmentAccess { var_name: Option<String> },
}

/// Output stream type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// Database query type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum QueryType {
    Select,
    Insert,
    Update,
    Delete,
}

/// Side effect types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SideEffect {
    /// Mutation of field in self or other object
    FieldMutation { target: String, field: String },
    /// Mutation of global/static variable
    GlobalMutation { name: String },
    /// Array/collection mutation
    CollectionMutation { operation: CollectionOp },
    /// External state change
    ExternalState { description: String },
}

/// Collection operation type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CollectionOp {
    Push,
    Pop,
    Insert,
    Remove,
    Clear,
}

/// I/O profile for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoProfile {
    pub file_operations: Vec<IoOperation>,
    pub network_operations: Vec<IoOperation>,
    pub console_operations: Vec<IoOperation>,
    pub database_operations: Vec<IoOperation>,
    pub environment_operations: Vec<IoOperation>,
    pub side_effects: Vec<SideEffect>,
    pub is_pure: bool,
}

impl IoProfile {
    /// Create a new empty I/O profile
    pub fn new() -> Self {
        Self {
            file_operations: Vec::new(),
            network_operations: Vec::new(),
            console_operations: Vec::new(),
            database_operations: Vec::new(),
            environment_operations: Vec::new(),
            side_effects: Vec::new(),
            is_pure: true,
        }
    }

    /// Check if function has file I/O
    pub fn has_file_io(&self) -> bool {
        !self.file_operations.is_empty()
    }

    /// Check if function has network I/O
    pub fn has_network_io(&self) -> bool {
        !self.network_operations.is_empty()
    }

    /// Check if function has console I/O
    pub fn has_console_io(&self) -> bool {
        !self.console_operations.is_empty()
    }

    /// Check if function has database I/O
    pub fn has_database_io(&self) -> bool {
        !self.database_operations.is_empty()
    }

    /// Classify responsibility based on I/O pattern
    pub fn primary_responsibility(&self) -> Responsibility {
        match (
            self.file_operations.is_empty(),
            self.network_operations.is_empty(),
            self.console_operations.is_empty(),
            self.database_operations.is_empty(),
            self.is_pure,
        ) {
            (false, _, _, _, _) => Responsibility::FileIO,
            (_, false, _, _, _) => Responsibility::NetworkIO,
            (_, _, false, _, _) => Responsibility::ConsoleIO,
            (_, _, _, false, _) => Responsibility::DatabaseIO,
            (true, true, true, true, true) => Responsibility::PureComputation,
            _ => Responsibility::MixedIO,
        }
    }

    /// I/O intensity score (higher = more I/O heavy)
    pub fn intensity(&self) -> f64 {
        (self.file_operations.len()
            + self.network_operations.len()
            + self.console_operations.len()
            + self.database_operations.len()
            + self.environment_operations.len()) as f64
    }

    /// Merge another profile into this one
    pub fn merge(&mut self, other: &IoProfile) {
        self.file_operations.extend(other.file_operations.clone());
        self.network_operations
            .extend(other.network_operations.clone());
        self.console_operations
            .extend(other.console_operations.clone());
        self.database_operations
            .extend(other.database_operations.clone());
        self.environment_operations
            .extend(other.environment_operations.clone());
        self.side_effects.extend(other.side_effects.clone());
        self.is_pure = self.is_pure && other.is_pure;
    }
}

impl Default for IoProfile {
    fn default() -> Self {
        Self::new()
    }
}

/// Responsibility classification based on I/O behavior
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Responsibility {
    PureComputation,
    FileIO,
    NetworkIO,
    ConsoleIO,
    DatabaseIO,
    MixedIO,
    SideEffects,
}

impl Responsibility {
    /// Convert to a human-readable string
    pub fn as_str(&self) -> &'static str {
        match self {
            Responsibility::PureComputation => "Pure Computation",
            Responsibility::FileIO => "File I/O",
            Responsibility::NetworkIO => "Network I/O",
            Responsibility::ConsoleIO => "Console I/O",
            Responsibility::DatabaseIO => "Database I/O",
            Responsibility::MixedIO => "Mixed I/O",
            Responsibility::SideEffects => "Side Effects",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_file_io_detection() {
        let code = r#"
        fn read_config() -> String {
            std::fs::read_to_string("config.toml").unwrap()
        }
        "#;

        let detector = IoDetector::new();
        let profile = detector.detect_io(code, Language::Rust);

        // Multiple patterns may match (std::fs::read, read_to_string, fs::read)
        assert!(!profile.file_operations.is_empty());
        assert_eq!(profile.primary_responsibility(), Responsibility::FileIO);
        assert!(!profile.is_pure);
    }

    #[test]
    fn test_rust_network_io_detection() {
        let code = r#"
        fn fetch_data() {
            let client = reqwest::blocking::Client::new();
            let response = client.get("https://api.example.com").send();
        }
        "#;

        let detector = IoDetector::new();
        let profile = detector.detect_io(code, Language::Rust);

        assert_eq!(profile.network_operations.len(), 1);
        assert_eq!(profile.primary_responsibility(), Responsibility::NetworkIO);
    }

    #[test]
    fn test_rust_console_io_detection() {
        let code = r#"
        fn log_message(msg: &str) {
            println!("Message: {}", msg);
        }
        "#;

        let detector = IoDetector::new();
        let profile = detector.detect_io(code, Language::Rust);

        assert_eq!(profile.console_operations.len(), 1);
        assert_eq!(profile.primary_responsibility(), Responsibility::ConsoleIO);
    }

    #[test]
    fn test_pure_function_detection() {
        let code = r#"
        fn calculate_sum(a: i32, b: i32) -> i32 {
            a + b
        }
        "#;

        let detector = IoDetector::new();
        let profile = detector.detect_io(code, Language::Rust);

        assert!(profile.is_pure);
        assert_eq!(
            profile.primary_responsibility(),
            Responsibility::PureComputation
        );
        assert_eq!(profile.intensity(), 0.0);
    }

    #[test]
    fn test_python_file_io_detection() {
        let code = r#"
        def read_config():
            with open('config.json') as f:
                return f.read()
        "#;

        let detector = IoDetector::new();
        let profile = detector.detect_io(code, Language::Python);

        assert_eq!(profile.file_operations.len(), 1);
        assert_eq!(profile.primary_responsibility(), Responsibility::FileIO);
    }

    #[test]
    fn test_python_network_io_detection() {
        let code = r#"
        def fetch_data():
            response = requests.get('https://api.example.com/data')
            return response.json()
        "#;

        let detector = IoDetector::new();
        let profile = detector.detect_io(code, Language::Python);

        assert_eq!(profile.network_operations.len(), 1);
        assert_eq!(profile.primary_responsibility(), Responsibility::NetworkIO);
    }

    #[test]
    fn test_javascript_file_io_detection() {
        let code = r#"
        function readConfig() {
            return fs.readFileSync('config.json', 'utf8');
        }
        "#;

        let detector = IoDetector::new();
        let profile = detector.detect_io(code, Language::JavaScript);

        // Multiple patterns may match (fs.readFile, fs.readFileSync)
        assert!(!profile.file_operations.is_empty());
        assert_eq!(profile.primary_responsibility(), Responsibility::FileIO);
    }

    #[test]
    fn test_javascript_network_io_detection() {
        let code = r#"
        async function fetchData() {
            const response = await fetch('https://api.example.com');
            return await response.json();
        }
        "#;

        let detector = IoDetector::new();
        let profile = detector.detect_io(code, Language::JavaScript);

        assert_eq!(profile.network_operations.len(), 1);
        assert_eq!(profile.primary_responsibility(), Responsibility::NetworkIO);
    }

    #[test]
    fn test_mixed_io_detection() {
        let code = r#"
        fn process_and_log() {
            let data = std::fs::read_to_string("input.txt").unwrap();
            println!("Processing: {}", data);
        }
        "#;

        let detector = IoDetector::new();
        let profile = detector.detect_io(code, Language::Rust);

        assert!(!profile.file_operations.is_empty());
        assert!(!profile.console_operations.is_empty());
        assert!(profile.intensity() > 1.0);
    }

    #[test]
    fn test_io_profile_merge() {
        let mut profile1 = IoProfile::new();
        profile1
            .file_operations
            .push(IoOperation::FileRead { path_expr: None });

        let mut profile2 = IoProfile::new();
        profile2
            .network_operations
            .push(IoOperation::NetworkRequest { endpoint: None });

        profile1.merge(&profile2);

        assert_eq!(profile1.file_operations.len(), 1);
        assert_eq!(profile1.network_operations.len(), 1);
    }

    #[test]
    fn test_responsibility_as_str() {
        assert_eq!(Responsibility::PureComputation.as_str(), "Pure Computation");
        assert_eq!(Responsibility::FileIO.as_str(), "File I/O");
        assert_eq!(Responsibility::NetworkIO.as_str(), "Network I/O");
        assert_eq!(Responsibility::ConsoleIO.as_str(), "Console I/O");
        assert_eq!(Responsibility::DatabaseIO.as_str(), "Database I/O");
        assert_eq!(Responsibility::MixedIO.as_str(), "Mixed I/O");
        assert_eq!(Responsibility::SideEffects.as_str(), "Side Effects");
    }
}
