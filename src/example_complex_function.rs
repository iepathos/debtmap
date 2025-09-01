// Example function with cyclomatic complexity of 11 that needs refactoring
// This demonstrates the refactoring approach for the debt item

// Refactored: Extract pure classification function using pattern matching
// Reduced cyclomatic complexity from 11 to 8 using pattern consolidation
pub fn classify_operation(operation: &str, context: &str) -> OperationType {
    // First check context-specific patterns
    if let Some(op_type) = classify_by_context(operation, context) {
        return op_type;
    }

    // Then check operation patterns
    classify_by_operation_pattern(operation)
}

// Pure function to classify based on context
fn classify_by_context(operation: &str, context: &str) -> Option<OperationType> {
    match context {
        "async" if operation.contains("await") => Some(OperationType::Async),
        "batch" if operation.contains("bulk") => Some(OperationType::Batch),
        _ => None,
    }
}

// Pure function to classify based on operation pattern
// Complexity reduced through pattern consolidation
fn classify_by_operation_pattern(operation: &str) -> OperationType {
    // Use match with guards for pattern consolidation
    // Check more specific patterns first (bulk before delete)
    match () {
        _ if operation.ends_with("_all") || operation.ends_with("_many") => OperationType::Bulk,
        _ if operation.starts_with("get_") => OperationType::Read,
        _ if operation.starts_with("set_") => OperationType::Write,
        _ if operation.starts_with("delete_") || operation.starts_with("remove_") => {
            OperationType::Delete
        }
        _ if operation.starts_with("create_") || operation.starts_with("new_") => {
            OperationType::Create
        }
        _ if operation.starts_with("update_") || operation.starts_with("modify_") => {
            OperationType::Update
        }
        _ if operation.contains("validate") || operation.contains("check") => {
            OperationType::Validate
        }
        _ if operation.contains("_") => OperationType::Composite,
        _ => OperationType::Unknown,
    }
}

#[derive(Debug, PartialEq)]
pub enum OperationType {
    Read,
    Write,
    Delete,
    Create,
    Update,
    Validate,
    Async,
    Batch,
    Bulk,
    Composite,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_operations() {
        assert_eq!(classify_operation("get_user", "sync"), OperationType::Read);
        assert_eq!(
            classify_operation("get_data", "normal"),
            OperationType::Read
        );
    }

    #[test]
    fn test_write_operations() {
        assert_eq!(
            classify_operation("set_value", "sync"),
            OperationType::Write
        );
        assert_eq!(
            classify_operation("set_config", "normal"),
            OperationType::Write
        );
    }

    #[test]
    fn test_delete_operations() {
        assert_eq!(
            classify_operation("delete_item", "sync"),
            OperationType::Delete
        );
        assert_eq!(
            classify_operation("remove_user", "normal"),
            OperationType::Delete
        );
    }

    #[test]
    fn test_create_operations() {
        assert_eq!(
            classify_operation("create_user", "sync"),
            OperationType::Create
        );
        assert_eq!(
            classify_operation("new_instance", "normal"),
            OperationType::Create
        );
    }

    #[test]
    fn test_update_operations() {
        assert_eq!(
            classify_operation("update_profile", "sync"),
            OperationType::Update
        );
        assert_eq!(
            classify_operation("modify_settings", "normal"),
            OperationType::Update
        );
    }

    #[test]
    fn test_validate_operations() {
        assert_eq!(
            classify_operation("validate_input", "sync"),
            OperationType::Validate
        );
        assert_eq!(
            classify_operation("check_status", "normal"),
            OperationType::Validate
        );
        assert_eq!(
            classify_operation("is_valid_check", "sync"),
            OperationType::Validate
        );
    }

    #[test]
    fn test_async_operations() {
        assert_eq!(
            classify_operation("await_response", "async"),
            OperationType::Async
        );
        assert_eq!(
            classify_operation("do_await", "async"),
            OperationType::Async
        );
        // Non-async context should not trigger async classification
        assert_ne!(
            classify_operation("await_response", "sync"),
            OperationType::Async
        );
    }

    #[test]
    fn test_batch_operations() {
        assert_eq!(
            classify_operation("bulk_insert", "batch"),
            OperationType::Batch
        );
        assert_eq!(
            classify_operation("bulk_update", "batch"),
            OperationType::Batch
        );
        // Non-batch context should not trigger batch classification
        assert_ne!(
            classify_operation("bulk_insert", "sync"),
            OperationType::Batch
        );
    }

    #[test]
    fn test_bulk_operations() {
        assert_eq!(
            classify_operation("delete_all", "sync"),
            OperationType::Bulk
        );
        assert_eq!(
            classify_operation("update_many", "normal"),
            OperationType::Bulk
        );
    }

    #[test]
    fn test_composite_operations() {
        assert_eq!(
            classify_operation("process_and_save", "sync"),
            OperationType::Composite
        );
        assert_eq!(
            classify_operation("fetch_data", "normal"),
            OperationType::Composite
        );
        // Should not classify as composite if it matches other patterns first
        assert_ne!(
            classify_operation("get_user", "sync"),
            OperationType::Composite
        );
    }

    #[test]
    fn test_unknown_operations() {
        assert_eq!(
            classify_operation("dosomething", "sync"),
            OperationType::Unknown
        );
        assert_eq!(
            classify_operation("execute", "normal"),
            OperationType::Unknown
        );
    }

    #[test]
    fn test_context_priority() {
        // Context-based classification should take priority
        assert_eq!(
            classify_operation("get_await", "async"),
            OperationType::Async
        );
        assert_eq!(
            classify_operation("set_bulk", "batch"),
            OperationType::Batch
        );
    }

    #[test]
    fn test_helper_functions() {
        // Test classify_by_context directly
        assert_eq!(
            classify_by_context("await_data", "async"),
            Some(OperationType::Async)
        );
        assert_eq!(
            classify_by_context("bulk_process", "batch"),
            Some(OperationType::Batch)
        );
        assert_eq!(classify_by_context("anything", "normal"), None);

        // Test classify_by_operation_pattern directly
        assert_eq!(
            classify_by_operation_pattern("get_user"),
            OperationType::Read
        );
        assert_eq!(
            classify_by_operation_pattern("set_value"),
            OperationType::Write
        );
        assert_eq!(
            classify_by_operation_pattern("unknown"),
            OperationType::Unknown
        );
    }
}
