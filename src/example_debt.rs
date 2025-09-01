// Example module with technical debt for demonstration
use std::collections::HashMap;

/// Refactored function with reduced complexity using functional patterns
pub fn classify_operation(operation: &str, context: &HashMap<String, String>) -> OperationType {
    // Extract classification logic into pure functions
    match () {
        _ if operation.starts_with("create_") => classify_create_operation(context),
        _ if operation.starts_with("delete_") => classify_delete_operation(context),
        _ if operation.starts_with("update_") => classify_update_operation(context),
        _ => classify_by_operation_pattern(operation),
    }
}

/// Pure function to classify create operations based on context
fn classify_create_operation(context: &HashMap<String, String>) -> OperationType {
    match () {
        _ if context.contains_key("admin") => OperationType::AdminCreate,
        _ if context.contains_key("user") => OperationType::UserCreate,
        _ => OperationType::DefaultCreate,
    }
}

/// Pure function to classify delete operations based on context
fn classify_delete_operation(context: &HashMap<String, String>) -> OperationType {
    match () {
        _ if context.contains_key("force") => OperationType::ForceDelete,
        _ if context.contains_key("soft") => OperationType::SoftDelete,
        _ => OperationType::NormalDelete,
    }
}

/// Pure function to classify update operations based on context
fn classify_update_operation(context: &HashMap<String, String>) -> OperationType {
    match () {
        _ if context.contains_key("partial") => OperationType::PartialUpdate,
        _ if context.contains_key("full") => OperationType::FullUpdate,
        _ => OperationType::DefaultUpdate,
    }
}

/// Pure function to classify operations by pattern matching
fn classify_by_operation_pattern(operation: &str) -> OperationType {
    match () {
        _ if operation.contains("async") || operation.contains("await") => {
            OperationType::AsyncOperation
        }
        _ if operation.starts_with("handle_") => OperationType::Handler,
        _ if operation.starts_with("process_") => OperationType::Processor,
        _ if operation.starts_with("validate_") => OperationType::Validator,
        _ if operation.starts_with("transform_") => OperationType::Transformer,
        _ => OperationType::Unknown,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperationType {
    AdminCreate,
    UserCreate,
    DefaultCreate,
    ForceDelete,
    SoftDelete,
    NormalDelete,
    PartialUpdate,
    FullUpdate,
    DefaultUpdate,
    AsyncOperation,
    Handler,
    Processor,
    Validator,
    Transformer,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_operation_classification() {
        let mut context = HashMap::new();

        // Test default create
        assert_eq!(
            classify_operation("create_user", &context),
            OperationType::DefaultCreate
        );

        // Test admin create
        context.insert("admin".to_string(), "true".to_string());
        assert_eq!(
            classify_operation("create_user", &context),
            OperationType::AdminCreate
        );

        // Test user create
        context.clear();
        context.insert("user".to_string(), "true".to_string());
        assert_eq!(
            classify_operation("create_resource", &context),
            OperationType::UserCreate
        );
    }

    #[test]
    fn test_delete_operation_classification() {
        let mut context = HashMap::new();

        // Test normal delete
        assert_eq!(
            classify_operation("delete_item", &context),
            OperationType::NormalDelete
        );

        // Test force delete
        context.insert("force".to_string(), "true".to_string());
        assert_eq!(
            classify_operation("delete_item", &context),
            OperationType::ForceDelete
        );

        // Test soft delete
        context.clear();
        context.insert("soft".to_string(), "true".to_string());
        assert_eq!(
            classify_operation("delete_item", &context),
            OperationType::SoftDelete
        );
    }

    #[test]
    fn test_update_operation_classification() {
        let mut context = HashMap::new();

        // Test default update
        assert_eq!(
            classify_operation("update_record", &context),
            OperationType::DefaultUpdate
        );

        // Test partial update
        context.insert("partial".to_string(), "true".to_string());
        assert_eq!(
            classify_operation("update_record", &context),
            OperationType::PartialUpdate
        );

        // Test full update
        context.clear();
        context.insert("full".to_string(), "true".to_string());
        assert_eq!(
            classify_operation("update_record", &context),
            OperationType::FullUpdate
        );
    }

    #[test]
    fn test_pattern_based_classification() {
        let context = HashMap::new();

        // Test async operations
        assert_eq!(
            classify_operation("async_fetch", &context),
            OperationType::AsyncOperation
        );
        assert_eq!(
            classify_operation("await_result", &context),
            OperationType::AsyncOperation
        );

        // Test handler
        assert_eq!(
            classify_operation("handle_request", &context),
            OperationType::Handler
        );

        // Test processor
        assert_eq!(
            classify_operation("process_data", &context),
            OperationType::Processor
        );

        // Test validator
        assert_eq!(
            classify_operation("validate_input", &context),
            OperationType::Validator
        );

        // Test transformer
        assert_eq!(
            classify_operation("transform_data", &context),
            OperationType::Transformer
        );

        // Test unknown
        assert_eq!(
            classify_operation("random_operation", &context),
            OperationType::Unknown
        );
    }

    #[test]
    fn test_pure_helper_functions() {
        let mut context = HashMap::new();

        // Test classify_create_operation
        assert_eq!(
            classify_create_operation(&context),
            OperationType::DefaultCreate
        );
        context.insert("admin".to_string(), "true".to_string());
        assert_eq!(
            classify_create_operation(&context),
            OperationType::AdminCreate
        );

        // Test classify_delete_operation
        context.clear();
        assert_eq!(
            classify_delete_operation(&context),
            OperationType::NormalDelete
        );
        context.insert("force".to_string(), "true".to_string());
        assert_eq!(
            classify_delete_operation(&context),
            OperationType::ForceDelete
        );

        // Test classify_update_operation
        context.clear();
        assert_eq!(
            classify_update_operation(&context),
            OperationType::DefaultUpdate
        );
        context.insert("partial".to_string(), "true".to_string());
        assert_eq!(
            classify_update_operation(&context),
            OperationType::PartialUpdate
        );

        // Test classify_by_operation_pattern
        assert_eq!(
            classify_by_operation_pattern("async_operation"),
            OperationType::AsyncOperation
        );
        assert_eq!(
            classify_by_operation_pattern("handle_event"),
            OperationType::Handler
        );
        assert_eq!(
            classify_by_operation_pattern("unknown"),
            OperationType::Unknown
        );
    }
}
