use debtmap::testing::analyze_testing_patterns;
use std::path::PathBuf;

#[test]
fn test_detects_test_without_assertions() {
    let source = r#"
        #[test]
        fn test_without_assertions() {
            let user = User::new("test");
            let result = user.validate();
            // Missing assertion!
        }
        
        #[test]
        fn test_with_assertions() {
            let user = User::new("test");
            let result = user.validate();
            assert!(result.is_ok());
        }
    "#;

    let file = syn::parse_str::<syn::File>(source).unwrap();
    let path = PathBuf::from("test.rs");
    let patterns = analyze_testing_patterns(&file, &path);

    // Debug: print all patterns
    for pattern in &patterns {
        eprintln!("Pattern: {}", pattern.message);
    }

    // Should only detect test_without_assertions
    let no_assertion_patterns: Vec<_> = patterns
        .iter()
        .filter(|p| p.message.contains("no assertions"))
        .collect();
    assert_eq!(
        no_assertion_patterns.len(),
        1,
        "Expected 1 no-assertion pattern, found {}",
        no_assertion_patterns.len()
    );
    assert!(matches!(
        &no_assertion_patterns[0].debt_type,
        debtmap::core::DebtType::TestQuality
    ));
}

#[test]
fn test_detects_overly_complex_test() {
    let source = r#"
        #[test]
        fn overly_complex_test() {
            let mut mock1 = MockService::new();
            let mut mock2 = MockDatabase::new();
            let mut mock3 = MockCache::new();
            let mut mock4 = MockApi::new();
            let mut mock5 = MockLogger::new();
            let mut mock6 = MockNotifier::new();
            
            mock1.expect_call().times(1).returning(|| Ok(()));
            mock2.expect_query().with(eq("test")).returning(|| Ok(vec![]));
            mock3.expect_get().returning(|| None);
            mock4.expect_fetch().returning(|| Ok(Response::new()));
            mock5.expect_log().returning(|| ());
            mock6.expect_notify().returning(|| Ok(()));
            
            for i in 0..10 {
                if i % 2 == 0 {
                    let result = service.process(i);
                    if let Some(data) = result {
                        assert_eq!(data.len(), i);
                    } else {
                        panic!("Unexpected None");
                    }
                }
            }
        }
    "#;

    let file = syn::parse_str::<syn::File>(source).unwrap();
    let path = PathBuf::from("test.rs");
    let patterns = analyze_testing_patterns(&file, &path);

    let complex_pattern = patterns
        .iter()
        .find(|p| matches!(p.debt_type, debtmap::core::DebtType::TestComplexity));

    assert!(complex_pattern.is_some());
    assert!(complex_pattern.unwrap().message.contains("overly complex"));
}

#[test]
fn test_detects_flaky_timing_test() {
    let source = r#"
        #[test]
        fn flaky_timing_test() {
            let start = std::time::Instant::now();
            std::thread::sleep(std::time::Duration::from_millis(100));
            let duration = start.elapsed();
            assert!(duration >= std::time::Duration::from_millis(90));
        }
    "#;

    let file = syn::parse_str::<syn::File>(source).unwrap();
    let path = PathBuf::from("test.rs");
    let patterns = analyze_testing_patterns(&file, &path);

    let timing_pattern = patterns
        .iter()
        .find(|p| p.message.contains("flaky pattern"));

    assert!(timing_pattern.is_some());
    assert!(timing_pattern.unwrap().message.contains("TimingDependency"));
}

#[test]
fn test_detects_random_value_usage() {
    let source = r#"
        #[test]
        fn test_with_random() {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let random_value = rng.gen_range(0..100);
            let result = calculate_something(random_value);
            assert!(result > 0);
        }
    "#;

    let file = syn::parse_str::<syn::File>(source).unwrap();
    let path = PathBuf::from("test.rs");
    let patterns = analyze_testing_patterns(&file, &path);

    let random_pattern = patterns.iter().find(|p| p.message.contains("RandomValues"));

    assert!(random_pattern.is_some());
}

#[test]
fn test_detects_external_service_call() {
    let source = r#"
        #[test]
        fn test_with_external_api() {
            let client = reqwest::Client::new();
            let response = client.get("https://api.example.com/data")
                .send()
                .unwrap();
            assert_eq!(response.status(), 200);
        }
    "#;

    let file = syn::parse_str::<syn::File>(source).unwrap();
    let path = PathBuf::from("test.rs");
    let patterns = analyze_testing_patterns(&file, &path);

    let external_pattern = patterns
        .iter()
        .find(|p| p.message.contains("ExternalDependency"));

    assert!(external_pattern.is_some());
}

#[test]
fn test_detects_filesystem_dependency() {
    let source = r#"
        #[test]
        fn test_with_filesystem() {
            let content = std::fs::read_to_string("test.txt").unwrap();
            assert!(!content.is_empty());
        }
    "#;

    let file = syn::parse_str::<syn::File>(source).unwrap();
    let path = PathBuf::from("test.rs");
    let patterns = analyze_testing_patterns(&file, &path);

    let fs_pattern = patterns
        .iter()
        .find(|p| p.message.contains("FilesystemDependency"));

    assert!(fs_pattern.is_some());
}

#[test]
fn test_detects_test_functions_by_attribute() {
    let source = r#"
        #[test]
        fn proper_test_function() {
            assert!(true);
        }
        
        #[tokio::test]
        async fn async_test_function() {
            assert!(true);
        }
        
        fn not_a_test_function() {
            // This should not be analyzed
        }
    "#;

    let file = syn::parse_str::<syn::File>(source).unwrap();
    let path = PathBuf::from("test.rs");
    let patterns = analyze_testing_patterns(&file, &path);

    // Both tests have assertions so should not report issues
    let no_assertion_patterns: Vec<_> = patterns
        .iter()
        .filter(|p| p.message.contains("no assertions"))
        .collect();
    assert_eq!(no_assertion_patterns.len(), 0);
}

#[test]
fn test_detects_test_functions_by_name() {
    let source = r#"
        fn test_something() {
            let x = 5;
            // Missing assertion
        }
        
        fn something_test() {
            let y = 10;
            // Missing assertion
        }
    "#;

    let file = syn::parse_str::<syn::File>(source).unwrap();
    let path = PathBuf::from("test.rs");
    let patterns = analyze_testing_patterns(&file, &path);

    assert_eq!(patterns.len(), 2);
    assert!(patterns.iter().all(|p| p.message.contains("no assertions")));
}

#[test]
fn test_expect_and_unwrap_count_as_assertions() {
    let source = r#"
        #[test]
        fn test_with_expect() {
            let result = some_function();
            result.expect("Should succeed");
        }
        
        #[test]
        fn test_with_unwrap() {
            let result = some_function();
            result.unwrap();
        }
    "#;

    let file = syn::parse_str::<syn::File>(source).unwrap();
    let path = PathBuf::from("test.rs");
    let patterns = analyze_testing_patterns(&file, &path);

    // expect and unwrap should count as assertions
    assert_eq!(patterns.len(), 0);
}

#[test]
fn test_detects_multiple_anti_patterns_in_single_test() {
    let source = r#"
        #[test]
        fn problematic_test() {
            // Complex setup with many mocks
            let mut mock1 = Mock::new();
            let mut mock2 = Mock::new();
            let mut mock3 = Mock::new();
            let mut mock4 = Mock::new();
            let mut mock5 = Mock::new();
            let mut mock6 = Mock::new();
            
            // Flaky timing dependency
            std::thread::sleep(std::time::Duration::from_millis(100));
            
            // Random value
            let random = rand::random::<u32>();
            
            // External service
            let client = reqwest::Client::new();
            
            // No assertions at all
        }
    "#;

    let file = syn::parse_str::<syn::File>(source).unwrap();
    let path = PathBuf::from("test.rs");
    let patterns = analyze_testing_patterns(&file, &path);

    // Should detect multiple issues
    assert!(patterns.len() >= 3);

    // Should have no assertions issue
    assert!(patterns.iter().any(|p| p.message.contains("no assertions")));

    // Should have flakiness issues
    assert!(patterns.iter().any(|p| p.message.contains("flaky pattern")));
}
