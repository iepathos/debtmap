/// Python-specific resource management pattern detection
use crate::core::{DebtItem, DebtType, Priority};
use rustpython_parser::ast;
// Collections imports removed - not needed in this module
use std::path::Path;

mod async_resource;
mod circular_ref;
mod context_manager;
mod resource_tracker;
mod unbounded_collection;

pub use async_resource::PythonAsyncResourceDetector;
pub use circular_ref::PythonCircularRefDetector;
pub use context_manager::PythonContextManagerDetector;
pub use resource_tracker::PythonResourceTracker;
pub use unbounded_collection::PythonUnboundedCollectionDetector;

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceIssue {
    pub issue_type: PythonResourceIssueType,
    pub severity: ResourceSeverity,
    pub location: ResourceLocation,
    pub suggestion: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PythonResourceIssueType {
    MissingContextManager {
        resource_type: String,
        variable_name: String,
    },
    UnclosedResource {
        resource_type: String,
        variable_name: String,
    },
    CircularReference {
        classes_involved: Vec<String>,
        pattern: CircularPattern,
    },
    UnboundedCollection {
        collection_name: String,
        growth_pattern: GrowthPattern,
    },
    AsyncResourceLeak {
        function_name: String,
        resource_type: String,
    },
    ThreadOrProcessLeak {
        resource_type: String,
        name: String,
    },
    MissingCleanup {
        class_name: String,
        resources: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceSeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceLocation {
    pub line: usize,
    pub column: usize,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircularPattern {
    SelfReference,
    MutualReference,
    ChainReference(usize), // chain length
    CallbackLoop,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GrowthPattern {
    UnboundedAppend,
    NoEviction,
    RecursiveAccumulation,
    MemoryCache,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceImpact {
    pub impact_level: ImpactLevel,
    pub affected_scope: AffectedScope,
    pub estimated_severity: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImpactLevel {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AffectedScope {
    Module,
    Class,
    Function,
    Global,
}

pub trait PythonResourceDetector {
    fn detect_issues(&self, module: &ast::Mod, path: &Path) -> Vec<ResourceIssue>;
    fn assess_resource_impact(&self, issue: &ResourceIssue) -> ResourceImpact;
}

pub struct PythonResourceAnalyzer {
    context_manager_detector: PythonContextManagerDetector,
    circular_ref_detector: PythonCircularRefDetector,
    unbounded_collection_detector: PythonUnboundedCollectionDetector,
    async_resource_detector: PythonAsyncResourceDetector,
    resource_tracker: PythonResourceTracker,
}

impl PythonResourceAnalyzer {
    pub fn new() -> Self {
        Self {
            context_manager_detector: PythonContextManagerDetector::new(),
            circular_ref_detector: PythonCircularRefDetector::new(),
            unbounded_collection_detector: PythonUnboundedCollectionDetector::new(),
            async_resource_detector: PythonAsyncResourceDetector::new(),
            resource_tracker: PythonResourceTracker::new(),
        }
    }

    pub fn analyze(&self, module: &ast::Mod, path: &Path) -> Vec<DebtItem> {
        let mut all_issues = Vec::new();

        // Collect issues from all detectors
        all_issues.extend(self.context_manager_detector.detect_issues(module, path));
        all_issues.extend(self.circular_ref_detector.detect_issues(module, path));
        all_issues.extend(
            self.unbounded_collection_detector
                .detect_issues(module, path),
        );
        all_issues.extend(self.async_resource_detector.detect_issues(module, path));
        all_issues.extend(self.resource_tracker.detect_issues(module, path));

        // Convert to debt items
        all_issues
            .into_iter()
            .map(|issue| self.convert_to_debt_item(issue, path))
            .collect()
    }

    fn convert_to_debt_item(&self, issue: ResourceIssue, path: &Path) -> DebtItem {
        let (priority, message, context) = match issue.issue_type {
            PythonResourceIssueType::MissingContextManager {
                resource_type,
                variable_name,
            } => (
                Priority::High,
                format!(
                    "Resource '{}' of type '{}' not using context manager",
                    variable_name, resource_type
                ),
                Some(issue.suggestion),
            ),
            PythonResourceIssueType::UnclosedResource {
                resource_type,
                variable_name,
            } => (
                Priority::Critical,
                format!("Unclosed {} resource '{}'", resource_type, variable_name),
                Some(issue.suggestion),
            ),
            PythonResourceIssueType::CircularReference {
                classes_involved,
                pattern,
            } => (
                Priority::High,
                format!(
                    "Circular reference detected: {:?} involving {:?}",
                    pattern, classes_involved
                ),
                Some(issue.suggestion),
            ),
            PythonResourceIssueType::UnboundedCollection {
                collection_name,
                growth_pattern,
            } => (
                Priority::Medium,
                format!(
                    "Unbounded collection '{}' with pattern {:?}",
                    collection_name, growth_pattern
                ),
                Some(issue.suggestion),
            ),
            PythonResourceIssueType::AsyncResourceLeak {
                function_name,
                resource_type,
            } => (
                Priority::High,
                format!(
                    "Async resource leak in '{}' for {}",
                    function_name, resource_type
                ),
                Some(issue.suggestion),
            ),
            PythonResourceIssueType::ThreadOrProcessLeak {
                resource_type,
                name,
            } => (
                Priority::Critical,
                format!(
                    "{} '{}' may not be properly cleaned up",
                    resource_type, name
                ),
                Some(issue.suggestion),
            ),
            PythonResourceIssueType::MissingCleanup {
                class_name,
                resources,
            } => (
                Priority::Medium,
                format!(
                    "Class '{}' manages resources but lacks cleanup: {:?}",
                    class_name, resources
                ),
                Some(issue.suggestion),
            ),
        };

        DebtItem {
            id: format!("py-resource-{}-{}", path.display(), issue.location.line),
            debt_type: DebtType::ResourceManagement,
            priority,
            file: path.to_path_buf(),
            line: issue.location.line,
            column: Some(issue.location.column),
            message,
            context,
        }
    }
}

impl Default for PythonResourceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
