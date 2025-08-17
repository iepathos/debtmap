use super::ModuleType;
use std::path::Path;

pub struct ModuleClassifier {
    test_patterns: Vec<String>,
    benchmark_patterns: Vec<String>,
    example_patterns: Vec<String>,
    doc_patterns: Vec<String>,
}

impl Default for ModuleClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleClassifier {
    pub fn new() -> Self {
        Self {
            test_patterns: vec![
                "tests/".to_string(),
                "/tests/".to_string(),
                "_test.rs".to_string(),
                "_tests.rs".to_string(),
                "/test.rs".to_string(),
                "test_".to_string(),
                "integration".to_string(),
                "spec/".to_string(),
                "__tests__/".to_string(),
            ],
            benchmark_patterns: vec![
                "benches/".to_string(),
                "/benches/".to_string(),
                "benchmark".to_string(),
                "_bench.rs".to_string(),
                "perf_".to_string(),
            ],
            example_patterns: vec![
                "examples/".to_string(),
                "/examples/".to_string(),
                "example_".to_string(),
                "_example.rs".to_string(),
                "demo_".to_string(),
                "_demo.rs".to_string(),
            ],
            doc_patterns: vec![
                "doc/".to_string(),
                "/doc/".to_string(),
                "docs/".to_string(),
                "/docs/".to_string(),
                "doc_test".to_string(),
            ],
        }
    }

    pub fn classify_module(&self, file_path: &Path) -> ModuleType {
        let path_str = file_path.to_string_lossy().to_lowercase();

        // Check explicit test directories and files
        if self.is_test_module(&path_str) {
            return ModuleType::Test;
        }

        // Check benchmark patterns
        if self.is_benchmark_module(&path_str) {
            return ModuleType::Benchmark;
        }

        // Check example patterns
        if self.is_example_module(&path_str) {
            return ModuleType::Example;
        }

        // Check documentation patterns
        if self.is_documentation_module(&path_str) {
            return ModuleType::Documentation;
        }

        // Analyze content for utility vs production
        if self.is_utility_module(&path_str) {
            ModuleType::Utility
        } else if self.is_infrastructure_module(&path_str) {
            ModuleType::Infrastructure
        } else {
            ModuleType::Production
        }
    }

    fn is_test_module(&self, path: &str) -> bool {
        // Standard test patterns
        path.starts_with("tests/")
            || path.contains("/tests/")
            || path.ends_with("_test.rs")
            || path.ends_with("_tests.rs")
            || path.ends_with("/test.rs")
            || path.contains("test_")
            // Integration test patterns
            || (path.contains("integration") && path.contains("test"))
            // Framework-specific patterns
            || path.contains("spec/")
            || path.contains("__tests__/")
    }

    fn is_benchmark_module(&self, path: &str) -> bool {
        path.starts_with("benches/")
            || path.contains("/benches/")
            || path.contains("benchmark")
            || path.contains("_bench.rs")
            || path.contains("perf_")
    }

    fn is_example_module(&self, path: &str) -> bool {
        path.starts_with("examples/")
            || path.contains("/examples/")
            || path.contains("example_")
            || path.ends_with("_example.rs")
            || path.contains("demo_")
            || path.ends_with("_demo.rs")
    }

    fn is_documentation_module(&self, path: &str) -> bool {
        path.starts_with("doc/")
            || path.contains("/doc/")
            || path.starts_with("docs/")
            || path.contains("/docs/")
            || path.contains("doc_test")
    }

    fn is_utility_module(&self, path: &str) -> bool {
        path.contains("util/")
            || path.contains("utils/")
            || path.contains("helper")
            || path.contains("common/")
            || path.contains("shared/")
    }

    fn is_infrastructure_module(&self, path: &str) -> bool {
        path.contains("config/")
            || path.contains("setup/")
            || path.contains("init/")
            || path.contains("bootstrap/")
            || path.contains("migration/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_classification_test_files() {
        let classifier = ModuleClassifier::new();

        assert_eq!(
            classifier.classify_module(Path::new("tests/integration_test.rs")),
            ModuleType::Test
        );
        assert_eq!(
            classifier.classify_module(Path::new("src/lib_test.rs")),
            ModuleType::Test
        );
        assert_eq!(
            classifier.classify_module(Path::new("tests/fixtures/data.rs")),
            ModuleType::Test
        );
    }

    #[test]
    fn test_module_classification_benchmark_files() {
        let classifier = ModuleClassifier::new();

        assert_eq!(
            classifier.classify_module(Path::new("benches/performance.rs")),
            ModuleType::Benchmark
        );
        assert_eq!(
            classifier.classify_module(Path::new("src/perf_test.rs")),
            ModuleType::Benchmark
        );
    }

    #[test]
    fn test_module_classification_production_files() {
        let classifier = ModuleClassifier::new();

        assert_eq!(
            classifier.classify_module(Path::new("src/main.rs")),
            ModuleType::Production
        );
        assert_eq!(
            classifier.classify_module(Path::new("src/lib.rs")),
            ModuleType::Production
        );
        assert_eq!(
            classifier.classify_module(Path::new("src/business/logic.rs")),
            ModuleType::Production
        );
    }

    #[test]
    fn test_module_classification_utility_files() {
        let classifier = ModuleClassifier::new();

        assert_eq!(
            classifier.classify_module(Path::new("src/utils/helpers.rs")),
            ModuleType::Utility
        );
        assert_eq!(
            classifier.classify_module(Path::new("src/common/shared.rs")),
            ModuleType::Utility
        );
    }
}
