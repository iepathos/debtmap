//! Module Function Classifier (Spec 149)
//!
//! Routes module-level functions through multi-signal classification to generate
//! evidence-based split recommendations instead of generic fallbacks.

use crate::analysis::io_detection::Language;
use crate::analysis::multi_signal_aggregation::{
    AggregatedClassification, ResponsibilityAggregator, ResponsibilityCategory, SignalSet,
};
use crate::organization::god_object::ModuleFunctionInfo;
use crate::organization::{ModuleSplit, Priority};
use std::collections::HashMap;

/// Classified function with multi-signal evidence
#[derive(Debug, Clone)]
pub struct ClassifiedFunction {
    pub function: ModuleFunctionInfo,
    pub classification: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: AggregatedClassification,
}

/// Module function classifier using multi-signal aggregation
pub struct ModuleFunctionClassifier {
    aggregator: ResponsibilityAggregator,
    language: Language,
}

impl ModuleFunctionClassifier {
    /// Create new classifier with default configuration
    pub fn new(language: Language) -> Self {
        Self {
            aggregator: ResponsibilityAggregator::new(),
            language,
        }
    }

    /// Classify a single module function using multi-signal analysis
    pub fn classify_function(&self, func: &ModuleFunctionInfo) -> ClassifiedFunction {
        // Skip test functions
        if func.is_test {
            let evidence = AggregatedClassification {
                primary: ResponsibilityCategory::TestFunction,
                confidence: 1.0,
                evidence: vec![],
                alternatives: vec![],
            };

            return ClassifiedFunction {
                function: func.clone(),
                classification: ResponsibilityCategory::TestFunction,
                confidence: 1.0,
                evidence,
            };
        }

        // Build signal set
        let signals = SignalSet {
            io_signal: self.aggregator.collect_io_signal(&func.body, self.language),
            purity_signal: self
                .aggregator
                .collect_purity_signal(&func.body, self.language),
            type_signal: self.collect_type_signal(func),
            name_signal: Some(self.aggregator.collect_name_signal(&func.name)),
            call_graph_signal: None, // TODO: Requires call graph context
            framework_signal: None,  // TODO: Requires file context
        };

        // Aggregate signals
        let evidence = self.aggregator.aggregate(&signals);

        ClassifiedFunction {
            function: func.clone(),
            classification: evidence.primary,
            confidence: evidence.confidence,
            evidence,
        }
    }

    /// Collect type signature signal from function
    fn collect_type_signal(
        &self,
        func: &ModuleFunctionInfo,
    ) -> Option<crate::analysis::multi_signal_aggregation::TypeSignatureClassification> {
        // Convert parameters to the format expected by collect_type_signal
        let params: Vec<(String, String)> = func
            .parameters
            .iter()
            .map(|p| (p.name.clone(), p.type_name.clone()))
            .collect();

        #[allow(deprecated)]
        self.aggregator
            .collect_type_signal(func.return_type.as_deref(), &params)
    }

    /// Classify all module functions and group by responsibility
    pub fn classify_and_group(
        &self,
        functions: &[ModuleFunctionInfo],
    ) -> HashMap<ResponsibilityCategory, Vec<ClassifiedFunction>> {
        let mut groups: HashMap<ResponsibilityCategory, Vec<ClassifiedFunction>> = HashMap::new();

        for func in functions {
            let classified = self.classify_function(func);
            groups
                .entry(classified.classification)
                .or_default()
                .push(classified);
        }

        groups
    }

    /// Generate module splits from classified functions
    pub fn generate_splits(
        &self,
        functions: &[ModuleFunctionInfo],
        min_functions_for_split: usize,
        min_confidence: f64,
    ) -> Vec<ModuleSplit> {
        // Classify and group functions
        let groups = self.classify_and_group(functions);

        let mut splits = Vec::new();

        for (responsibility, classified_functions) in groups {
            // Skip if too few functions or test functions
            if classified_functions.len() < min_functions_for_split
                || responsibility == ResponsibilityCategory::TestFunction
            {
                continue;
            }

            // Calculate aggregate confidence
            let avg_confidence: f64 = classified_functions
                .iter()
                .map(|f| f.confidence)
                .sum::<f64>()
                / classified_functions.len() as f64;

            // Skip if confidence too low
            if avg_confidence < min_confidence {
                continue;
            }

            // Calculate priority based on confidence and function count
            let priority = Self::calculate_priority(avg_confidence, classified_functions.len());

            // Generate split recommendation
            let method_names: Vec<String> = classified_functions
                .iter()
                .map(|f| f.function.name.clone())
                .collect();
            let representative_methods: Vec<String> =
                method_names.iter().take(8).cloned().collect();

            let split = ModuleSplit {
                suggested_name: format!(
                    "{}_module",
                    Self::responsibility_to_snake_case(responsibility)
                ),
                responsibility: responsibility.as_str().to_string(),
                methods_to_move: method_names,
                structs_to_move: vec![],
                estimated_lines: classified_functions
                    .iter()
                    .map(|f| f.function.line_count)
                    .sum(),
                method_count: classified_functions.len(),
                warning: Self::generate_warning(avg_confidence),
                priority,
                cohesion_score: Some(avg_confidence),
                dependencies_in: vec![],
                dependencies_out: vec![],
                domain: String::new(),
                rationale: Some(Self::aggregate_evidence(&classified_functions)),
                method: crate::organization::SplitAnalysisMethod::MethodBased,
                severity: None,
                interface_estimate: None,
                classification_evidence: Self::build_aggregate_evidence(&classified_functions),
                representative_methods,
                fields_needed: vec![],
                trait_suggestion: None,
                behavior_category: Some(responsibility.as_str().to_string()),
                core_type: None,
                data_flow: vec![],
                suggested_type_definition: None,
                data_flow_stage: None,
                pipeline_position: None,
                input_types: vec![],
                output_types: vec![],
                merge_history: vec![],
                alternative_names: vec![],
                naming_confidence: None,
                naming_strategy: None,
                cluster_quality: None,
            };

            splits.push(split);
        }

        // Sort by confidence and size
        splits.sort_by(|a, b| {
            b.cohesion_score
                .unwrap_or(0.0)
                .partial_cmp(&a.cohesion_score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.method_count.cmp(&a.method_count))
        });

        splits
    }

    /// Calculate priority based on confidence and function count
    fn calculate_priority(confidence: f64, function_count: usize) -> Priority {
        if confidence > 0.70 && function_count > 10 {
            Priority::High
        } else if confidence > 0.50 || function_count > 5 {
            Priority::Medium
        } else {
            Priority::Low
        }
    }

    /// Generate warning message for low confidence splits
    fn generate_warning(confidence: f64) -> Option<String> {
        if confidence < 0.50 {
            Some(format!(
                "Low confidence ({:.2}) - manual review recommended",
                confidence
            ))
        } else {
            None
        }
    }

    /// Aggregate evidence from multiple classified functions
    fn aggregate_evidence(functions: &[ClassifiedFunction]) -> String {
        use std::collections::HashMap;

        let mut signal_counts: HashMap<String, usize> = HashMap::new();
        let mut total_confidence = 0.0;

        for func in functions {
            for evidence in &func.evidence.evidence {
                *signal_counts
                    .entry(evidence.description.clone())
                    .or_insert(0) += 1;
            }
            total_confidence += func.confidence;
        }

        let avg_confidence = total_confidence / functions.len() as f64;

        // Find most common signals
        let mut signal_list: Vec<_> = signal_counts.into_iter().collect();
        signal_list.sort_by(|a, b| b.1.cmp(&a.1));

        let top_signals: Vec<String> = signal_list
            .iter()
            .take(3)
            .map(|(signal, count)| format!("{} ({} functions)", signal, count))
            .collect();

        format!(
            "Avg confidence: {:.2}. Top signals: {}",
            avg_confidence,
            top_signals.join(", ")
        )
    }

    /// Build aggregate classification evidence from multiple functions
    fn build_aggregate_evidence(
        functions: &[ClassifiedFunction],
    ) -> Option<AggregatedClassification> {
        if functions.is_empty() {
            return None;
        }

        // Use the first function's evidence as template and merge others
        let first = &functions[0];
        let avg_confidence =
            functions.iter().map(|f| f.confidence).sum::<f64>() / functions.len() as f64;

        // Collect all evidence pieces from all functions
        let mut all_evidence = Vec::new();
        for func in functions {
            all_evidence.extend(func.evidence.evidence.clone());
        }

        // Return aggregated classification
        Some(AggregatedClassification {
            primary: first.evidence.primary,
            confidence: avg_confidence,
            evidence: all_evidence,
            alternatives: first.evidence.alternatives.clone(),
        })
    }

    /// Convert responsibility category to snake_case
    fn responsibility_to_snake_case(responsibility: ResponsibilityCategory) -> String {
        responsibility
            .as_str()
            .to_lowercase()
            .replace([' ', '/'], "_")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_function_with_formatting_return_type() {
        let classifier = ModuleFunctionClassifier::new(Language::Rust);

        let func = ModuleFunctionInfo {
            name: "format_output".to_string(),
            body: "fn format_output(data: &Data) -> String { data.to_string() }".to_string(),
            return_type: Some("String".to_string()),
            parameters: vec![],
            line_count: 1,
            is_public: true,
            is_async: false,
            is_test: false,
        };

        let classified = classifier.classify_function(&func);

        // Should detect formatting based on return type
        assert!(classified.confidence > 0.0);
    }

    #[test]
    fn test_skip_test_functions() {
        let classifier = ModuleFunctionClassifier::new(Language::Rust);

        let func = ModuleFunctionInfo {
            name: "test_something".to_string(),
            body: "fn test_something() { assert!(true); }".to_string(),
            return_type: None,
            parameters: vec![],
            line_count: 1,
            is_public: false,
            is_async: false,
            is_test: true,
        };

        let classified = classifier.classify_function(&func);

        assert_eq!(
            classified.classification,
            ResponsibilityCategory::TestFunction
        );
        assert_eq!(classified.confidence, 1.0);
    }

    #[test]
    fn test_generate_splits_minimum_threshold() {
        let classifier = ModuleFunctionClassifier::new(Language::Rust);

        // Create 2 functions (below minimum of 3)
        let functions = vec![
            ModuleFunctionInfo {
                name: "func1".to_string(),
                body: "fn func1() {}".to_string(),
                return_type: None,
                parameters: vec![],
                line_count: 1,
                is_public: true,
                is_async: false,
                is_test: false,
            },
            ModuleFunctionInfo {
                name: "func2".to_string(),
                body: "fn func2() {}".to_string(),
                return_type: None,
                parameters: vec![],
                line_count: 1,
                is_public: true,
                is_async: false,
                is_test: false,
            },
        ];

        let splits = classifier.generate_splits(&functions, 3, 0.30);

        // Should not generate split with only 2 functions
        assert_eq!(splits.len(), 0);
    }
}
