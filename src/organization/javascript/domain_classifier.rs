/// Classify JavaScript/TypeScript class domains based on naming patterns
pub fn classify_javascript_class_domain(class_name: &str, methods: &[String]) -> String {
    let lower = class_name.to_lowercase();

    // JavaScript/TypeScript-specific patterns
    if lower.contains("component") || lower.contains("view") || lower.contains("widget") {
        return "ui".to_string();
    }

    if lower.contains("controller") || lower.contains("route") || lower.contains("router") {
        return "routing".to_string();
    }

    if lower.contains("service") {
        return "service".to_string();
    }

    if lower.contains("model") || lower.contains("schema") || lower.contains("entity") {
        return "model".to_string();
    }

    if lower.contains("util") || lower.contains("helper") || lower.contains("tools") {
        return "utilities".to_string();
    }

    if lower.contains("store") || lower.contains("state") || lower.contains("reducer") {
        return "state_management".to_string();
    }

    if lower.contains("api") || lower.contains("client") || lower.contains("http") {
        return "api".to_string();
    }

    if lower.contains("hook") {
        return "hooks".to_string();
    }

    if lower.contains("provider") || lower.contains("context") {
        return "context".to_string();
    }

    if lower.contains("middleware") {
        return "middleware".to_string();
    }

    if lower.contains("validator") || lower.contains("validation") {
        return "validation".to_string();
    }

    if lower.contains("formatter") || lower.contains("serializer") {
        return "formatting".to_string();
    }

    if lower.contains("repository") || lower.contains("dao") {
        return "data_access".to_string();
    }

    if lower.contains("manager") || lower.contains("handler") {
        return "service".to_string();
    }

    if lower.contains("test") || lower.contains("spec") || lower.contains("mock") {
        return "testing".to_string();
    }

    if lower.contains("config") || lower.contains("settings") {
        return "configuration".to_string();
    }

    if lower.contains("auth") || lower.contains("permission") {
        return "authorization".to_string();
    }

    // Fallback to method-name-based classification
    infer_domain_from_methods(methods)
}

/// Infer domain from method names when class name is ambiguous
fn infer_domain_from_methods(methods: &[String]) -> String {
    let method_str = methods.join(" ").to_lowercase();

    let mut domain_scores: Vec<(&str, usize)> = vec![
        (
            "ui",
            count_keywords(
                &method_str,
                &["render", "mount", "unmount", "onclick", "onchange"],
            ),
        ),
        (
            "routing",
            count_keywords(&method_str, &["route", "navigate", "redirect", "handle"]),
        ),
        (
            "service",
            count_keywords(&method_str, &["process", "execute", "handle", "perform"]),
        ),
        (
            "model",
            count_keywords(
                &method_str,
                &["toobject", "tojson", "validate", "serialize"],
            ),
        ),
        (
            "state_management",
            count_keywords(
                &method_str,
                &["dispatch", "subscribe", "getstate", "setstate"],
            ),
        ),
        (
            "api",
            count_keywords(
                &method_str,
                &["get", "post", "put", "delete", "fetch", "request"],
            ),
        ),
        (
            "validation",
            count_keywords(&method_str, &["validate", "check", "verify", "ensure"]),
        ),
        (
            "data_access",
            count_keywords(&method_str, &["save", "find", "query", "fetch", "load"]),
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
    fn test_component_pattern() {
        assert_eq!(classify_javascript_class_domain("UserComponent", &[]), "ui");
        assert_eq!(classify_javascript_class_domain("AppView", &[]), "ui");
        assert_eq!(classify_javascript_class_domain("FormWidget", &[]), "ui");
    }

    #[test]
    fn test_controller_pattern() {
        assert_eq!(
            classify_javascript_class_domain("UserController", &[]),
            "routing"
        );
        assert_eq!(
            classify_javascript_class_domain("ApiRouter", &[]),
            "routing"
        );
    }

    #[test]
    fn test_service_pattern() {
        assert_eq!(
            classify_javascript_class_domain("UserService", &[]),
            "service"
        );
        assert_eq!(
            classify_javascript_class_domain("EmailManager", &[]),
            "service"
        );
    }

    #[test]
    fn test_model_pattern() {
        assert_eq!(classify_javascript_class_domain("UserModel", &[]), "model");
        assert_eq!(
            classify_javascript_class_domain("ProductSchema", &[]),
            "model"
        );
    }

    #[test]
    fn test_util_pattern() {
        assert_eq!(
            classify_javascript_class_domain("StringUtil", &[]),
            "utilities"
        );
        assert_eq!(
            classify_javascript_class_domain("DateHelper", &[]),
            "utilities"
        );
    }

    #[test]
    fn test_store_pattern() {
        assert_eq!(
            classify_javascript_class_domain("UserStore", &[]),
            "state_management"
        );
        assert_eq!(
            classify_javascript_class_domain("AppState", &[]),
            "state_management"
        );
        assert_eq!(
            classify_javascript_class_domain("CartReducer", &[]),
            "state_management"
        );
    }

    #[test]
    fn test_api_pattern() {
        assert_eq!(classify_javascript_class_domain("ApiClient", &[]), "api");
        assert_eq!(
            classify_javascript_class_domain("HttpService", &[]),
            "service"
        ); // Service takes precedence
    }

    #[test]
    fn test_hook_pattern() {
        assert_eq!(
            classify_javascript_class_domain("useUserHook", &[]),
            "hooks"
        );
    }

    #[test]
    fn test_provider_pattern() {
        assert_eq!(
            classify_javascript_class_domain("ThemeProvider", &[]),
            "context"
        );
        assert_eq!(
            classify_javascript_class_domain("UserContext", &[]),
            "context"
        );
    }

    #[test]
    fn test_middleware_pattern() {
        assert_eq!(
            classify_javascript_class_domain("AuthMiddleware", &[]),
            "middleware"
        );
    }

    #[test]
    fn test_validator_pattern() {
        assert_eq!(
            classify_javascript_class_domain("FormValidator", &[]),
            "validation"
        );
    }

    #[test]
    fn test_formatter_pattern() {
        assert_eq!(
            classify_javascript_class_domain("DateFormatter", &[]),
            "formatting"
        );
        assert_eq!(
            classify_javascript_class_domain("JsonSerializer", &[]),
            "formatting"
        );
    }

    #[test]
    fn test_repository_pattern() {
        assert_eq!(
            classify_javascript_class_domain("UserRepository", &[]),
            "data_access"
        );
    }

    #[test]
    fn test_test_pattern() {
        assert_eq!(classify_javascript_class_domain("UserTest", &[]), "testing");
        assert_eq!(classify_javascript_class_domain("ApiSpec", &[]), "api"); // "Api" takes precedence
        assert_eq!(
            classify_javascript_class_domain("MockService", &[]),
            "service"
        ); // "Service" takes precedence
    }

    #[test]
    fn test_config_pattern() {
        assert_eq!(
            classify_javascript_class_domain("AppConfig", &[]),
            "configuration"
        );
        assert_eq!(
            classify_javascript_class_domain("Settings", &[]),
            "configuration"
        );
    }

    #[test]
    fn test_auth_pattern() {
        assert_eq!(
            classify_javascript_class_domain("AuthGuard", &[]),
            "authorization"
        );
        assert_eq!(
            classify_javascript_class_domain("PermissionChecker", &[]),
            "authorization"
        );
    }

    #[test]
    fn test_method_based_inference_ui() {
        let methods = vec![
            "render".to_string(),
            "mount".to_string(),
            "onClick".to_string(),
        ];
        assert_eq!(classify_javascript_class_domain("MyClass", &methods), "ui");
    }

    #[test]
    fn test_method_based_inference_api() {
        let methods = vec!["get".to_string(), "post".to_string(), "fetch".to_string()];
        assert_eq!(
            classify_javascript_class_domain("DataProcessor", &methods),
            "api"
        );
    }

    #[test]
    fn test_method_based_inference_state() {
        let methods = vec![
            "dispatch".to_string(),
            "getState".to_string(),
            "subscribe".to_string(),
        ];
        assert_eq!(
            classify_javascript_class_domain("StateProcessor", &methods),
            "state_management"
        );
    }

    #[test]
    fn test_fallback_to_general() {
        let methods = vec!["foo".to_string(), "bar".to_string()];
        assert_eq!(
            classify_javascript_class_domain("MyClass", &methods),
            "general"
        );
    }

    #[test]
    fn test_infer_domain_from_methods_validation() {
        let methods = vec!["validateEmail".to_string(), "checkPassword".to_string()];
        assert_eq!(infer_domain_from_methods(&methods), "validation");
    }

    #[test]
    fn test_infer_domain_from_methods_data_access() {
        let methods = vec!["save".to_string(), "find".to_string(), "query".to_string()];
        assert_eq!(infer_domain_from_methods(&methods), "data_access");
    }

    #[test]
    fn test_count_keywords() {
        let text = "render component and onclick handler";
        assert_eq!(count_keywords(text, &["render", "onclick", "mount"]), 2);
        assert_eq!(count_keywords(text, &["save", "find"]), 0);
    }
}
