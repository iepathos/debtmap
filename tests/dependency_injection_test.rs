//! Integration tests for the dependency injection system

use anyhow::Result;
use debtmap::core::injection::{
    AnalyzerFactory, AppContainer, AppContainerBuilder, ServiceLocator,
};
use debtmap::core::traits::{
    Analyzer, Cache, CacheStats, ConfigProvider, Formatter, PriorityCalculator, PriorityFactor,
    Scorer,
};
use debtmap::core::types::{
    AnalysisResult, DebtCategory, DebtItem, FunctionInfo, Language, ModuleInfo, ProjectMetrics,
    Severity, SourceLocation,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// Test implementations
struct TestAnalyzer {
    language: Language,
    name: String,
}

impl Analyzer for TestAnalyzer {
    type Input = String;
    type Output = ModuleInfo;

    fn analyze(&self, _input: Self::Input) -> Result<Self::Output> {
        Ok(ModuleInfo {
            name: format!("test_module_{}", self.name),
            language: self.language,
            path: PathBuf::from(format!("test.{}", self.name)),
            functions: vec![FunctionInfo {
                name: "test_function".to_string(),
                location: SourceLocation {
                    file: PathBuf::from("test.rs"),
                    line: 1,
                    column: 0,
                    end_line: Some(10),
                    end_column: Some(0),
                },
                parameters: vec!["arg1".to_string(), "arg2".to_string()],
                return_type: Some("Result<()>".to_string()),
                is_public: true,
                is_async: false,
                is_generic: false,
                doc_comment: Some("Test function".to_string()),
            }],
            exports: vec!["export1".to_string()],
            imports: vec!["import1".to_string(), "import2".to_string()],
        })
    }

    fn name(&self) -> &str {
        &self.name
    }
}

struct TestScorer {
    base_score: f64,
}

impl Scorer for TestScorer {
    type Item = DebtItem;

    fn score(&self, item: &Self::Item) -> f64 {
        let category_multiplier = match item.category {
            DebtCategory::Complexity => 2.0,
            DebtCategory::Testing => 1.5,
            DebtCategory::Documentation => 1.2,
            _ => 1.0,
        };

        let severity_multiplier = match item.severity {
            Severity::Critical => 3.0,
            Severity::Major => 2.0,
            Severity::Warning => 1.5,
            Severity::Info => 1.0,
        };

        self.base_score * category_multiplier * severity_multiplier
    }

    fn methodology(&self) -> &str {
        "Test scoring methodology based on category and severity"
    }
}

struct TestCache {
    storage: std::sync::Mutex<HashMap<String, Vec<u8>>>,
    hit_count: std::sync::atomic::AtomicUsize,
    miss_count: std::sync::atomic::AtomicUsize,
}

impl TestCache {
    fn new() -> Self {
        Self {
            storage: std::sync::Mutex::new(HashMap::new()),
            hit_count: std::sync::atomic::AtomicUsize::new(0),
            miss_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

impl Cache for TestCache {
    type Key = String;
    type Value = Vec<u8>;

    fn get(&self, key: &Self::Key) -> Option<Self::Value> {
        let storage = self.storage.lock().unwrap();
        if let Some(value) = storage.get(key) {
            self.hit_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Some(value.clone())
        } else {
            self.miss_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            None
        }
    }

    fn set(&mut self, key: Self::Key, value: Self::Value) {
        let mut storage = self.storage.lock().unwrap();
        storage.insert(key, value);
    }

    fn clear(&mut self) {
        let mut storage = self.storage.lock().unwrap();
        storage.clear();
        self.hit_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.miss_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }

    fn stats(&self) -> CacheStats {
        let storage = self.storage.lock().unwrap();
        let memory_usage: usize = storage.values().map(|v| v.len()).sum();

        CacheStats {
            hits: self.hit_count.load(std::sync::atomic::Ordering::Relaxed),
            misses: self.miss_count.load(std::sync::atomic::Ordering::Relaxed),
            entries: storage.len(),
            memory_usage,
        }
    }
}

struct TestConfigProvider {
    config: std::sync::RwLock<HashMap<String, String>>,
}

impl TestConfigProvider {
    fn new() -> Self {
        let mut config = HashMap::new();
        config.insert("complexity_threshold".to_string(), "15".to_string());
        config.insert("max_file_size".to_string(), "2000000".to_string());
        config.insert("enable_caching".to_string(), "true".to_string());

        Self {
            config: std::sync::RwLock::new(config),
        }
    }
}

impl ConfigProvider for TestConfigProvider {
    fn get(&self, key: &str) -> Option<String> {
        let config = self.config.read().unwrap();
        config.get(key).cloned()
    }

    fn set(&mut self, key: String, value: String) {
        let mut config = self.config.write().unwrap();
        config.insert(key, value);
    }

    fn load_from_file(&self, _path: &std::path::Path) -> Result<()> {
        // Test implementation - could load from test fixtures
        Ok(())
    }
}

struct TestPriorityCalculator {
    base_priority: f64,
}

impl PriorityCalculator for TestPriorityCalculator {
    type Item = DebtItem;

    fn calculate_priority(&self, item: &Self::Item) -> f64 {
        let severity_weight = match item.severity {
            Severity::Critical => 1.0,
            Severity::Major => 0.75,
            Severity::Warning => 0.5,
            Severity::Info => 0.25,
        };

        let category_weight = match item.category {
            DebtCategory::Complexity => 0.9,
            DebtCategory::Testing => 0.8,
            DebtCategory::Documentation => 0.6,
            _ => 0.4,
        };

        self.base_priority * severity_weight * category_weight
    }

    fn get_factors(&self, item: &Self::Item) -> Vec<PriorityFactor> {
        vec![
            PriorityFactor {
                name: "severity".to_string(),
                weight: 0.6,
                value: match item.severity {
                    Severity::Critical => 1.0,
                    Severity::Major => 0.75,
                    Severity::Warning => 0.5,
                    Severity::Info => 0.25,
                },
                description: format!("Severity level: {:?}", item.severity),
            },
            PriorityFactor {
                name: "category".to_string(),
                weight: 0.4,
                value: match item.category {
                    DebtCategory::Complexity => 0.9,
                    DebtCategory::Testing => 0.8,
                    _ => 0.5,
                },
                description: format!("Debt category: {:?}", item.category),
            },
        ]
    }
}

struct TestFormatter {
    format_type: String,
}

impl Formatter for TestFormatter {
    type Report = AnalysisResult;

    fn format(&self, report: &Self::Report) -> Result<String> {
        match self.format_type.as_str() {
            "json" => Ok(format!(
                "{{\"total_files\": {}, \"format\": \"json\"}}",
                report.metrics.total_files
            )),
            "markdown" => Ok(format!(
                "# Analysis Report\nTotal files: {}",
                report.metrics.total_files
            )),
            "terminal" => Ok(format!(
                "Analysis complete: {} files processed",
                report.metrics.total_files
            )),
            _ => Ok(format!("Report: {} files", report.metrics.total_files)),
        }
    }

    fn format_name(&self) -> &str {
        &self.format_type
    }
}

#[test]
fn test_complete_container_creation() {
    let container = AppContainerBuilder::new()
        .with_rust_analyzer(TestAnalyzer {
            language: Language::Rust,
            name: "rust".to_string(),
        })
        .with_python_analyzer(TestAnalyzer {
            language: Language::Python,
            name: "python".to_string(),
        })
        .with_js_analyzer(TestAnalyzer {
            language: Language::JavaScript,
            name: "js".to_string(),
        })
        .with_ts_analyzer(TestAnalyzer {
            language: Language::TypeScript,
            name: "ts".to_string(),
        })
        .with_debt_scorer(TestScorer { base_score: 10.0 })
        .with_cache(TestCache::new())
        .with_config(TestConfigProvider::new())
        .with_priority_calculator(TestPriorityCalculator { base_priority: 0.5 })
        .with_json_formatter(TestFormatter {
            format_type: "json".to_string(),
        })
        .with_markdown_formatter(TestFormatter {
            format_type: "markdown".to_string(),
        })
        .with_terminal_formatter(TestFormatter {
            format_type: "terminal".to_string(),
        })
        .build();

    assert!(container.is_ok(), "Container should build successfully");

    let container = container.unwrap();

    // Test analyzer injection
    let rust_result = container.rust_analyzer.analyze("test code".to_string());
    assert!(rust_result.is_ok());
    let module_info = rust_result.unwrap();
    assert_eq!(module_info.language, Language::Rust);
    assert_eq!(module_info.name, "test_module_rust");

    // Test scorer injection
    let test_item = DebtItem {
        id: "test_1".to_string(),
        category: DebtCategory::Complexity,
        severity: Severity::Major,
        description: "Test debt item".to_string(),
        location: SourceLocation {
            file: PathBuf::from("test.rs"),
            line: 10,
            column: 5,
            end_line: Some(20),
            end_column: Some(10),
        },
        impact: 8.0,
        effort: 2.0,
        priority: 0.7,
        suggestions: vec!["Refactor to reduce complexity".to_string()],
    };

    let score = container.debt_scorer.score(&test_item);
    assert_eq!(score, 40.0); // base_score(10) * complexity(2) * high(2)

    // Test config provider
    let threshold = container.config.get("complexity_threshold");
    assert_eq!(threshold, Some("15".to_string()));

    // Test formatters
    let analysis_result = AnalysisResult {
        project_path: PathBuf::from("test_project"),
        modules: vec![],
        debt_items: vec![],
        total_score: 42.0,
        metrics: ProjectMetrics {
            total_files: 42,
            total_functions: 100,
            total_lines: 1000,
            average_complexity: 5.0,
            test_coverage: Some(80.0),
            debt_score: 42.0,
            language_breakdown: HashMap::new(),
        },
        timestamp: chrono::Utc::now(),
    };

    let json_output = container.json_formatter.format(&analysis_result);
    assert!(json_output.is_ok());
    assert!(json_output.unwrap().contains("\"total_files\""));

    let markdown_output = container.markdown_formatter.format(&analysis_result);
    assert!(markdown_output.is_ok());
    assert!(markdown_output.unwrap().contains("# Analysis Report"));

    let terminal_output = container.terminal_formatter.format(&analysis_result);
    assert!(terminal_output.is_ok());
    assert!(terminal_output.unwrap().contains("Analysis complete"));
}

#[test]
fn test_analyzer_factory_integration() {
    let factory = AnalyzerFactory;

    // Test creation of each analyzer type
    let languages = vec![
        Language::Rust,
        Language::Python,
        Language::JavaScript,
        Language::TypeScript,
    ];

    for language in languages {
        let analyzer = factory.create_analyzer(language);

        // Use valid code for each language
        let test_code = match language {
            Language::Rust => "fn main() { println!(\"Hello\"); }",
            Language::Python => "def main():\n    print(\"Hello\")",
            Language::JavaScript => "function main() { console.log(\"Hello\"); }",
            Language::TypeScript => "function main(): void { console.log(\"Hello\"); }",
        };

        // Verify the analyzer can be used
        let result = analyzer.analyze(test_code.to_string());
        assert!(
            result.is_ok(),
            "Failed to analyze for {:?}: {:?}",
            language,
            result
        );

        let module_info = result.unwrap();
        assert_eq!(module_info.language, language);

        // Verify name is correct
        let expected_name = match language {
            Language::Rust => "RustAnalyzer",
            Language::Python => "PythonAnalyzer",
            Language::JavaScript => "JavaScriptAnalyzer",
            Language::TypeScript => "TypeScriptAnalyzer",
        };
        assert_eq!(analyzer.name(), expected_name);
    }
}

#[test]
fn test_service_locator_integration() {
    let mut locator = ServiceLocator::new();

    // Register multiple services
    locator.register(TestScorer { base_score: 15.0 });
    locator.register(TestConfigProvider::new());
    locator.register(TestPriorityCalculator { base_priority: 0.7 });

    // Resolve and use services
    let scorer = locator.resolve::<TestScorer>();
    assert!(scorer.is_some());
    let scorer = scorer.unwrap();

    let test_item = DebtItem {
        id: "test_2".to_string(),
        category: DebtCategory::Testing,
        severity: Severity::Warning,
        description: "Missing tests".to_string(),
        location: SourceLocation {
            file: PathBuf::from("test.rs"),
            line: 5,
            column: 0,
            end_line: None,
            end_column: None,
        },
        impact: 5.0,
        effort: 1.0,
        priority: 0.5,
        suggestions: vec!["Add unit tests".to_string()],
    };

    let score = scorer.score(&test_item);
    assert_eq!(score, 33.75); // base(15) * testing(1.5) * medium(1.5)

    // Test config provider resolution
    let config = locator.resolve::<TestConfigProvider>();
    assert!(config.is_some());
    let config = config.unwrap();
    assert_eq!(config.get("enable_caching"), Some("true".to_string()));

    // Test priority calculator resolution
    let calculator = locator.resolve::<TestPriorityCalculator>();
    assert!(calculator.is_some());
    let calculator = calculator.unwrap();
    let priority = calculator.calculate_priority(&test_item);
    assert!(priority > 0.0 && priority < 1.0);

    // Test non-existent service
    let missing = locator.resolve::<TestCache>();
    assert!(missing.is_none());
}

#[test]
fn test_trait_boundaries_and_contracts() {
    // Test that all trait implementations adhere to their contracts

    // Test Analyzer contract
    let analyzer = TestAnalyzer {
        language: Language::Rust,
        name: "contract_test".to_string(),
    };

    let result = analyzer.analyze("".to_string());
    assert!(result.is_ok(), "Analyzer should handle empty input");

    let result = analyzer.analyze("very long input".repeat(1000));
    assert!(result.is_ok(), "Analyzer should handle large input");

    // Test Cache contract
    let mut cache = TestCache::new();

    // Test get on empty cache
    assert_eq!(cache.get(&"nonexistent".to_string()), None);

    // Test set and get
    cache.set("key1".to_string(), vec![1, 2, 3]);
    assert_eq!(cache.get(&"key1".to_string()), Some(vec![1, 2, 3]));

    // Test stats
    let stats = cache.stats();
    assert_eq!(stats.entries, 1);
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);

    // Test clear
    cache.clear();
    assert_eq!(cache.get(&"key1".to_string()), None);
    let stats = cache.stats();
    assert_eq!(stats.entries, 0);

    // Test Scorer contract
    let scorer = TestScorer { base_score: 1.0 };

    // Test with different severity/category combinations
    for category in &[
        DebtCategory::Complexity,
        DebtCategory::Testing,
        DebtCategory::Documentation,
    ] {
        for severity in &[
            Severity::Info,
            Severity::Warning,
            Severity::Major,
            Severity::Critical,
        ] {
            let item = DebtItem {
                id: format!("test_{:?}_{:?}", category, severity),
                category: *category,
                severity: *severity,
                description: "Test".to_string(),
                location: SourceLocation {
                    file: PathBuf::from("test.rs"),
                    line: 1,
                    column: 0,
                    end_line: None,
                    end_column: None,
                },
                impact: 5.0,
                effort: 1.0,
                priority: 0.5,
                suggestions: vec![],
            };

            let score = scorer.score(&item);
            assert!(score > 0.0, "Score should always be positive");
            assert!(score.is_finite(), "Score should be finite");
        }
    }

    // Test ConfigProvider contract
    let mut config = TestConfigProvider::new();

    // Test get existing
    assert!(config.get("complexity_threshold").is_some());

    // Test get non-existing
    assert!(config.get("nonexistent_key").is_none());

    // Test set and get
    config.set("new_key".to_string(), "new_value".to_string());
    assert_eq!(config.get("new_key"), Some("new_value".to_string()));

    // Test PriorityCalculator contract
    let calculator = TestPriorityCalculator { base_priority: 1.0 };

    let item = DebtItem {
        id: "test_3".to_string(),
        category: DebtCategory::Complexity,
        severity: Severity::Major,
        description: "Complex function".to_string(),
        location: SourceLocation {
            file: PathBuf::from("test.rs"),
            line: 1,
            column: 0,
            end_line: None,
            end_column: None,
        },
        impact: 7.0,
        effort: 3.0,
        priority: 0.6,
        suggestions: vec!["Break down into smaller functions".to_string()],
    };

    let priority = calculator.calculate_priority(&item);
    assert!(
        priority >= 0.0 && priority <= 1.0,
        "Priority should be normalized"
    );

    let factors = calculator.get_factors(&item);
    assert!(!factors.is_empty(), "Should return priority factors");

    // Verify factors sum to reasonable weight
    let total_weight: f64 = factors.iter().map(|f| f.weight).sum();
    assert!(
        (total_weight - 1.0).abs() < 0.01,
        "Factor weights should sum to ~1.0"
    );
}

#[test]
fn test_container_with_arc_sharing() {
    // Test that the Arc-wrapped services can be shared safely
    let container = AppContainerBuilder::new()
        .with_rust_analyzer(TestAnalyzer {
            language: Language::Rust,
            name: "shared_rust".to_string(),
        })
        .with_python_analyzer(TestAnalyzer {
            language: Language::Python,
            name: "shared_python".to_string(),
        })
        .with_js_analyzer(TestAnalyzer {
            language: Language::JavaScript,
            name: "shared_js".to_string(),
        })
        .with_ts_analyzer(TestAnalyzer {
            language: Language::TypeScript,
            name: "shared_ts".to_string(),
        })
        .with_debt_scorer(TestScorer { base_score: 20.0 })
        .with_cache(TestCache::new())
        .with_config(TestConfigProvider::new())
        .with_priority_calculator(TestPriorityCalculator { base_priority: 0.8 })
        .with_json_formatter(TestFormatter {
            format_type: "json".to_string(),
        })
        .with_markdown_formatter(TestFormatter {
            format_type: "markdown".to_string(),
        })
        .with_terminal_formatter(TestFormatter {
            format_type: "terminal".to_string(),
        })
        .build()
        .unwrap();

    // Clone the Arc references
    let scorer1 = Arc::clone(&container.debt_scorer);
    let scorer2 = Arc::clone(&container.debt_scorer);

    // Use from multiple references
    let item = DebtItem {
        id: "test_4".to_string(),
        category: DebtCategory::Complexity,
        severity: Severity::Info,
        description: "Shared test".to_string(),
        location: SourceLocation {
            file: PathBuf::from("test.rs"),
            line: 1,
            column: 0,
            end_line: None,
            end_column: None,
        },
        impact: 3.0,
        effort: 0.5,
        priority: 0.3,
        suggestions: vec![],
    };

    let score1 = scorer1.score(&item);
    let score2 = scorer2.score(&item);

    assert_eq!(score1, score2, "Same scorer should give same results");
    assert_eq!(score1, 40.0); // base(20) * complexity(2) * low(1)
}

#[test]
fn test_error_handling_in_container_builder() {
    // Test various error conditions

    // Missing all required components
    let builder = AppContainerBuilder::new();
    let result = builder.build();
    assert!(result.is_err());

    // Missing specific analyzer
    let builder = AppContainerBuilder::new()
        .with_python_analyzer(TestAnalyzer {
            language: Language::Python,
            name: "python".to_string(),
        })
        .with_js_analyzer(TestAnalyzer {
            language: Language::JavaScript,
            name: "js".to_string(),
        })
        .with_ts_analyzer(TestAnalyzer {
            language: Language::TypeScript,
            name: "ts".to_string(),
        })
        .with_debt_scorer(TestScorer { base_score: 10.0 })
        .with_cache(TestCache::new())
        .with_config(TestConfigProvider::new())
        .with_priority_calculator(TestPriorityCalculator { base_priority: 0.5 })
        .with_json_formatter(TestFormatter {
            format_type: "json".to_string(),
        })
        .with_markdown_formatter(TestFormatter {
            format_type: "markdown".to_string(),
        })
        .with_terminal_formatter(TestFormatter {
            format_type: "terminal".to_string(),
        });

    let result = builder.build();
    assert!(result.is_err());
    if let Err(error_msg) = result {
        assert!(error_msg.contains("Rust analyzer"));
    } else {
        panic!("Expected error, but got success");
    }

    // Missing formatter
    let builder = AppContainerBuilder::new()
        .with_rust_analyzer(TestAnalyzer {
            language: Language::Rust,
            name: "rust".to_string(),
        })
        .with_python_analyzer(TestAnalyzer {
            language: Language::Python,
            name: "python".to_string(),
        })
        .with_js_analyzer(TestAnalyzer {
            language: Language::JavaScript,
            name: "js".to_string(),
        })
        .with_ts_analyzer(TestAnalyzer {
            language: Language::TypeScript,
            name: "ts".to_string(),
        })
        .with_debt_scorer(TestScorer { base_score: 10.0 })
        .with_cache(TestCache::new())
        .with_config(TestConfigProvider::new())
        .with_priority_calculator(TestPriorityCalculator { base_priority: 0.5 })
        .with_json_formatter(TestFormatter {
            format_type: "json".to_string(),
        })
        .with_markdown_formatter(TestFormatter {
            format_type: "markdown".to_string(),
        });
    // Missing terminal formatter

    let result = builder.build();
    assert!(result.is_err());
    if let Err(error_msg) = result {
        assert!(error_msg.contains("Terminal formatter"));
    } else {
        panic!("Expected error, but got success");
    }
}
