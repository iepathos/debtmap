//! Type definitions for Python exception flow analysis

use crate::priority::call_graph::FunctionId;
use std::collections::{HashMap, HashSet};

/// Information about a raised exception
#[derive(Debug, Clone)]
pub(super) struct ExceptionInfo {
    pub exception_type: ExceptionType,
    pub is_documented: bool,
    #[allow(dead_code)]
    pub context_message: Option<String>,
    #[allow(dead_code)]
    pub source_exception: Option<Box<ExceptionInfo>>,
}

/// Type of exception
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExceptionType {
    Builtin(BuiltinException),
    Custom(String),
    Variable(String),
    Unknown,
}

impl ExceptionType {
    pub(super) fn from_name(name: &str, custom_exceptions: &HashMap<String, ExceptionClass>) -> Self {
        if let Ok(builtin) = name.parse::<BuiltinException>() {
            ExceptionType::Builtin(builtin)
        } else if custom_exceptions.contains_key(name) {
            ExceptionType::Custom(name.to_string())
        } else {
            ExceptionType::Variable(name.to_string())
        }
    }

    pub(super) fn name(&self) -> String {
        match self {
            ExceptionType::Builtin(b) => b.as_str().to_string(),
            ExceptionType::Custom(s) | ExceptionType::Variable(s) => s.clone(),
            ExceptionType::Unknown => "Unknown".to_string(),
        }
    }

    pub(super) fn is_broad(&self) -> bool {
        matches!(self, ExceptionType::Builtin(BuiltinException::Exception))
    }

    pub(super) fn is_base_exception(&self) -> bool {
        matches!(
            self,
            ExceptionType::Builtin(BuiltinException::BaseException)
        )
    }

    pub(super) fn is_subclass_of(&self, parent: &str) -> bool {
        let child_name = self.name();

        // Exact match
        if child_name == parent {
            return true;
        }

        // Recursively check built-in hierarchy
        let mut current = child_name.clone();
        while let Some(parent_type) = find_parent_exception(&current) {
            if parent_type == parent {
                return true;
            }
            current = parent_type;
        }

        false
    }
}

/// Built-in Python exceptions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BuiltinException {
    BaseException,
    Exception,
    ValueError,
    TypeError,
    KeyError,
    AttributeError,
    IndexError,
    RuntimeError,
    NotImplementedError,
    IOError,
    OSError,
    FileNotFoundError,
    ImportError,
    ModuleNotFoundError,
    NameError,
    AssertionError,
    ZeroDivisionError,
    StopIteration,
    KeyboardInterrupt,
    SystemExit,
}

impl BuiltinException {
    pub(super) fn as_str(&self) -> &str {
        match self {
            Self::BaseException => "BaseException",
            Self::Exception => "Exception",
            Self::ValueError => "ValueError",
            Self::TypeError => "TypeError",
            Self::KeyError => "KeyError",
            Self::AttributeError => "AttributeError",
            Self::IndexError => "IndexError",
            Self::RuntimeError => "RuntimeError",
            Self::NotImplementedError => "NotImplementedError",
            Self::IOError => "IOError",
            Self::OSError => "OSError",
            Self::FileNotFoundError => "FileNotFoundError",
            Self::ImportError => "ImportError",
            Self::ModuleNotFoundError => "ModuleNotFoundError",
            Self::NameError => "NameError",
            Self::AssertionError => "AssertionError",
            Self::ZeroDivisionError => "ZeroDivisionError",
            Self::StopIteration => "StopIteration",
            Self::KeyboardInterrupt => "KeyboardInterrupt",
            Self::SystemExit => "SystemExit",
        }
    }
}

impl std::str::FromStr for BuiltinException {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BaseException" => Ok(Self::BaseException),
            "Exception" => Ok(Self::Exception),
            "ValueError" => Ok(Self::ValueError),
            "TypeError" => Ok(Self::TypeError),
            "KeyError" => Ok(Self::KeyError),
            "AttributeError" => Ok(Self::AttributeError),
            "IndexError" => Ok(Self::IndexError),
            "RuntimeError" => Ok(Self::RuntimeError),
            "NotImplementedError" => Ok(Self::NotImplementedError),
            "IOError" => Ok(Self::IOError),
            "OSError" => Ok(Self::OSError),
            "FileNotFoundError" => Ok(Self::FileNotFoundError),
            "ImportError" => Ok(Self::ImportError),
            "ModuleNotFoundError" => Ok(Self::ModuleNotFoundError),
            "NameError" => Ok(Self::NameError),
            "AssertionError" => Ok(Self::AssertionError),
            "ZeroDivisionError" => Ok(Self::ZeroDivisionError),
            "StopIteration" => Ok(Self::StopIteration),
            "KeyboardInterrupt" => Ok(Self::KeyboardInterrupt),
            "SystemExit" => Ok(Self::SystemExit),
            _ => Err(()),
        }
    }
}

/// List of built-in exceptions for quick lookup
pub(super) const BUILTIN_EXCEPTIONS: &[&str] = &[
    "BaseException",
    "Exception",
    "ValueError",
    "TypeError",
    "KeyError",
    "AttributeError",
    "IndexError",
    "RuntimeError",
    "NotImplementedError",
    "IOError",
    "OSError",
    "FileNotFoundError",
    "ImportError",
    "ModuleNotFoundError",
    "NameError",
    "AssertionError",
    "ZeroDivisionError",
    "StopIteration",
    "KeyboardInterrupt",
    "SystemExit",
];

/// Built-in exception hierarchy: child -> parent
const BUILTIN_EXCEPTION_HIERARCHY: &[(&str, &str)] = &[
    // BaseException is the root
    ("Exception", "BaseException"),
    ("SystemExit", "BaseException"),
    ("KeyboardInterrupt", "BaseException"),
    ("GeneratorExit", "BaseException"),
    // Exception hierarchy
    ("StopIteration", "Exception"),
    ("ArithmeticError", "Exception"),
    ("AssertionError", "Exception"),
    ("AttributeError", "Exception"),
    ("BufferError", "Exception"),
    ("EOFError", "Exception"),
    ("ImportError", "Exception"),
    ("LookupError", "Exception"),
    ("MemoryError", "Exception"),
    ("NameError", "Exception"),
    ("OSError", "Exception"),
    ("ReferenceError", "Exception"),
    ("RuntimeError", "Exception"),
    ("SyntaxError", "Exception"),
    ("SystemError", "Exception"),
    ("TypeError", "Exception"),
    ("ValueError", "Exception"),
    ("Warning", "Exception"),
    // ArithmeticError subclasses
    ("FloatingPointError", "ArithmeticError"),
    ("OverflowError", "ArithmeticError"),
    ("ZeroDivisionError", "ArithmeticError"),
    // ImportError subclasses
    ("ModuleNotFoundError", "ImportError"),
    // LookupError subclasses
    ("IndexError", "LookupError"),
    ("KeyError", "LookupError"),
    // OSError subclasses (and IOError alias)
    ("IOError", "OSError"),
    ("FileNotFoundError", "OSError"),
    ("FileExistsError", "OSError"),
    ("PermissionError", "OSError"),
    ("TimeoutError", "OSError"),
    // NameError subclasses
    ("UnboundLocalError", "NameError"),
    // RuntimeError subclasses
    ("NotImplementedError", "RuntimeError"),
    ("RecursionError", "RuntimeError"),
];

/// Find the parent exception type for a given exception name
pub(super) fn find_parent_exception(exception_name: &str) -> Option<String> {
    BUILTIN_EXCEPTION_HIERARCHY
        .iter()
        .find(|(child, _)| *child == exception_name)
        .map(|(_, parent)| parent.to_string())
}

/// Exception flow for a function
#[derive(Debug)]
pub(super) struct ExceptionFlow {
    #[allow(dead_code)]
    pub function_name: String,
    pub raised_exceptions: Vec<ExceptionInfo>,
    pub caught_exceptions: Vec<CaughtException>,
    pub transformed_exceptions: Vec<ExceptionTransformation>,
    pub documented_exceptions: Vec<DocumentedException>,
}

impl ExceptionFlow {
    pub(super) fn new(function_name: String) -> Self {
        Self {
            function_name,
            raised_exceptions: Vec::new(),
            caught_exceptions: Vec::new(),
            transformed_exceptions: Vec::new(),
            documented_exceptions: Vec::new(),
        }
    }
}

/// A caught exception
#[derive(Debug)]
pub(super) struct CaughtException {
    pub exception_types: Vec<ExceptionType>,
    #[allow(dead_code)]
    pub handler_type: HandlerType,
    pub is_bare_except: bool,
    pub is_overly_broad: bool,
    pub handler_action: HandlerAction,
}

/// Type of exception handler
#[derive(Debug, Clone, Copy)]
pub(super) enum HandlerType {
    Specific,
    Multiple,
    Broad,
    Bare,
    BaseException,
}

/// Action taken in exception handler
#[derive(Debug)]
pub(super) enum HandlerAction {
    Reraise,
    Transform,
    Log,
    Ignore,
    Handle,
}

/// Exception transformation (catch one, raise another)
#[derive(Debug)]
pub(crate) struct ExceptionTransformation {
    #[allow(dead_code)]
    pub caught_type: ExceptionType,
    pub raised_type: ExceptionType,
    pub preserves_context: bool,
}

/// Custom exception class
#[derive(Debug)]
pub(super) struct ExceptionClass {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub base_classes: Vec<String>,
    #[allow(dead_code)]
    pub docstring: Option<String>,
}

/// Documented exception from docstring
#[derive(Debug, Clone)]
pub(super) struct DocumentedException {
    pub exception_type: String,
    #[allow(dead_code)]
    pub description: String,
}

/// Exception flow pattern detected
#[derive(Debug)]
pub struct ExceptionFlowPattern {
    pub(super) pattern_type: ExceptionPatternType,
    pub(super) severity: Severity,
    pub(super) confidence: f32,
    pub(super) function_name: String,
    #[allow(dead_code)]
    pub(super) exception_type: Option<String>,
    pub(super) explanation: String,
    pub(super) suggestion: String,
}

/// Type of exception pattern
#[derive(Debug)]
pub(super) enum ExceptionPatternType {
    BareExcept,
    OverlyBroadHandler,
    ExceptionSwallowing,
    UndocumentedException,
    ExceptionNotRaised,
    TransformationLost,
    LogAndIgnore,
}

impl ExceptionPatternType {
    pub(super) fn as_str(&self) -> &str {
        match self {
            Self::BareExcept => "bare-except",
            Self::OverlyBroadHandler => "overly-broad",
            Self::ExceptionSwallowing => "swallowing",
            Self::UndocumentedException => "undocumented",
            Self::ExceptionNotRaised => "not-raised",
            Self::TransformationLost => "lost-context",
            Self::LogAndIgnore => "log-ignore",
        }
    }
}

/// Severity of pattern
#[derive(Debug)]
pub(super) enum Severity {
    High,
    Medium,
    Low,
}

/// Exception propagation graph
#[derive(Debug)]
pub struct ExceptionGraph {
    /// Exception information for each function
    pub function_exceptions: HashMap<FunctionId, FunctionExceptions>,
    /// Propagation edges: caller -> (callee, exception_type)
    pub propagation_edges: HashMap<FunctionId, HashSet<(FunctionId, ExceptionType)>>,
}

impl ExceptionGraph {
    pub(super) fn new() -> Self {
        Self {
            function_exceptions: HashMap::new(),
            propagation_edges: HashMap::new(),
        }
    }

    /// Get all exceptions that may propagate to a function through its callees
    pub fn get_propagating_exceptions(&self, func_id: &FunctionId) -> Vec<ExceptionType> {
        self.propagation_edges
            .get(func_id)
            .map(|edges| edges.iter().map(|(_, exc)| exc.clone()).collect())
            .unwrap_or_default()
    }
}

/// Exception information for a function
#[derive(Debug, Clone)]
pub struct FunctionExceptions {
    /// Exceptions raised directly in this function
    pub raised: Vec<ExceptionType>,
    /// Exceptions caught in this function
    pub caught: Vec<ExceptionType>,
    /// Exceptions that propagate to callers (raised but not caught)
    pub propagates: Vec<ExceptionType>,
    /// Exceptions documented in docstring
    pub documented: Vec<String>,
}

impl FunctionExceptions {
    #[allow(dead_code)]
    pub(super) fn new() -> Self {
        Self {
            raised: Vec::new(),
            caught: Vec::new(),
            propagates: Vec::new(),
            documented: Vec::new(),
        }
    }
}
