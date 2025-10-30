//! Pattern matching helpers for semantic classification
//!
//! This module contains pure pattern matching functions that identify
//! functions based on name patterns.

/// Check if function name matches debug patterns
pub(crate) fn matches_debug_pattern(name: &str) -> bool {
    let name_lower = name.to_lowercase();

    // Specific debug prefixes
    let prefixes = ["debug_", "print_", "dump_", "trace_"];
    // Specific debug suffixes
    let suffixes = ["_diagnostics", "_debug", "_stats"];
    // Specific debug contains patterns
    let contains = ["diagnostics"];

    prefixes.iter().any(|p| name_lower.starts_with(p))
        || suffixes.iter().any(|s| name_lower.ends_with(s))
        || contains.iter().any(|c| name_lower.contains(c))
}

/// Check if name matches output-focused I/O patterns (not read/write operations)
pub(crate) fn matches_output_io_pattern(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    let output_patterns = ["print", "display", "show", "log", "trace", "dump"];

    output_patterns.iter().any(|p| name_lower.contains(p))
}

/// Check if name matches accessor patterns
pub(crate) fn matches_accessor_name(
    name: &str,
    config: &crate::config::AccessorDetectionConfig,
) -> bool {
    let name_lower = name.to_lowercase();

    // Single-word accessors
    if config.single_word_patterns.contains(&name_lower) {
        return true;
    }

    // Prefix patterns
    if config
        .prefix_patterns
        .iter()
        .any(|p| name_lower.starts_with(p))
    {
        return true;
    }

    false
}

pub(crate) fn is_entry_point_by_name(name: &str) -> bool {
    let entry_patterns = [
        "main", "run", "start", "init", "handle", "process", "execute", "serve", "listen",
    ];

    let name_lower = name.to_lowercase();
    entry_patterns
        .iter()
        .any(|pattern| name_lower.starts_with(pattern) || name_lower.ends_with(pattern))
}

pub(crate) fn is_orchestrator_by_name(name: &str) -> bool {
    let name_lower = name.to_lowercase();

    // Exclude common non-orchestration patterns first
    let exclude_patterns = [
        "print",
        "format",
        "create",
        "build",
        "extract",
        "parse",
        "new",
        "from",
        "to",
        "into",
        "write",
        "read",
        "display",
        "render",
        "emit",
        // Exclude adapter/wrapper patterns
        "adapt",
        "wrap",
        "convert",
        "transform",
        "translate",
        // Exclude functional patterns
        "map",
        "filter",
        "reduce",
        "fold",
        "collect",
        "apply",
        // Exclude single-purpose functions
        "get",
        "set",
        "find",
        "search",
        "check",
        "validate",
    ];

    for pattern in &exclude_patterns {
        if name_lower.starts_with(pattern) || name_lower.ends_with(pattern) {
            return false;
        }
    }

    // Check common orchestration patterns that override excludes
    // (These would have been in include_patterns config)
    let include_patterns = [
        "workflow_",
        "pipeline_",
        "process_",
        "orchestrate_",
        "coordinate_",
        "execute_flow_",
    ];
    for pattern in &include_patterns {
        if name_lower.starts_with(pattern) {
            return true;
        }
    }

    // Then check for true orchestration patterns
    let orchestrator_patterns = [
        "orchestrate",
        "coordinate",
        "manage",
        "dispatch",
        "route",
        "if_requested",
        "if_needed",
        "if_enabled",
        "maybe",
        "try_",
        "attempt_",
        "delegate",
        "forward",
    ];

    // Check for conditional patterns like generate_report_if_requested
    if name_lower.contains("_if_") || name_lower.contains("_when_") {
        return true;
    }

    orchestrator_patterns
        .iter()
        .any(|pattern| name_lower.contains(pattern))
}
