/// Classify Python class domains based on naming patterns
pub fn classify_python_class_domain(class_name: &str, methods: &[String]) -> String {
    let lower = class_name.to_lowercase();

    // Python-specific patterns based on common naming conventions
    if lower.contains("repository") || lower.contains("dao") || lower.contains("dataaccess") {
        return "data_access".to_string();
    }

    if lower.contains("manager") || lower.contains("service") {
        return "service".to_string();
    }

    if lower.contains("controller") || lower.contains("handler") {
        return "controller".to_string();
    }

    if lower.contains("validator") || lower.contains("checker") {
        return "validation".to_string();
    }

    if lower.contains("serializer")
        || lower.contains("formatter")
        || lower.contains("encoder")
        || lower.contains("decoder")
    {
        return "formatting".to_string();
    }

    if lower.contains("view") || lower.contains("template") {
        return "presentation".to_string();
    }

    if lower.contains("model") || lower.contains("entity") || lower.contains("schema") {
        return "model".to_string();
    }

    if lower.contains("test") || lower.contains("fixture") {
        return "testing".to_string();
    }

    if lower.contains("middleware") || lower.contains("decorator") {
        return "middleware".to_string();
    }

    if lower.contains("client") || lower.contains("api") {
        return "api_client".to_string();
    }

    if lower.contains("cache") || lower.contains("store") {
        return "caching".to_string();
    }

    if lower.contains("logger") || lower.contains("audit") {
        return "logging".to_string();
    }

    if lower.contains("auth") || lower.contains("permission") {
        return "authorization".to_string();
    }

    if lower.contains("session") {
        return "session_management".to_string();
    }

    if lower.contains("notification") || lower.contains("mailer") || lower.contains("email") {
        return "notifications".to_string();
    }

    // Fallback to method-name-based classification
    infer_domain_from_methods(methods)
}

/// Infer domain from method names when class name is ambiguous
fn infer_domain_from_methods(methods: &[String]) -> String {
    let method_str = methods.join(" ").to_lowercase();

    // Count occurrences of different domain indicators
    let mut domain_scores: Vec<(&str, usize)> = vec![
        (
            "data_access",
            count_keywords(
                &method_str,
                &["save", "find", "query", "fetch", "load", "persist"],
            ),
        ),
        (
            "service",
            count_keywords(&method_str, &["process", "handle", "execute", "perform"]),
        ),
        (
            "validation",
            count_keywords(&method_str, &["validate", "check", "verify", "ensure"]),
        ),
        (
            "formatting",
            count_keywords(
                &method_str,
                &["format", "serialize", "deserialize", "encode", "decode"],
            ),
        ),
        (
            "presentation",
            count_keywords(&method_str, &["render", "display", "show"]),
        ),
        (
            "api_client",
            count_keywords(&method_str, &["get", "post", "put", "delete", "request"]),
        ),
        (
            "notifications",
            count_keywords(&method_str, &["send", "notify", "email"]),
        ),
        (
            "caching",
            count_keywords(&method_str, &["cache", "invalidate", "refresh"]),
        ),
    ];

    domain_scores.sort_by(|a, b| b.1.cmp(&a.1));

    if domain_scores[0].1 > 0 {
        domain_scores[0].0.to_string()
    } else {
        "general".to_string()
    }
}

fn count_keywords(text: &str, keywords: &[&str]) -> usize {
    keywords.iter().filter(|kw| text.contains(*kw)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_pattern() {
        assert_eq!(
            classify_python_class_domain("UserRepository", &[]),
            "data_access"
        );
        assert_eq!(
            classify_python_class_domain("ProductDAO", &[]),
            "data_access"
        );
    }

    #[test]
    fn test_service_pattern() {
        assert_eq!(classify_python_class_domain("EmailService", &[]), "service");
        assert_eq!(classify_python_class_domain("UserManager", &[]), "service");
    }

    #[test]
    fn test_controller_pattern() {
        assert_eq!(
            classify_python_class_domain("UserController", &[]),
            "controller"
        );
        assert_eq!(
            classify_python_class_domain("ApiHandler", &[]),
            "controller"
        );
    }

    #[test]
    fn test_validation_pattern() {
        assert_eq!(
            classify_python_class_domain("PasswordValidator", &[]),
            "validation"
        );
        assert_eq!(
            classify_python_class_domain("EmailChecker", &[]),
            "validation"
        );
    }

    #[test]
    fn test_serializer_pattern() {
        assert_eq!(
            classify_python_class_domain("UserSerializer", &[]),
            "formatting"
        );
        assert_eq!(
            classify_python_class_domain("JsonFormatter", &[]),
            "formatting"
        );
    }

    #[test]
    fn test_view_pattern() {
        assert_eq!(
            classify_python_class_domain("UserView", &[]),
            "presentation"
        );
        assert_eq!(
            classify_python_class_domain("TemplateRenderer", &[]),
            "presentation"
        );
    }

    #[test]
    fn test_model_pattern() {
        assert_eq!(classify_python_class_domain("UserModel", &[]), "model");
        assert_eq!(classify_python_class_domain("ProductEntity", &[]), "model");
        assert_eq!(classify_python_class_domain("OrderSchema", &[]), "model");
    }

    #[test]
    fn test_test_pattern() {
        assert_eq!(classify_python_class_domain("TestUser", &[]), "testing");
        assert_eq!(classify_python_class_domain("UserFixture", &[]), "testing");
    }

    #[test]
    fn test_middleware_pattern() {
        assert_eq!(
            classify_python_class_domain("AuthMiddleware", &[]),
            "middleware"
        );
        assert_eq!(
            classify_python_class_domain("LoggingDecorator", &[]),
            "middleware"
        );
    }

    #[test]
    fn test_api_client_pattern() {
        assert_eq!(
            classify_python_class_domain("HttpClient", &[]),
            "api_client"
        );
        assert_eq!(
            classify_python_class_domain("ApiWrapper", &[]),
            "api_client"
        );
    }

    #[test]
    fn test_cache_pattern() {
        assert_eq!(classify_python_class_domain("RedisCache", &[]), "caching");
        assert_eq!(classify_python_class_domain("DataStore", &[]), "caching");
    }

    #[test]
    fn test_auth_pattern() {
        assert_eq!(classify_python_class_domain("AuthService", &[]), "service"); // Service takes precedence
        assert_eq!(
            classify_python_class_domain("PermissionChecker", &[]),
            "validation"
        ); // Checker triggers validation pattern
    }

    #[test]
    fn test_session_pattern() {
        assert_eq!(
            classify_python_class_domain("SessionManager", &[]),
            "service"
        ); // Manager/Service takes precedence
        assert_eq!(
            classify_python_class_domain("UserSession", &[]),
            "session_management"
        );
    }

    #[test]
    fn test_notification_pattern() {
        assert_eq!(
            classify_python_class_domain("EmailNotifier", &[]),
            "notifications"
        );
        assert_eq!(
            classify_python_class_domain("MailerService", &[]),
            "service"
        ); // Service takes precedence
    }

    #[test]
    fn test_method_based_inference_data_access() {
        let methods = vec![
            "save_user".to_string(),
            "find_by_id".to_string(),
            "query_all".to_string(),
        ];
        assert_eq!(
            classify_python_class_domain("DataProcessor", &methods),
            "data_access"
        );
    }

    #[test]
    fn test_method_based_inference_validation() {
        let methods = vec![
            "validate_email".to_string(),
            "check_password".to_string(),
            "verify_token".to_string(),
        ];
        assert_eq!(
            classify_python_class_domain("InputProcessor", &methods),
            "validation"
        );
    }

    #[test]
    fn test_method_based_inference_api() {
        let methods = vec![
            "get_user".to_string(),
            "post_data".to_string(),
            "delete_item".to_string(),
        ];
        assert_eq!(
            classify_python_class_domain("ApiHandler", &methods),
            "controller"
        ); // "Handler" takes precedence, but if it was generic:
    }

    #[test]
    fn test_fallback_to_general() {
        let methods = vec!["foo".to_string(), "bar".to_string()];
        assert_eq!(classify_python_class_domain("MyClass", &methods), "general");
    }

    #[test]
    fn test_infer_domain_from_methods_formatting() {
        let methods = vec!["serialize_data".to_string(), "deserialize_json".to_string()];
        assert_eq!(infer_domain_from_methods(&methods), "formatting");
    }

    #[test]
    fn test_infer_domain_from_methods_service() {
        let methods = vec!["process_order".to_string(), "handle_payment".to_string()];
        assert_eq!(infer_domain_from_methods(&methods), "service");
    }

    #[test]
    fn test_count_keywords() {
        let text = "save user and fetch data from database";
        assert_eq!(count_keywords(text, &["save", "fetch", "query"]), 2);
        assert_eq!(count_keywords(text, &["process", "handle"]), 0);
    }
}
