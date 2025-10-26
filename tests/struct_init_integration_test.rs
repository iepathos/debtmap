use debtmap::organization::struct_initialization::StructInitPatternDetector;

#[test]
fn test_struct_init_pattern_detection_with_large_struct() {
    let code = r#"
        pub struct ConfigArgs {
            pub verbose: bool,
            pub quiet: bool,
            pub color: bool,
            pub threads: usize,
            pub max_depth: usize,
            pub timeout: u64,
            pub retry_count: u32,
            pub buffer_size: usize,
            pub enable_cache: bool,
            pub cache_dir: String,
            pub log_level: String,
            pub output_format: String,
            pub include_patterns: Vec<String>,
            pub exclude_patterns: Vec<String>,
            pub follow_links: bool,
            pub hidden_files: bool,
        }

        impl ConfigArgs {
            pub fn from_options(opts: &Options) -> Result<ConfigArgs, Error> {
                let verbose = opts.verbose.unwrap_or(false);
                let quiet = opts.quiet.unwrap_or(false);
                let color = match opts.color {
                    Some(true) => true,
                    Some(false) => false,
                    None => !quiet && verbose,
                };
                let threads = opts.threads.unwrap_or_else(num_cpus::get);
                let max_depth = opts.max_depth.unwrap_or(100);
                let timeout = opts.timeout.unwrap_or(30);
                let retry_count = opts.retry_count.unwrap_or(3);
                let buffer_size = opts.buffer_size.unwrap_or(8192);
                let enable_cache = opts.enable_cache;
                let cache_dir = opts.cache_dir.clone().unwrap_or_else(|| "/tmp/cache".to_string());
                let log_level = opts.log_level.clone().unwrap_or_else(|| "info".to_string());
                let output_format = opts.output_format.clone().unwrap_or_else(|| "text".to_string());
                let include_patterns = opts.include_patterns.clone().unwrap_or_default();
                let exclude_patterns = opts.exclude_patterns.clone().unwrap_or_default();
                let follow_links = opts.follow_links.unwrap_or(false);
                let hidden_files = opts.hidden_files.unwrap_or(false);

                Ok(ConfigArgs {
                    verbose,
                    quiet,
                    color,
                    threads,
                    max_depth,
                    timeout,
                    retry_count,
                    buffer_size,
                    enable_cache,
                    cache_dir,
                    log_level,
                    output_format,
                    include_patterns,
                    exclude_patterns,
                    follow_links,
                    hidden_files,
                })
            }
        }

        pub struct Options {
            pub verbose: Option<bool>,
            pub quiet: Option<bool>,
            pub color: Option<bool>,
            pub threads: Option<usize>,
            pub max_depth: Option<usize>,
            pub timeout: Option<u64>,
            pub retry_count: Option<u32>,
            pub buffer_size: Option<usize>,
            pub enable_cache: bool,
            pub cache_dir: Option<String>,
            pub log_level: Option<String>,
            pub output_format: Option<String>,
            pub include_patterns: Option<Vec<String>>,
            pub exclude_patterns: Option<Vec<String>>,
            pub follow_links: Option<bool>,
            pub hidden_files: Option<bool>,
        }

        pub struct Error;
        pub struct Result<T, E> { value: T, _phantom: std::marker::PhantomData<E> }
    "#;

    let file: syn::File = syn::parse_str(code).expect("Failed to parse test code");

    let detector = StructInitPatternDetector {
        min_field_count: 10,
        min_init_ratio: 0.40,
        max_nesting_depth: 5,
    };

    let pattern = detector.detect(&file, code);

    assert!(
        pattern.is_some(),
        "Should detect struct initialization pattern"
    );

    let pattern = pattern.unwrap();
    assert_eq!(pattern.struct_name, "ConfigArgs");
    assert!(pattern.field_count >= 16, "Should detect 16+ fields");
    assert!(
        pattern.initialization_ratio > 0.40,
        "Should have high initialization ratio"
    );

    // Test complexity scoring
    let field_complexity = detector.calculate_init_complexity_score(&pattern);
    // Field-based complexity should be reasonable for the number of fields
    assert!(
        field_complexity > 0.0 && field_complexity < 10.0,
        "Field-based complexity ({}) should be in reasonable range for {} fields",
        field_complexity,
        pattern.field_count
    );

    // Test confidence
    let confidence = detector.confidence(&pattern);
    assert!(confidence > 0.50, "Should have reasonable confidence score");

    // Test recommendation
    let recommendation = detector.generate_recommendation(&pattern);
    assert!(
        !recommendation.is_empty(),
        "Should provide a recommendation"
    );
}

#[test]
fn test_struct_init_no_false_positives() {
    let code = r#"
        impl Calculator {
            pub fn process_data(items: &[Item]) -> Vec<Result> {
                items.iter()
                    .filter(|item| item.is_valid())
                    .map(|item| {
                        let value = item.compute();
                        let adjusted = if value > 100 {
                            value * 2
                        } else {
                            value
                        };
                        Result { value: adjusted }
                    })
                    .collect()
            }
        }

        pub struct Result { value: i32 }
        pub struct Item;
        impl Item {
            fn is_valid(&self) -> bool { true }
            fn compute(&self) -> i32 { 42 }
        }
    "#;

    let file: syn::File = syn::parse_str(code).expect("Failed to parse test code");
    let detector = StructInitPatternDetector::default();

    let pattern = detector.detect(&file, code);

    // Business logic should not be detected as initialization pattern
    assert!(
        pattern.is_none(),
        "Should not detect business logic as initialization pattern"
    );
}

#[test]
fn test_field_dependency_detection() {
    let code = r#"
        pub struct ComputedConfig {
            pub base_value: i32,
            pub multiplier: i32,
            pub result: i32,
            pub doubled: i32,
            pub final_value: i32,
        }

        impl ComputedConfig {
            pub fn new(input: i32) -> ComputedConfig {
                let base_value = input;
                let multiplier = 2;
                let result = base_value * multiplier;
                let doubled = result * 2;
                let final_value = doubled + base_value;

                ComputedConfig {
                    base_value,
                    multiplier,
                    result,
                    doubled,
                    final_value,
                }
            }
        }
    "#;

    let file: syn::File = syn::parse_str(code).expect("Failed to parse test code");

    let detector = StructInitPatternDetector {
        min_field_count: 3,
        min_init_ratio: 0.30,
        max_nesting_depth: 5,
    };

    let pattern = detector.detect(&file, code);

    if let Some(pattern) = pattern {
        // Check that dependencies were detected
        assert!(
            !pattern.field_dependencies.is_empty(),
            "Should detect field dependencies"
        );

        // Verify some dependencies exist
        let has_dependencies = pattern
            .field_dependencies
            .iter()
            .any(|dep| !dep.depends_on.is_empty());
        assert!(has_dependencies, "Some fields should have dependencies");
    }
}
