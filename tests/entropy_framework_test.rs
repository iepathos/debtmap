#[cfg(test)]
mod entropy_framework_tests {
    use debtmap::complexity::entropy_core::{
        EntropyConfig, EntropyToken, LanguageEntropyAnalyzer, PatternMetrics,
        UniversalEntropyCalculator,
    };
    use debtmap::complexity::entropy_traits::GenericToken;

    #[test]
    fn test_shannon_entropy_calculation() {
        let config = EntropyConfig::default();
        let calculator = UniversalEntropyCalculator::new(config);

        // Test with uniform distribution
        let tokens = vec![
            GenericToken::keyword("if".to_string()),
            GenericToken::identifier("var".to_string()),
            GenericToken::operator("=".to_string()),
            GenericToken::literal("42".to_string()),
        ];

        let entropy = calculator.shannon_entropy(&tokens);
        assert!(entropy > 0.0 && entropy <= 1.0);
    }

    #[test]
    fn test_entropy_with_no_tokens() {
        let config = EntropyConfig::default();
        let calculator = UniversalEntropyCalculator::new(config);

        let tokens: Vec<GenericToken> = vec![];
        let entropy = calculator.shannon_entropy(&tokens);
        assert_eq!(entropy, 0.0);
    }

    #[test]
    fn test_entropy_with_identical_tokens() {
        let config = EntropyConfig::default();
        let calculator = UniversalEntropyCalculator::new(config);

        // All tokens of same type - low entropy
        let tokens = vec![
            GenericToken::keyword("if".to_string()),
            GenericToken::keyword("while".to_string()),
            GenericToken::keyword("for".to_string()),
            GenericToken::keyword("match".to_string()),
        ];

        let entropy = calculator.shannon_entropy(&tokens);
        assert_eq!(entropy, 0.0); // All same category
    }

    #[test]
    fn test_complexity_adjustment() {
        let config = EntropyConfig::default();
        let calculator = UniversalEntropyCalculator::new(config);

        // Test with different pattern and similarity values
        let adjusted = calculator.adjust_complexity(0.8, 0.3, 0.2);
        assert!(adjusted > 0.0 && adjusted < 0.8);
    }

    #[test]
    fn test_pattern_metrics() {
        let mut metrics = PatternMetrics::new();
        metrics.total_patterns = 10;
        metrics.unique_patterns = 5;
        metrics.calculate_repetition();

        assert_eq!(metrics.repetition_ratio, 0.5);
    }

    #[test]
    fn test_token_category_equality() {
        let token1 = GenericToken::keyword("if".to_string());
        let token2 = GenericToken::keyword("else".to_string());

        // Same category but different value
        assert_eq!(
            EntropyToken::to_category(&token1),
            EntropyToken::to_category(&token2)
        );
    }

    #[test]
    fn test_token_weights() {
        let keyword = GenericToken::keyword("if".to_string());
        let literal = GenericToken::literal("42".to_string());
        let control_flow = GenericToken::control_flow("while".to_string());

        assert_eq!(EntropyToken::weight(&keyword), 1.0);
        assert_eq!(EntropyToken::weight(&literal), 0.3);
        assert_eq!(EntropyToken::weight(&control_flow), 1.2);
    }

    #[test]
    fn test_cache_functionality() {
        let config = EntropyConfig {
            enabled: true,
            max_cache_size: 2,
            base_threshold: 0.5,
            pattern_weight: 0.3,
            similarity_weight: 0.2,
        };
        let mut calculator = UniversalEntropyCalculator::new(config);

        // Mock analyzer for testing
        struct MockAnalyzer;
        impl LanguageEntropyAnalyzer for MockAnalyzer {
            type AstNode = String;
            type Token = GenericToken;

            fn extract_tokens(&self, _node: &Self::AstNode) -> Vec<Self::Token> {
                vec![GenericToken::keyword("test".to_string())]
            }

            fn detect_patterns(&self, _node: &Self::AstNode) -> PatternMetrics {
                PatternMetrics::new()
            }

            fn calculate_branch_similarity(&self, _node: &Self::AstNode) -> f64 {
                0.0
            }

            fn analyze_structure(&self, _node: &Self::AstNode) -> (usize, u32) {
                (0, 0)
            }

            fn generate_cache_key(&self, node: &Self::AstNode) -> String {
                node.clone()
            }
        }

        let analyzer = MockAnalyzer;
        let node1 = "function1".to_string();
        let node2 = "function2".to_string();

        // First call should miss cache
        let _score1 = calculator.calculate(&analyzer, &node1);
        let (hits, misses, _) = calculator.cache_stats();
        assert_eq!(hits, 0);
        assert_eq!(misses, 1);

        // Second call with same key should hit cache
        let _score1_again = calculator.calculate(&analyzer, &node1);
        let (hits, misses, _) = calculator.cache_stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);

        // Different key should miss cache
        let _score2 = calculator.calculate(&analyzer, &node2);
        let (hits, misses, hit_rate) = calculator.cache_stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 2);
        assert_eq!(hit_rate, 1.0 / 3.0);
    }
}
