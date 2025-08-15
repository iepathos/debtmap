use debtmap::analyzers::function_registry::{FunctionSignatureRegistry, ReturnTypeInfo};
use debtmap::analyzers::rust_call_graph::extract_call_graph_with_signatures;
use debtmap::analyzers::signature_extractor::SignatureExtractor;
use debtmap::analyzers::type_registry::GlobalTypeRegistry;
use debtmap::analyzers::type_tracker::{ResolvedType, TypeSource, TypeTracker};
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn test_function_signature_extraction() {
    let code = r#"
        fn create_parser() -> Parser {
            Parser::new()
        }

        fn load_config() -> Result<Config, Error> {
            Config::from_file("config.toml")
        }

        async fn fetch_data() -> Vec<u8> {
            vec![]
        }

        pub fn get_analyzer(lang: Language) -> Box<dyn Analyzer> {
            Box::new(RustAnalyzer::new())
        }
    "#;

    let syntax = syn::parse_file(code).unwrap();
    let mut extractor = SignatureExtractor::new();
    extractor.extract_from_file(&syntax);

    // Check that functions were extracted
    assert!(extractor.registry.get_function("create_parser").is_some());
    assert!(extractor.registry.get_function("load_config").is_some());
    assert!(extractor.registry.get_function("fetch_data").is_some());
    assert!(extractor.registry.get_function("get_analyzer").is_some());

    // Check return types
    let parser_sig = extractor.registry.get_function("create_parser").unwrap();
    assert_eq!(parser_sig.return_type.type_name, "Parser");
    assert!(!parser_sig.is_async);

    let config_sig = extractor.registry.get_function("load_config").unwrap();
    assert_eq!(config_sig.return_type.type_name, "Result");
    assert!(config_sig.return_type.is_result);

    let fetch_sig = extractor.registry.get_function("fetch_data").unwrap();
    assert_eq!(fetch_sig.return_type.type_name, "Vec");
    assert!(fetch_sig.is_async);
}

#[test]
fn test_method_signature_extraction() {
    let code = r#"
        struct Config {
            path: String,
        }

        impl Config {
            pub fn new() -> Self {
                Self { path: String::new() }
            }

            pub fn load() -> Result<Self, Error> {
                Ok(Self { path: "config.toml".to_string() })
            }

            pub fn validate(&self) -> bool {
                !self.path.is_empty()
            }

            pub fn with_path(mut self, path: String) -> Self {
                self.path = path;
                self
            }
        }
    "#;

    let syntax = syn::parse_file(code).unwrap();
    let mut extractor = SignatureExtractor::new();
    extractor.extract_from_file(&syntax);

    // Check that methods were extracted
    let new_method = extractor.registry.get_method("Config", "new").unwrap();
    assert_eq!(new_method.return_type.type_name, "Config");
    assert!(!new_method.takes_self);

    let load_method = extractor.registry.get_method("Config", "load").unwrap();
    assert_eq!(load_method.return_type.type_name, "Result");
    assert!(load_method.return_type.is_result);

    let validate_method = extractor.registry.get_method("Config", "validate").unwrap();
    assert_eq!(validate_method.return_type.type_name, "bool");
    assert!(validate_method.takes_self);

    let with_path_method = extractor
        .registry
        .get_method("Config", "with_path")
        .unwrap();
    assert_eq!(with_path_method.return_type.type_name, "Config");
    assert!(with_path_method.takes_self);
    assert!(with_path_method.takes_mut_self);
}

#[test]
fn test_builder_pattern_detection() {
    let code = r#"
        struct ServiceBuilder {
            config: Option<Config>,
            timeout: Option<u32>,
        }

        struct Service {
            config: Config,
            timeout: u32,
        }

        impl ServiceBuilder {
            pub fn new() -> Self {
                Self { config: None, timeout: None }
            }

            pub fn with_config(mut self, config: Config) -> Self {
                self.config = Some(config);
                self
            }

            pub fn with_timeout(mut self, timeout: u32) -> Self {
                self.timeout = Some(timeout);
                self
            }

            pub fn build(self) -> Service {
                Service {
                    config: self.config.unwrap_or_default(),
                    timeout: self.timeout.unwrap_or(30),
                }
            }
        }
    "#;

    let syntax = syn::parse_file(code).unwrap();
    let mut extractor = SignatureExtractor::new();
    extractor.extract_from_file(&syntax);

    // Check builder detection
    assert!(extractor.registry.is_builder("ServiceBuilder"));

    let builder_info = extractor.registry.get_builder("ServiceBuilder").unwrap();
    assert_eq!(builder_info.target_type, "Service");
    assert_eq!(builder_info.build_method, "build");
    assert!(builder_info
        .chain_methods
        .contains(&"with_config".to_string()));
    assert!(builder_info
        .chain_methods
        .contains(&"with_timeout".to_string()));
}

#[test]
fn test_function_call_resolution() {
    let code = r#"
        fn create_parser() -> Parser {
            Parser::new()
        }

        fn main() {
            let parser = create_parser();
            parser.parse("test");
        }
    "#;

    let syntax = syn::parse_file(code).unwrap();
    let mut extractor = SignatureExtractor::new();
    extractor.extract_from_file(&syntax);
    let registry = Arc::new(extractor.registry);

    let mut type_tracker = TypeTracker::new();
    type_tracker.set_function_registry(registry.clone());

    // Simulate resolving the function call
    let return_info = registry
        .resolve_function_return("create_parser", &[])
        .unwrap();
    assert_eq!(return_info.type_name, "Parser");
}

#[test]
fn test_static_constructor_resolution() {
    let code = r#"
        impl HashMap<K, V> {
            pub fn new() -> Self {
                Self { inner: Vec::new() }
            }
        }

        impl Result<T, E> {
            pub fn ok(value: T) -> Self {
                Ok(value)
            }
        }
    "#;

    let syntax = syn::parse_file(code).unwrap();
    let mut extractor = SignatureExtractor::new();
    extractor.extract_from_file(&syntax);
    let registry = Arc::new(extractor.registry);

    // Test HashMap::new resolution
    let new_method = registry.get_method("HashMap", "new").unwrap();
    assert_eq!(new_method.return_type.type_name, "HashMap");

    // Test Result::ok resolution
    let ok_method = registry.get_method("Result", "ok").unwrap();
    assert_eq!(ok_method.return_type.type_name, "Result");
}

#[test]
fn test_method_chain_resolution() {
    let code = r#"
        struct String;
        
        impl String {
            pub fn trim(&self) -> &str {
                ""
            }
        }

        impl str {
            pub fn to_string(&self) -> String {
                String
            }
        }
    "#;

    let syntax = syn::parse_file(code).unwrap();
    let mut extractor = SignatureExtractor::new();
    extractor.extract_from_file(&syntax);
    let registry = Arc::new(extractor.registry);

    // Check method return types
    let trim_method = registry.get_method("String", "trim").unwrap();
    assert_eq!(trim_method.return_type.type_name, "str");

    let to_string_method = registry.get_method("str", "to_string").unwrap();
    assert_eq!(to_string_method.return_type.type_name, "String");
}

#[test]
fn test_generic_function_signatures() {
    let code = r#"
        fn convert<T, U>(value: T) -> Result<U, Error> {
            unimplemented!()
        }

        fn identity<T>(value: T) -> T {
            value
        }

        fn collect<T: Iterator>(iter: T) -> Vec<T::Item> {
            iter.collect()
        }
    "#;

    let syntax = syn::parse_file(code).unwrap();
    let mut extractor = SignatureExtractor::new();
    extractor.extract_from_file(&syntax);

    // Check generic parameters were extracted
    let convert_sig = extractor.registry.get_function("convert").unwrap();
    assert_eq!(convert_sig.generic_params, vec!["T", "U"]);
    assert_eq!(convert_sig.return_type.type_name, "Result");

    let identity_sig = extractor.registry.get_function("identity").unwrap();
    assert_eq!(identity_sig.generic_params, vec!["T"]);
    assert_eq!(identity_sig.return_type.type_name, "T");

    let collect_sig = extractor.registry.get_function("collect").unwrap();
    assert_eq!(collect_sig.generic_params, vec!["T"]);
    assert_eq!(collect_sig.return_type.type_name, "Vec");
}

#[test]
fn test_call_graph_with_signatures() {
    let code = r#"
        fn create_config() -> Config {
            Config::default()
        }

        fn process_data() {
            let config = create_config();
            config.validate();
        }

        impl Config {
            fn default() -> Self {
                Self {}
            }

            fn validate(&self) -> bool {
                true
            }
        }
    "#;

    let syntax = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let type_registry = Arc::new(GlobalTypeRegistry::new());

    let (call_graph, function_registry) =
        extract_call_graph_with_signatures(&syntax, &path, type_registry);

    // Verify function signatures were extracted
    assert!(function_registry.get_function("create_config").is_some());
    assert!(function_registry.get_method("Config", "default").is_some());
    assert!(function_registry.get_method("Config", "validate").is_some());

    // Verify return types
    let create_config_sig = function_registry.get_function("create_config").unwrap();
    assert_eq!(create_config_sig.return_type.type_name, "Config");
}

#[test]
fn test_option_result_handling() {
    let code = r#"
        fn try_parse() -> Result<Parser, Error> {
            Ok(Parser::new())
        }

        fn maybe_config() -> Option<Config> {
            Some(Config::default())
        }

        fn unwrap_or_default() -> Config {
            maybe_config().unwrap_or_else(Config::default)
        }
    "#;

    let syntax = syn::parse_file(code).unwrap();
    let mut extractor = SignatureExtractor::new();
    extractor.extract_from_file(&syntax);

    let try_parse_sig = extractor.registry.get_function("try_parse").unwrap();
    assert!(try_parse_sig.return_type.is_result);
    assert_eq!(try_parse_sig.return_type.type_name, "Result");

    let maybe_config_sig = extractor.registry.get_function("maybe_config").unwrap();
    assert!(maybe_config_sig.return_type.is_option);
    assert_eq!(maybe_config_sig.return_type.type_name, "Option");
}

#[test]
fn test_visibility_tracking() {
    let code = r#"
        pub fn public_function() -> i32 { 42 }
        pub(crate) fn crate_function() -> i32 { 42 }
        fn private_function() -> i32 { 42 }

        impl MyType {
            pub fn public_method(&self) {}
            pub(crate) fn crate_method(&self) {}
            fn private_method(&self) {}
        }
    "#;

    let syntax = syn::parse_file(code).unwrap();
    let mut extractor = SignatureExtractor::new();
    extractor.extract_from_file(&syntax);

    use debtmap::analyzers::function_registry::VisibilityInfo;

    let public_fn = extractor.registry.get_function("public_function").unwrap();
    assert!(matches!(public_fn.visibility, VisibilityInfo::Public));

    let crate_fn = extractor.registry.get_function("crate_function").unwrap();
    assert!(matches!(crate_fn.visibility, VisibilityInfo::PublicCrate));

    let private_fn = extractor.registry.get_function("private_function").unwrap();
    assert!(matches!(private_fn.visibility, VisibilityInfo::Private));
}
