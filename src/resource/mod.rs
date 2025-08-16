/// Resource management pattern detection framework
use crate::core::{DebtItem, DebtType, Priority};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceManagementIssue {
    MissingDrop {
        type_name: String,
        resource_fields: Vec<ResourceField>,
        suggested_drop_impl: String,
        severity: ResourceSeverity,
    },
    ResourceLeak {
        resource_type: ResourceType,
        acquisition_site: SourceLocation,
        leak_site: SourceLocation,
        cleanup_suggestion: String,
    },
    AsyncResourceIssue {
        function_name: String,
        issue_type: AsyncResourceIssueType,
        cancellation_safety: CancellationSafety,
        mitigation_strategy: String,
    },
    UnboundedCollection {
        collection_name: String,
        collection_type: String,
        growth_pattern: GrowthPattern,
        bounding_strategy: BoundingStrategy,
    },
    RaiiViolation {
        violation_type: RaiiViolationType,
        resource_involved: String,
        correct_pattern: String,
    },
    HandleLeak {
        handle_type: HandleType,
        leak_location: SourceLocation,
        proper_cleanup: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceType {
    FileHandle,
    NetworkConnection,
    DatabaseConnection,
    MemoryAllocation,
    SystemHandle,
    ThreadHandle,
    Mutex,
    Channel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AsyncResourceIssueType {
    ResourceNotCleaned,
    CancellationUnsafe,
    SharedResourceRace,
    DropInAsync,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GrowthPattern {
    UnboundedInsertion,
    NoEviction,
    MemoryAccumulation,
    RecursiveGrowth,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BoundingStrategy {
    SizeLimit,
    TimeBasedEviction,
    LruEviction,
    CapacityCheck,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RaiiViolationType {
    ManualResourceManagement,
    ResourceNotInConstructor,
    NoResourceRelease,
    PartialCleanup,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HandleType {
    File,
    Socket,
    Process,
    Thread,
    Timer,
    Device,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceSeverity {
    Critical, // System resources that can cause crashes
    High,     // Important resources that affect performance
    Medium,   // Resources with moderate impact
    Low,      // Resources with minimal impact
}

#[derive(Debug, Clone, PartialEq)]
pub enum CancellationSafety {
    Safe,    // Resource properly cleaned up on cancellation
    Unsafe,  // Resource may leak on cancellation
    Unknown, // Cannot determine safety
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceField {
    pub field_name: String,
    pub field_type: String,
    pub is_owning: bool,
    pub cleanup_required: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

pub trait ResourceDetector {
    fn detect_issues(&self, file: &syn::File, path: &Path) -> Vec<ResourceManagementIssue>;
    fn detector_name(&self) -> &'static str;
    fn assess_resource_impact(&self, issue: &ResourceManagementIssue) -> ResourceImpact;
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceImpact {
    Critical, // Can cause system failure
    High,     // Significant resource waste
    Medium,   // Moderate resource impact
    Low,      // Minor resource inefficiency
}

// Re-export submodules
mod async_detector;
mod collection_detector;
mod drop_detector;

pub use async_detector::AsyncResourceDetector;
pub use collection_detector::UnboundedCollectionDetector;
pub use drop_detector::DropDetector;

/// Convert resource management issue to debt item
pub fn convert_resource_issue_to_debt_item(
    issue: ResourceManagementIssue,
    _impact: ResourceImpact,
    path: &Path,
) -> DebtItem {
    let line = get_line_from_issue(&issue);
    let (priority, message, context) = match issue {
        ResourceManagementIssue::MissingDrop {
            ref type_name,
            ref resource_fields,
            ref suggested_drop_impl,
            ..
        } => (
            Priority::High,
            format!(
                "Type '{}' holds {} resource(s) but lacks Drop implementation",
                type_name,
                resource_fields.len()
            ),
            Some(format!("Implement Drop:\n{}", suggested_drop_impl)),
        ),
        ResourceManagementIssue::ResourceLeak {
            ref resource_type,
            ref cleanup_suggestion,
            ..
        } => (
            Priority::Critical,
            format!("Potential {:?} leak detected", resource_type),
            Some(cleanup_suggestion.clone()),
        ),
        ResourceManagementIssue::AsyncResourceIssue {
            ref function_name,
            ref issue_type,
            ref mitigation_strategy,
            ..
        } => (
            Priority::High,
            format!(
                "Async resource issue in '{}': {:?}",
                function_name, issue_type
            ),
            Some(mitigation_strategy.clone()),
        ),
        ResourceManagementIssue::UnboundedCollection {
            ref collection_name,
            ref growth_pattern,
            ref bounding_strategy,
            ..
        } => (
            Priority::Medium,
            format!(
                "Collection '{}' has unbounded growth: {:?}",
                collection_name, growth_pattern
            ),
            Some(format!("Consider: {:?}", bounding_strategy)),
        ),
        ResourceManagementIssue::RaiiViolation {
            ref violation_type,
            ref resource_involved,
            ref correct_pattern,
        } => (
            Priority::Medium,
            format!(
                "RAII violation: {:?} for {}",
                violation_type, resource_involved
            ),
            Some(format!("Use pattern: {}", correct_pattern)),
        ),
        ResourceManagementIssue::HandleLeak {
            ref handle_type,
            ref proper_cleanup,
            ..
        } => (
            Priority::High,
            format!("{:?} handle not properly cleaned up", handle_type),
            Some(proper_cleanup.clone()),
        ),
    };

    DebtItem {
        id: format!("resource-{}-{}", path.display(), line),
        debt_type: DebtType::ResourceManagement,
        priority,
        file: path.to_path_buf(),
        line,
        message,
        context,
    }
}

fn get_line_from_issue(issue: &ResourceManagementIssue) -> usize {
    match issue {
        ResourceManagementIssue::ResourceLeak { leak_site, .. } => leak_site.line,
        ResourceManagementIssue::HandleLeak { leak_location, .. } => leak_location.line,
        _ => 0, // For issues without specific line numbers
    }
}
