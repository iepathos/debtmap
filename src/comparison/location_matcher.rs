use anyhow::Result;

use crate::priority::{UnifiedAnalysis, UnifiedDebtItem};

/// Location pattern for matching debt items
#[derive(Debug, Clone, PartialEq)]
pub enum LocationPattern {
    /// Exact match: file:function:line
    Exact {
        file: String,
        function: String,
        line: usize,
    },
    /// Function match: file:function (any line)
    Function { file: String, function: String },
    /// File match: file (all items in file)
    File { file: String },
    /// Line range match: file:*:line (any function at line)
    LineRange { file: String, line: usize },
}

impl LocationPattern {
    /// Parse location string into appropriate pattern
    pub fn parse(location: &str) -> Result<Self> {
        let parts: Vec<&str> = location.split(':').collect();
        match parts.len() {
            1 => Ok(Self::File {
                file: parts[0].to_string(),
            }),
            2 => Ok(Self::Function {
                file: parts[0].to_string(),
                function: parts[1].to_string(),
            }),
            3 => {
                if parts[1] == "*" {
                    Ok(Self::LineRange {
                        file: parts[0].to_string(),
                        line: parts[2].parse()?,
                    })
                } else {
                    Ok(Self::Exact {
                        file: parts[0].to_string(),
                        function: parts[1].to_string(),
                        line: parts[2].parse()?,
                    })
                }
            }
            _ => Err(anyhow::anyhow!(
                "Invalid location format: {}. Expected file[:function[:line]]",
                location
            )),
        }
    }
}

/// Matching strategy used to find items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchStrategy {
    /// Exact file:function:line match
    Exact,
    /// Function-level match (file:function)
    FunctionLevel,
    /// Approximate name match (fuzzy)
    ApproximateName,
    /// File-level match (all items in file)
    FileLevel,
}

impl MatchStrategy {
    /// Get confidence score for this strategy
    pub fn confidence(&self) -> f64 {
        match self {
            Self::Exact => 1.0,
            Self::FunctionLevel => 0.8,
            Self::ApproximateName => 0.6,
            Self::FileLevel => 0.4,
        }
    }
}

/// Result of a location match operation
#[derive(Debug, Clone)]
pub struct MatchResult<'a> {
    pub items: Vec<&'a UnifiedDebtItem>,
    pub strategy: MatchStrategy,
    pub confidence: f64,
}

/// Handles location matching with multiple strategies
pub struct LocationMatcher;

impl LocationMatcher {
    pub fn new() -> Self {
        Self
    }

    /// Find items matching the given location using cascading strategies
    pub fn find_matches<'a>(
        &self,
        analysis: &'a UnifiedAnalysis,
        location: &str,
    ) -> Result<MatchResult<'a>> {
        let pattern = LocationPattern::parse(location)?;

        // Try strategies in order of specificity
        if let Some(result) = self.try_exact_match(analysis, &pattern) {
            return Ok(result);
        }

        if let Some(result) = self.try_function_match(analysis, &pattern) {
            return Ok(result);
        }

        if let Some(result) = self.try_approximate_match(analysis, &pattern) {
            return Ok(result);
        }

        if let Some(result) = self.try_file_match(analysis, &pattern) {
            return Ok(result);
        }

        Err(anyhow::anyhow!(
            "No items found matching location: {} (tried all strategies: exact, function-level, approximate, file-level)",
            location
        ))
    }

    /// Try exact match: file:function:line
    fn try_exact_match<'a>(
        &self,
        analysis: &'a UnifiedAnalysis,
        pattern: &LocationPattern,
    ) -> Option<MatchResult<'a>> {
        let (file, function, line) = match pattern {
            LocationPattern::Exact {
                file,
                function,
                line,
            } => (file, function, *line),
            _ => return None,
        };

        let item = analysis.items.iter().find(|item| {
            let item_file = normalize_path(&item.location.file);
            let target_file = normalize_path_str(file);

            item_file == target_file
                && item.location.function == *function
                && item.location.line == line
        })?;

        Some(MatchResult {
            items: vec![item],
            strategy: MatchStrategy::Exact,
            confidence: MatchStrategy::Exact.confidence(),
        })
    }

    /// Try function-level match: file:function (any line)
    fn try_function_match<'a>(
        &self,
        analysis: &'a UnifiedAnalysis,
        pattern: &LocationPattern,
    ) -> Option<MatchResult<'a>> {
        let (file, function) = match pattern {
            LocationPattern::Exact { file, function, .. }
            | LocationPattern::Function { file, function } => (file, function),
            _ => return None,
        };

        let items: Vec<&UnifiedDebtItem> = analysis
            .items
            .iter()
            .filter(|item| {
                let item_file = normalize_path(&item.location.file);
                let target_file = normalize_path_str(file);

                item_file == target_file && item.location.function == *function
            })
            .collect();

        if items.is_empty() {
            None
        } else {
            Some(MatchResult {
                items,
                strategy: MatchStrategy::FunctionLevel,
                confidence: MatchStrategy::FunctionLevel.confidence(),
            })
        }
    }

    /// Try approximate name match: fuzzy match on function/struct names
    fn try_approximate_match<'a>(
        &self,
        analysis: &'a UnifiedAnalysis,
        pattern: &LocationPattern,
    ) -> Option<MatchResult<'a>> {
        let (file, target_name) = match pattern {
            LocationPattern::Exact { file, function, .. }
            | LocationPattern::Function { file, function } => (file, function),
            _ => return None,
        };

        let target_file = normalize_path_str(file);

        // Find items in the same file with similar names
        let candidates: Vec<(&UnifiedDebtItem, f64)> = analysis
            .items
            .iter()
            .filter(|item| normalize_path(&item.location.file) == target_file)
            .filter_map(|item| {
                let similarity = calculate_similarity(target_name, &item.location.function);
                if similarity >= 0.5 {
                    Some((item, similarity))
                } else {
                    None
                }
            })
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Find the best match(es) - take all items with the highest similarity score
        let max_similarity = candidates
            .iter()
            .map(|(_, sim)| *sim)
            .fold(0.0f64, |a, b| a.max(b));

        let items: Vec<&UnifiedDebtItem> = candidates
            .iter()
            .filter(|(_, sim)| *sim == max_similarity)
            .map(|(item, _)| *item)
            .collect();

        Some(MatchResult {
            items,
            strategy: MatchStrategy::ApproximateName,
            confidence: max_similarity * MatchStrategy::ApproximateName.confidence(),
        })
    }

    /// Try file-level match: all items in the file
    fn try_file_match<'a>(
        &self,
        analysis: &'a UnifiedAnalysis,
        pattern: &LocationPattern,
    ) -> Option<MatchResult<'a>> {
        let file = match pattern {
            LocationPattern::File { file } => file,
            LocationPattern::Exact { file, .. }
            | LocationPattern::Function { file, .. }
            | LocationPattern::LineRange { file, .. } => file,
        };

        let target_file = normalize_path_str(file);

        let items: Vec<&UnifiedDebtItem> = analysis
            .items
            .iter()
            .filter(|item| normalize_path(&item.location.file) == target_file)
            .collect();

        if items.is_empty() {
            None
        } else {
            // Confidence decreases with more items (less specific)
            let confidence =
                MatchStrategy::FileLevel.confidence() * (1.0 / (items.len() as f64).sqrt());
            Some(MatchResult {
                items,
                strategy: MatchStrategy::FileLevel,
                confidence: confidence.max(0.3), // Minimum confidence
            })
        }
    }
}

impl Default for LocationMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalize a path for comparison
fn normalize_path(path: &std::path::Path) -> String {
    let path_str = path.to_string_lossy();
    normalize_path_str(&path_str)
}

/// Normalize a path string for comparison
fn normalize_path_str(path: &str) -> String {
    path.strip_prefix("./").unwrap_or(path).to_string()
}

/// Calculate similarity between two strings (simple prefix-based)
fn calculate_similarity(a: &str, b: &str) -> f64 {
    if a == b {
        return 1.0;
    }

    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    // Check if one is a prefix of the other
    if a_lower.starts_with(&b_lower) || b_lower.starts_with(&a_lower) {
        let shorter = a_lower.len().min(b_lower.len());
        let longer = a_lower.len().max(b_lower.len());
        return shorter as f64 / longer as f64;
    }

    // Check if they share a common prefix
    let common_prefix_len = a_lower
        .chars()
        .zip(b_lower.chars())
        .take_while(|(ca, cb)| ca == cb)
        .count();

    let max_len = a_lower.len().max(b_lower.len());
    if max_len == 0 {
        return 0.0;
    }

    // Simple similarity based on common prefix length
    common_prefix_len as f64 / max_len as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{
        unified_scorer::{Location, UnifiedScore},
        DebtType, FunctionRole, ImpactMetrics,
    };
    use im::Vector;
    use std::path::PathBuf;

    fn create_test_item(file: &str, function: &str, line: usize) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from(file),
                function: function.to_string(),
                line,
            },
            debt_type: DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 10,
                cognitive: 20,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 0.0,
                dependency_factor: 0.0,
                role_multiplier: 1.0,
                final_score: 50.0,
                pre_adjustment_score: None,
                adjustment_applied: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: crate::priority::ActionableRecommendation {
                primary_action: "Test".to_string(),
                rationale: "Test".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 2,
            function_length: 50,
            cyclomatic_complexity: 10,
            cognitive_complexity: 20,
            entropy_details: None,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
        }
    }

    fn create_test_analysis(items: Vec<UnifiedDebtItem>) -> UnifiedAnalysis {
        UnifiedAnalysis {
            items: Vector::from(items),
            file_items: Vector::new(),
            total_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: 50.0,
            debt_density: 0.0,
            total_lines_of_code: 1000,
            call_graph: crate::priority::CallGraph::new(),
            data_flow_graph: crate::data_flow::DataFlowGraph::new(),
            overall_coverage: None,
            has_coverage_data: false,
            timings: None,
        }
    }

    #[test]
    fn test_parse_exact_location() {
        let pattern = LocationPattern::parse("src/main.rs:func:42").unwrap();
        assert_eq!(
            pattern,
            LocationPattern::Exact {
                file: "src/main.rs".to_string(),
                function: "func".to_string(),
                line: 42
            }
        );
    }

    #[test]
    fn test_parse_function_location() {
        let pattern = LocationPattern::parse("src/main.rs:func").unwrap();
        assert_eq!(
            pattern,
            LocationPattern::Function {
                file: "src/main.rs".to_string(),
                function: "func".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_file_location() {
        let pattern = LocationPattern::parse("src/main.rs").unwrap();
        assert_eq!(
            pattern,
            LocationPattern::File {
                file: "src/main.rs".to_string()
            }
        );
    }

    #[test]
    fn test_parse_line_range() {
        let pattern = LocationPattern::parse("src/main.rs:*:42").unwrap();
        assert_eq!(
            pattern,
            LocationPattern::LineRange {
                file: "src/main.rs".to_string(),
                line: 42
            }
        );
    }

    #[test]
    fn test_exact_match() {
        let analysis = create_test_analysis(vec![create_test_item("src/main.rs", "func", 42)]);
        let matcher = LocationMatcher::new();

        let result = matcher
            .find_matches(&analysis, "src/main.rs:func:42")
            .unwrap();
        assert_eq!(result.strategy, MatchStrategy::Exact);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn test_exact_match_with_path_normalization() {
        let analysis = create_test_analysis(vec![create_test_item("./src/main.rs", "func", 42)]);
        let matcher = LocationMatcher::new();

        let result = matcher
            .find_matches(&analysis, "src/main.rs:func:42")
            .unwrap();
        assert_eq!(result.strategy, MatchStrategy::Exact);
        assert_eq!(result.items.len(), 1);
    }

    #[test]
    fn test_function_level_match() {
        let analysis = create_test_analysis(vec![
            create_test_item("src/main.rs", "func", 42),
            create_test_item("src/main.rs", "func", 50),
        ]);
        let matcher = LocationMatcher::new();

        // Try exact match with wrong line, should fall back to function-level
        let result = matcher
            .find_matches(&analysis, "src/main.rs:func:99")
            .unwrap();
        assert_eq!(result.strategy, MatchStrategy::FunctionLevel);
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.confidence, 0.8);
    }

    #[test]
    fn test_approximate_match() {
        let analysis = create_test_analysis(vec![
            create_test_item("src/main.rs", "EnhancedMarkdownWriter", 10),
            create_test_item("src/main.rs", "other_func", 20),
        ]);
        let matcher = LocationMatcher::new();

        // Try to match with similar name
        let result = matcher
            .find_matches(&analysis, "src/main.rs:EnhancedMarkdown:1")
            .unwrap();
        assert_eq!(result.strategy, MatchStrategy::ApproximateName);
        assert_eq!(result.items.len(), 1);
        // Confidence is similarity * strategy base confidence
        // "EnhancedMarkdown" vs "EnhancedMarkdownWriter" = 16/23 * 0.6 ≈ 0.42
        assert!(result.confidence > 0.3);
    }

    #[test]
    fn test_file_level_match() {
        let analysis = create_test_analysis(vec![
            create_test_item("src/main.rs", "func1", 10),
            create_test_item("src/main.rs", "func2", 20),
            create_test_item("src/other.rs", "func3", 30),
        ]);
        let matcher = LocationMatcher::new();

        // Try to match non-existent function, should fall back to file-level
        let result = matcher
            .find_matches(&analysis, "src/main.rs:nonexistent:1")
            .unwrap();
        assert_eq!(result.strategy, MatchStrategy::FileLevel);
        assert_eq!(result.items.len(), 2);
        assert!(result.confidence >= 0.3);
    }

    #[test]
    fn test_no_match() {
        let analysis = create_test_analysis(vec![create_test_item("src/main.rs", "func", 42)]);
        let matcher = LocationMatcher::new();

        let result = matcher.find_matches(&analysis, "src/other.rs:func:42");
        assert!(result.is_err());
    }

    #[test]
    fn test_similarity_exact() {
        assert_eq!(calculate_similarity("func", "func"), 1.0);
    }

    #[test]
    fn test_similarity_prefix() {
        let sim = calculate_similarity("EnhancedMarkdownWriter", "EnhancedMarkdown");
        assert!(sim > 0.5);
        assert!(sim < 1.0);
    }

    #[test]
    fn test_similarity_different() {
        let sim = calculate_similarity("func1", "other");
        assert!(sim < 0.5);
    }
}
