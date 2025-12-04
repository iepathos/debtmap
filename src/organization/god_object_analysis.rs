use std::collections::HashMap;

use super::confidence::MINIMUM_CONFIDENCE;
use super::god_object::thresholds::{ensure_not_reserved, GodObjectThresholds};
use super::god_object::types::*;

pub fn calculate_god_object_score(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    thresholds: &GodObjectThresholds,
) -> f64 {
    let method_factor = (method_count as f64 / thresholds.max_methods as f64).min(3.0);
    let field_factor = (field_count as f64 / thresholds.max_fields as f64).min(3.0);
    let responsibility_factor = (responsibility_count as f64 / 3.0).min(3.0);
    let size_factor = (lines_of_code as f64 / thresholds.max_lines as f64).min(3.0);

    // Calculate violation count for minimum score determination
    let mut violation_count = 0;
    if method_count > thresholds.max_methods {
        violation_count += 1;
    }
    if field_count > thresholds.max_fields {
        violation_count += 1;
    }
    if responsibility_count > thresholds.max_traits {
        violation_count += 1;
    }
    if lines_of_code > thresholds.max_lines {
        violation_count += 1;
    }

    // Exponential scaling for severe violations
    let base_score = method_factor * field_factor * responsibility_factor * size_factor;

    // Apply appropriate scoring based on violation severity
    // More nuanced approach to prevent over-flagging moderate files
    if violation_count > 0 {
        // Graduated minimum scores based on violation count
        let base_min_score = match violation_count {
            1 => 30.0, // Single violation: Moderate score
            2 => 50.0, // Two violations: Borderline CRITICAL
            _ => 70.0, // Three+ violations: Likely CRITICAL
        };

        // Reduced multiplier from 50.0 to 20.0 for more conservative scoring
        let score = base_score * 20.0 * (violation_count as f64);
        score.max(base_min_score)
    } else {
        base_score * 10.0
    }
}

/// Calculate complexity-weighted god object score.
///
/// Unlike raw method counting, this function weights each method by its
/// cyclomatic complexity, ensuring that 100 simple functions (complexity 1-3)
/// score better than 10 complex functions (complexity 17+).
///
/// # Arguments
///
/// * `weighted_method_count` - Sum of complexity weights for all functions
/// * `field_count` - Number of fields in the type
/// * `responsibility_count` - Number of distinct responsibilities
/// * `lines_of_code` - Total lines of code
/// * `avg_complexity` - Average cyclomatic complexity across functions
/// * `thresholds` - God object thresholds for the language
///
/// # Returns
///
/// God object score (0-100+). Scores >70 indicate definite god objects.
pub fn calculate_god_object_score_weighted(
    weighted_method_count: f64,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    avg_complexity: f64,
    thresholds: &GodObjectThresholds,
) -> f64 {
    // Use weighted count instead of raw count
    let method_factor = (weighted_method_count / thresholds.max_methods as f64).min(3.0);
    let field_factor = (field_count as f64 / thresholds.max_fields as f64).min(3.0);
    let responsibility_factor = (responsibility_count as f64 / 3.0).min(3.0);
    let size_factor = (lines_of_code as f64 / thresholds.max_lines as f64).min(3.0);

    // Add complexity bonus/penalty
    let complexity_factor = if avg_complexity < 3.0 {
        0.7 // Reward simple functions
    } else if avg_complexity > 10.0 {
        1.5 // Penalize complex functions
    } else {
        1.0
    };

    // Calculate violation count for minimum score determination
    let mut violation_count = 0;
    if weighted_method_count > thresholds.max_methods as f64 {
        violation_count += 1;
    }
    if field_count > thresholds.max_fields {
        violation_count += 1;
    }
    if responsibility_count > thresholds.max_traits {
        violation_count += 1;
    }
    if lines_of_code > thresholds.max_lines {
        violation_count += 1;
    }

    // Exponential scaling for severe violations
    let base_score = method_factor * field_factor * responsibility_factor * size_factor;

    // Apply complexity factor and ensure appropriate score for violations
    // Scale scores more conservatively to prevent small files from being CRITICAL
    if violation_count > 0 {
        // More nuanced minimum scores based on violation severity
        // 1 violation (e.g., just responsibilities): 30-50 range
        // 2 violations: 50-70 range
        // 3+ violations: 70+ range (CRITICAL territory)
        let base_min_score = match violation_count {
            1 => 30.0, // Moderate threshold - won't trigger CRITICAL (< 50)
            2 => 50.0, // High threshold - borderline CRITICAL
            _ => 70.0, // Multiple violations - likely CRITICAL
        };

        // Reduced multiplier from 50.0 to 20.0 for more conservative scoring
        let score = base_score * 20.0 * complexity_factor * (violation_count as f64);
        score.max(base_min_score)
    } else {
        base_score * 10.0 * complexity_factor
    }
}

pub fn determine_confidence(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    complexity_sum: u32,
    thresholds: &GodObjectThresholds,
) -> GodObjectConfidence {
    let mut violations = 0;

    if method_count > thresholds.max_methods {
        violations += 1;
    }
    if field_count > thresholds.max_fields {
        violations += 1;
    }
    if responsibility_count > thresholds.max_traits {
        violations += 1;
    }
    if lines_of_code > thresholds.max_lines {
        violations += 1;
    }
    if complexity_sum > thresholds.max_complexity {
        violations += 1;
    }

    match violations {
        5 => GodObjectConfidence::Definite,
        3..=4 => GodObjectConfidence::Probable,
        1..=2 => GodObjectConfidence::Possible,
        _ => GodObjectConfidence::NotGodObject,
    }
}

pub fn group_methods_by_responsibility(methods: &[String]) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();

    for method in methods {
        let result = infer_responsibility_with_confidence(method, None);

        // If confidence is too low (None category), keep method in original location
        // by assigning it to "unclassified" group
        let responsibility = result
            .category
            .unwrap_or_else(|| "unclassified".to_string());

        groups
            .entry(responsibility)
            .or_default()
            .push(method.clone());
    }

    groups
}

/// Infer responsibility using both I/O detection and name-based heuristics.
///
/// This provides more accurate classification than name-based heuristics alone
/// by analyzing actual I/O operations in the function body.
///
/// # Arguments
///
/// * `method_name` - Name of the method/function
/// * `method_body` - Optional source code of the method body
/// * `language` - Programming language for I/O pattern detection
///
/// # Returns
///
/// Responsibility category string
///
/// # Strategy
///
/// 1. If method body is provided, use I/O detection (primary signal)
/// 2. Fall back to name-based heuristics if no I/O detected or body not available
/// 3. For conflicting signals, I/O detection takes precedence
pub fn infer_responsibility_with_io_detection(
    method_name: &str,
    method_body: Option<&str>,
    language: crate::analysis::io_detection::Language,
) -> String {
    use crate::analysis::io_detection::{IoDetector, Responsibility};

    // If we have the method body, use I/O detection as primary signal
    if let Some(body) = method_body {
        let detector = IoDetector::new();
        let profile = detector.detect_io(body, language);

        // If I/O operations detected, use I/O-based classification
        if !profile.is_pure {
            let io_responsibility = profile.primary_responsibility();
            return match io_responsibility {
                Responsibility::FileIO => "File I/O".to_string(),
                Responsibility::NetworkIO => "Network I/O".to_string(),
                Responsibility::ConsoleIO => "Console I/O".to_string(),
                Responsibility::DatabaseIO => "Database I/O".to_string(),
                Responsibility::MixedIO => "Mixed I/O".to_string(),
                Responsibility::SideEffects => "Side Effects".to_string(),
                Responsibility::PureComputation => {
                    // For pure functions, name heuristics might be more informative
                    infer_responsibility_with_confidence(method_name, None)
                        .category
                        .unwrap_or_else(|| "utilities".to_string())
                }
            };
        }
    }

    // Fall back to name-based heuristics
    infer_responsibility_with_confidence(method_name, None)
        .category
        .unwrap_or_else(|| "utilities".to_string())
}

/// Map I/O-based responsibility to traditional responsibility categories.
///
/// This helps maintain backward compatibility with existing classification while
/// leveraging the improved accuracy of I/O detection.
pub fn map_io_to_traditional_responsibility(io_resp: &str) -> String {
    match io_resp {
        "File I/O" | "Network I/O" | "Database I/O" => "persistence".to_string(),
        "Console I/O" => "output".to_string(),
        "Mixed I/O" => "processing".to_string(),
        _ => io_resp.to_string(),
    }
}

/// Infer responsibility from call patterns.
///
/// Analyzes what functions a method calls and who calls it to infer responsibility.
/// This complements name-based and I/O-based detection by looking at actual usage patterns.
///
/// # Arguments
///
/// * `function_name` - Name of the function
/// * `callees` - Functions that this function calls
/// * `callers` - Functions that call this function
///
/// # Returns
///
/// Optional responsibility string based on call patterns
///
/// # Examples
///
/// ```rust,ignore
/// // Function mostly called by formatting functions
/// let callees = vec![];
/// let callers = vec!["format_output", "render_table"];
/// let resp = infer_responsibility_from_call_patterns("escape_html", &callees, &callers);
/// assert_eq!(resp, Some("Formatting Support"));
/// ```
pub fn infer_responsibility_from_call_patterns(
    function_name: &str,
    callees: &[String],
    callers: &[String],
) -> Option<String> {
    // Special case: if function name suggests it's a helper and has many callers
    if (function_name.contains("helper") || function_name.contains("util")) && callers.len() >= 3 {
        return Some("utilities".to_string());
    }

    // Analyze caller patterns - who uses this function?
    let caller_categories = categorize_functions(callers);

    // Analyze callee patterns - what does this function use?
    let callee_categories = categorize_functions(callees);

    // If majority of callers are in one category, this is likely support for that category
    // Skip if the category is "utilities" (catch-all)
    if let Some((category, count)) = find_dominant_category(&caller_categories) {
        if count >= 2 && category != "utilities" {
            return Some(format!("{} Support", category));
        }
    }

    // If majority of callees are in one category, this is likely orchestration for that category
    if let Some((category, count)) = find_dominant_category(&callee_categories) {
        if count >= 3 {
            return Some(format!("{} Orchestration", category));
        }
    }

    None
}

/// Categorize functions by their name patterns
fn categorize_functions(functions: &[String]) -> std::collections::HashMap<String, usize> {
    let mut categories = std::collections::HashMap::new();

    for func in functions {
        let category = infer_responsibility_with_confidence(func, None)
            .category
            .unwrap_or_else(|| "utilities".to_string());
        *categories.entry(category).or_insert(0) += 1;
    }

    categories
}

/// Find the dominant category in a set of categorized functions
fn find_dominant_category(
    categories: &std::collections::HashMap<String, usize>,
) -> Option<(String, usize)> {
    categories
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(category, count)| (category.clone(), *count))
}

/// Infer responsibility using multi-signal aggregation (Spec 145).
///
/// This provides the highest accuracy classification by combining:
/// - I/O Detection (40% weight)
/// - Call Graph Analysis (30% weight)
/// - Type Signatures (15% weight)
/// - Purity Analysis (10% weight)
/// - Framework Patterns (5% weight)
/// - Name Heuristics (5% weight)
///
/// Target accuracy: ~88% (vs ~50% with name-based alone)
///
/// # Arguments
///
/// * `method_name` - Name of the method/function
/// * `method_body` - Optional source code of the method body
/// * `language` - Programming language
///
/// # Returns
///
/// Tuple of (responsibility string, confidence score, classification evidence)
pub fn infer_responsibility_multi_signal(
    method_name: &str,
    method_body: Option<&str>,
    language: crate::analysis::io_detection::Language,
) -> (
    String,
    f64,
    crate::analysis::multi_signal_aggregation::AggregatedClassification,
) {
    use crate::analysis::multi_signal_aggregation::{ResponsibilityAggregator, SignalSet};

    let aggregator = ResponsibilityAggregator::new();

    // Collect all available signals
    let mut signals = SignalSet::default();

    // I/O signal (if body available)
    if let Some(body) = method_body {
        signals.io_signal = aggregator.collect_io_signal(body, language);
        signals.purity_signal = aggregator.collect_purity_signal(body, language);
    }

    // Name signal (always available)
    signals.name_signal = Some(aggregator.collect_name_signal(method_name));

    // Aggregate all signals
    let result = aggregator.aggregate(&signals);

    // Convert to traditional responsibility string
    let responsibility = result.primary.as_str().to_string();
    let confidence = result.confidence;

    (responsibility, confidence, result)
}

/// Group methods by responsibility using multi-signal aggregation.
///
/// This provides more accurate grouping than name-based heuristics alone.
pub fn group_methods_by_responsibility_multi_signal(
    methods: &[(String, Option<String>)],
    language: crate::analysis::io_detection::Language,
) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();

    for (method_name, method_body) in methods {
        let (responsibility, _confidence, _evidence) =
            infer_responsibility_multi_signal(method_name, method_body.as_deref(), language);

        groups
            .entry(responsibility)
            .or_default()
            .push(method_name.clone());
    }

    groups
}

/// Group methods by responsibility with classification evidence.
///
/// Returns both grouped methods and their classification evidence.
pub fn group_methods_by_responsibility_with_evidence(
    methods: &[(String, Option<String>)],
    language: crate::analysis::io_detection::Language,
) -> (
    HashMap<String, Vec<String>>,
    HashMap<String, crate::analysis::multi_signal_aggregation::AggregatedClassification>,
) {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    let mut evidence_map: HashMap<
        String,
        crate::analysis::multi_signal_aggregation::AggregatedClassification,
    > = HashMap::new();

    for (method_name, method_body) in methods {
        let (responsibility, _confidence, evidence) =
            infer_responsibility_multi_signal(method_name, method_body.as_deref(), language);

        // Store evidence for this responsibility (use first occurrence)
        evidence_map
            .entry(responsibility.clone())
            .or_insert(evidence);

        groups
            .entry(responsibility)
            .or_default()
            .push(method_name.clone());
    }

    (groups, evidence_map)
}

/// Group methods by responsibility with domain pattern detection first (Spec 175).
///
/// This function implements a two-phase classification:
/// 1. First pass: Detect semantic domain patterns (observer, callback, etc.)
/// 2. Second pass: Classify remaining methods by responsibility
///
/// This prevents methods from being lumped into generic "Utilities" when they
/// actually form cohesive domain-specific clusters.
///
/// # Arguments
///
/// * `methods` - Method names with optional bodies for analysis
/// * `language` - Programming language for I/O pattern detection
/// * `structures` - Available data structures in the file for pattern detection
///
/// # Returns
///
/// Tuple of (responsibility groups, classification evidence)
pub fn group_methods_by_responsibility_with_domain_patterns(
    methods: &[(String, Option<String>)],
    language: crate::analysis::io_detection::Language,
    structures: &[String],
) -> (
    HashMap<String, Vec<String>>,
    HashMap<String, crate::analysis::multi_signal_aggregation::AggregatedClassification>,
) {
    use crate::organization::domain_patterns::{
        cluster_methods_by_domain, DomainPatternDetector, FileContext, MethodInfo,
        MIN_DOMAIN_CLUSTER_SIZE,
    };
    use std::collections::HashSet;

    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    let mut evidence_map: HashMap<
        String,
        crate::analysis::multi_signal_aggregation::AggregatedClassification,
    > = HashMap::new();

    // Phase 1: Domain pattern detection
    let detector = DomainPatternDetector::new();

    // Build file context for pattern detection
    let context_methods: Vec<MethodInfo> = methods
        .iter()
        .map(|(name, body)| MethodInfo {
            name: name.clone(),
            body: body.clone().unwrap_or_default(),
            doc_comment: None,
        })
        .collect();

    let context = FileContext {
        methods: context_methods,
        structures: structures.iter().cloned().collect::<HashSet<_>>(),
        call_edges: vec![], // Call graph analysis would go here in future
    };

    // Detect domain patterns and cluster methods
    let domain_clusters = cluster_methods_by_domain(&context.methods, &context, &detector);

    // Track which methods were assigned to domain clusters
    let mut clustered_methods: HashSet<String> = HashSet::new();

    for (pattern, cluster_methods) in domain_clusters {
        if cluster_methods.len() >= MIN_DOMAIN_CLUSTER_SIZE {
            let category = pattern.description();
            let method_names: Vec<String> =
                cluster_methods.iter().map(|m| m.name.clone()).collect();

            // Mark these methods as clustered
            for method_name in &method_names {
                clustered_methods.insert(method_name.clone());
            }

            // Create evidence for domain pattern
            let evidence = crate::analysis::multi_signal_aggregation::AggregatedClassification {
                primary: crate::analysis::multi_signal_aggregation::ResponsibilityCategory::Unknown,
                confidence: 0.70, // Domain patterns have good confidence
                evidence: vec![],
                alternatives: vec![],
            };

            evidence_map.insert(category.clone(), evidence);
            groups.insert(category, method_names);
        }
    }

    // Phase 2: Classify remaining methods by responsibility
    for (method_name, method_body) in methods {
        // Skip methods already assigned to domain clusters
        if clustered_methods.contains(method_name) {
            continue;
        }

        let (responsibility, _confidence, evidence) =
            infer_responsibility_multi_signal(method_name, method_body.as_deref(), language);

        // Store evidence for this responsibility (use first occurrence)
        evidence_map
            .entry(responsibility.clone())
            .or_insert(evidence);

        groups
            .entry(responsibility)
            .or_default()
            .push(method_name.clone());
    }

    (groups, evidence_map)
}

/// Result of responsibility classification with confidence scoring.
///
/// This struct represents the outcome of attempting to classify a method's
/// responsibility. The `category` field is `None` if confidence is below
/// the minimum threshold.
///
/// # Fields
///
/// * `category` - The classified responsibility category, or `None` if confidence too low
/// * `confidence` - Confidence score from 0.0 to 1.0
/// * `signals_used` - Which signals contributed to this classification
///
/// # Examples
///
/// ```
/// # use debtmap::organization::god_object_analysis::{ClassificationResult, SignalType};
/// let result = ClassificationResult {
///     category: Some("parsing".to_string()),
///     confidence: 0.85,
///     signals_used: vec![SignalType::NameHeuristic, SignalType::IoDetection],
/// };
/// assert!(result.category.is_some());
/// assert!(result.confidence >= 0.50);
/// ```
/// Infer responsibility category from function/method name using pattern matching.
///
/// This function uses a data-driven approach to categorize functions by matching
/// method name prefixes against predefined categories. It uses the `BehavioralCategorizer`
/// to determine the appropriate category based on method naming patterns.
///
/// # Implementation
///
/// The function:
/// 1. Converts the method name to lowercase for case-insensitive matching
/// 2. Iterates through categories until finding one with a matching prefix
/// 3. Returns the category name, or "utilities" if no match is found
///
/// # Pattern Recognition
///
/// - `format_*`, `render_*`, `write_*`, `print_*`, `display_*`, `show_*`, `draw_*`, `output_*`, `emit_*` → "output"
/// - `parse_*`, `read_*`, `extract_*`, `decode_*`, `deserialize_*`, `unmarshal_*`, `scan_*` → "parsing"
/// - `filter_*`, `select_*`, `find_*`, `search_*`, `query_*`, `lookup_*`, `match_*` → "filtering"
/// - `transform_*`, `convert_*`, `map_*`, `apply_*`, `adapt_*` → "transformation"
/// - `get_*`, `set_*`, `fetch_*`, `retrieve_*`, `access_*` → "data_access"
/// - `validate_*`, `check_*`, `verify_*`, `is_*`, `ensure_*`, `assert_*` → "validation"
/// - `calculate_*`, `compute_*`, `evaluate_*`, `measure_*` → "computation"
/// - `create_*`, `build_*`, `new_*`, `make_*`, `construct_*` → "construction"
/// - `save_*`, `load_*`, `store_*`, `persist_*`, `cache_*` → "persistence"
/// - `process_*`, `handle_*`, `execute_*`, `run_*` → "processing"
/// - `send_*`, `receive_*`, `transmit_*`, `broadcast_*`, `notify_*` → "communication"
/// - Everything else → "utilities"
///
/// # Examples
///
/// ```
/// # use debtmap::organization::god_object_analysis::infer_responsibility_from_method;
/// assert_eq!(infer_responsibility_from_method("format_output"), "Rendering");
/// assert_eq!(infer_responsibility_from_method("parse_json"), "Parsing");
/// assert_eq!(infer_responsibility_from_method("calculate_average"), "Computation");
/// assert_eq!(infer_responsibility_from_method("helper_function"), "Helper");
/// ```
///
/// # Performance
///
/// This is a pure function with O(n*m) complexity where n is the number of categories
/// (currently 12) and m is the average number of prefixes per category (~3).
/// In practice, most matches occur in the first few categories.
///
/// # Extending Patterns
///
/// To add new patterns, modify the `BehavioralCategorizer` implementation rather than this function.
/// See the documentation on `BehavioralCategorizer` for details on adding new behavioral categories.
///
/// # Alternative
///
/// For more accurate classification, consider `infer_responsibility_with_io_detection`
/// which analyzes actual I/O operations in the function body rather than just names.
#[deprecated(
    since = "0.4.0",
    note = "Use infer_responsibility_with_confidence for confidence-based classification"
)]
pub fn infer_responsibility_from_method(method_name: &str) -> String {
    // Use unified behavioral categorization (Spec 208)
    use crate::organization::BehavioralCategorizer;
    let category = BehavioralCategorizer::categorize_method(method_name);
    category.display_name()
}

/// Classify method responsibility with confidence scoring (Spec 174).
///
/// This function replaces unconditional "utilities" fallback with confidence-based
/// classification. It returns `None` for the category when confidence is too low,
/// preventing poor decomposition recommendations.
///
/// # Arguments
///
/// * `method_name` - The name of the method to classify
/// * `method_body` - Optional method body for deeper analysis
///
/// # Returns
///
/// A `ClassificationResult` with:
/// - `category`: `Some(name)` if confidence ≥ threshold, `None` otherwise
/// - `confidence`: Score from 0.0 to 1.0
/// - `signals_used`: Which signals contributed to the classification
///
/// # Confidence Thresholds
///
/// - Recognized categories: 0.85 (high confidence)
/// - Domain fallback: 0.45 (low confidence, rejected by MINIMUM_CONFIDENCE of 0.50)
///
/// # Examples
///
/// ```
/// # use debtmap::organization::god_object_analysis::infer_responsibility_with_confidence;
/// // High confidence classification
/// let result = infer_responsibility_with_confidence("parse_json", None);
/// assert!(result.category.is_some());
/// assert!(result.confidence >= 0.50);
///
/// // Low confidence - refused classification
/// let result = infer_responsibility_with_confidence("helper", None);
/// // May return None if confidence too low
/// ```
///
/// # Implementation
///
/// Currently uses name-based heuristics as the primary signal.
/// Future enhancements will integrate:
/// - I/O detection (weight: 0.40)
/// - Call graph analysis (weight: 0.30)
/// - Type signatures (weight: 0.15)
/// - Purity analysis (weight: 0.10)
pub fn infer_responsibility_with_confidence(
    method_name: &str,
    _method_body: Option<&str>,
) -> ClassificationResult {
    use crate::organization::BehavioralCategorizer;

    let category = BehavioralCategorizer::categorize_method(method_name);
    let category_name = category.display_name();

    // Assign confidence based on category type
    let confidence = match category {
        crate::organization::BehaviorCategory::Domain(_) => 0.45, // Below threshold
        _ => 0.85, // High confidence for recognized patterns
    };

    // Apply confidence thresholds
    if confidence < MINIMUM_CONFIDENCE {
        log::debug!(
            "Low confidence classification for method '{}': confidence {:.2} below minimum {:.2}",
            method_name,
            confidence,
            MINIMUM_CONFIDENCE
        );
        return ClassificationResult {
            category: None,
            confidence,
            signals_used: vec![SignalType::NameHeuristic],
        };
    }

    ClassificationResult {
        category: Some(category_name),
        confidence,
        signals_used: vec![SignalType::NameHeuristic],
    }
}

/// Map old category names to new names for backward compatibility.
///
/// This function provides a migration path for code and configuration files that
/// still use the old verbose category names (e.g., "output").
///
/// # Arguments
///
/// * `old_name` - The category name to normalize
///
/// # Returns
///
/// The normalized category name in the new format (lowercase, underscores)
///
/// # Examples
///
/// ```
/// # use debtmap::organization::god_object_analysis::normalize_category_name;
/// assert_eq!(normalize_category_name("output"), "output");
/// assert_eq!(normalize_category_name("parsing"), "parsing");
/// assert_eq!(normalize_category_name("data_access"), "data_access");
/// assert_eq!(normalize_category_name("Data Access"), "data_access"); // Normalizes to lowercase with underscores
/// ```
pub fn normalize_category_name(old_name: &str) -> String {
    match old_name {
        "output" => "output".to_string(),
        "parsing" => "parsing".to_string(),
        "filtering" => "filtering".to_string(),
        "data_access" => "data_access".to_string(),
        // Already normalized names pass through
        name => name.to_lowercase().replace(' ', "_"),
    }
}

pub fn recommend_module_splits(
    type_name: &str,
    _methods: &[String],
    responsibility_groups: &HashMap<String, Vec<String>>,
) -> Vec<ModuleSplit> {
    recommend_module_splits_with_evidence(
        type_name,
        _methods,
        responsibility_groups,
        &HashMap::new(),
    )
}

/// Enhanced version that includes field access tracking and trait extraction
pub fn recommend_module_splits_enhanced(
    type_name: &str,
    responsibility_groups: &HashMap<String, Vec<String>>,
    field_tracker: Option<&crate::organization::FieldAccessTracker>,
) -> Vec<ModuleSplit> {
    recommend_module_splits_enhanced_with_evidence(
        type_name,
        responsibility_groups,
        &HashMap::new(),
        field_tracker,
    )
}

/// Full-featured recommendation with evidence, field tracking, and trait extraction
pub fn recommend_module_splits_enhanced_with_evidence(
    type_name: &str,
    responsibility_groups: &HashMap<String, Vec<String>>,
    evidence_map: &HashMap<
        String,
        crate::analysis::multi_signal_aggregation::AggregatedClassification,
    >,
    field_tracker: Option<&crate::organization::FieldAccessTracker>,
) -> Vec<ModuleSplit> {
    let mut recommendations = Vec::new();

    for (responsibility, methods) in responsibility_groups {
        if methods.len() > 5 {
            let classification_evidence = evidence_map.get(responsibility).cloned();

            // Sanitize the responsibility name for use in module name
            let sanitized_responsibility = sanitize_module_name(responsibility);

            // Get representative methods (first 5-8)
            let representative_methods: Vec<String> = methods.iter().take(8).cloned().collect();

            // Infer behavioral category from responsibility
            let behavior_category = Some(responsibility.clone());

            // Calculate minimal field set if field tracker available
            let fields_needed = field_tracker
                .map(|tracker| tracker.get_minimal_field_set(methods))
                .unwrap_or_default();

            // Generate trait suggestion using behavioral categorization
            use crate::organization::behavioral_decomposition::{
                suggest_trait_extraction, BehavioralCategorizer, MethodCluster,
            };

            let category = BehavioralCategorizer::categorize_method(
                methods.first().map(|s| s.as_str()).unwrap_or(""),
            );

            let cluster = MethodCluster {
                category,
                methods: methods.clone(),
                fields_accessed: fields_needed.clone(),
                internal_calls: 0, // Will be populated by call graph analysis
                external_calls: 0, // Will be populated by call graph analysis
                cohesion_score: 0.0,
            };

            let trait_suggestion = Some(suggest_trait_extraction(&cluster, type_name));

            recommendations.push(ModuleSplit {
                suggested_name: format!(
                    "{}_{}",
                    type_name.to_lowercase(),
                    sanitized_responsibility
                ),
                methods_to_move: methods.clone(),
                structs_to_move: vec![],
                responsibility: responsibility.clone(),
                estimated_lines: methods.len() * 20,
                method_count: methods.len(),
                warning: None,
                priority: Priority::Medium,
                cohesion_score: None,
                dependencies_in: vec![],
                dependencies_out: vec![],
                domain: String::new(),
                rationale: Some(format!(
                    "Methods grouped by '{}' responsibility pattern",
                    responsibility
                )),
                method: SplitAnalysisMethod::MethodBased,
                severity: None,
                interface_estimate: None,
                classification_evidence,
                representative_methods,
                fields_needed,
                trait_suggestion,
                behavior_category,
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
            });
        }
    }

    recommendations
}

pub fn recommend_module_splits_with_evidence(
    type_name: &str,
    _methods: &[String],
    responsibility_groups: &HashMap<String, Vec<String>>,
    evidence_map: &HashMap<
        String,
        crate::analysis::multi_signal_aggregation::AggregatedClassification,
    >,
) -> Vec<ModuleSplit> {
    use crate::organization::confidence::{MIN_METHODS_FOR_SPLIT, MODULE_SPLIT_CONFIDENCE};

    let mut recommendations = Vec::new();

    for (responsibility, methods) in responsibility_groups {
        if methods.len() > MIN_METHODS_FOR_SPLIT {
            let classification_evidence = evidence_map.get(responsibility).cloned();

            // Calculate average confidence from evidence map
            // If evidence_map is provided (not empty), enforce confidence threshold
            // If evidence_map is empty, allow split for backward compatibility
            if !evidence_map.is_empty() {
                let avg_confidence = if let Some(evidence) = &classification_evidence {
                    evidence.confidence
                } else {
                    0.0
                };

                // Skip splits below confidence threshold
                if avg_confidence < MODULE_SPLIT_CONFIDENCE {
                    log::debug!(
                        "Skipping module split for '{}': confidence {:.2} below threshold {:.2}",
                        responsibility,
                        avg_confidence,
                        MODULE_SPLIT_CONFIDENCE
                    );
                    continue;
                }
            }

            // Sanitize the responsibility name for use in module name
            let sanitized_responsibility = sanitize_module_name(responsibility);

            // Get representative methods (first 5-8)
            let representative_methods: Vec<String> = methods.iter().take(8).cloned().collect();

            // Infer behavioral category from responsibility
            let behavior_category = Some(responsibility.clone());

            // Generate trait suggestion using behavioral categorization
            use crate::organization::behavioral_decomposition::{
                suggest_trait_extraction, BehavioralCategorizer, MethodCluster,
            };

            let category = BehavioralCategorizer::categorize_method(
                methods.first().map(|s| s.as_str()).unwrap_or(""),
            );

            let cluster = MethodCluster {
                category,
                methods: methods.clone(),
                fields_accessed: vec![], // Will be populated when field tracker is available
                internal_calls: 0,       // Will be populated by call graph analysis
                external_calls: 0,       // Will be populated by call graph analysis
                cohesion_score: 0.0,
            };

            let trait_suggestion = Some(suggest_trait_extraction(&cluster, type_name));

            recommendations.push(ModuleSplit {
                suggested_name: format!(
                    "{}_{}",
                    type_name.to_lowercase(),
                    sanitized_responsibility
                ),
                methods_to_move: methods.clone(),
                structs_to_move: vec![],
                responsibility: responsibility.clone(),
                estimated_lines: methods.len() * 20, // Rough estimate
                method_count: methods.len(),
                warning: None,
                priority: Priority::Medium,
                cohesion_score: None,
                dependencies_in: vec![],
                dependencies_out: vec![],
                domain: String::new(),
                rationale: Some(format!(
                    "Methods grouped by '{}' responsibility pattern",
                    responsibility
                )),
                method: SplitAnalysisMethod::MethodBased,
                severity: None,
                interface_estimate: None,
                classification_evidence,
                representative_methods,
                fields_needed: vec![], // Will be populated by field access analysis when available
                trait_suggestion,
                behavior_category,
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
            });
        }
    }

    recommendations
}

/// Count distinct semantic domains in struct list
pub fn count_distinct_domains(structs: &[StructMetrics]) -> usize {
    use std::collections::HashSet;
    let domains: HashSet<String> = structs
        .iter()
        .map(|s| classify_struct_domain(&s.name))
        .collect();
    domains.len()
}

/// Calculate struct-to-function ratio
pub fn calculate_struct_ratio(struct_count: usize, total_functions: usize) -> f64 {
    if total_functions == 0 {
        return 0.0;
    }
    (struct_count as f64) / (total_functions as f64)
}

/// Determine severity of cross-domain mixing issue
pub fn determine_cross_domain_severity(
    struct_count: usize,
    domain_count: usize,
    lines: usize,
    is_god_object: bool,
) -> RecommendationSeverity {
    // CRITICAL: God object with cross-domain mixing
    if is_god_object && domain_count >= 3 {
        return RecommendationSeverity::Critical;
    }

    // CRITICAL: Massive cross-domain mixing
    if struct_count > 15 && domain_count >= 5 {
        return RecommendationSeverity::Critical;
    }

    // HIGH: Significant cross-domain issues
    if struct_count >= 10 && domain_count >= 4 {
        return RecommendationSeverity::High;
    }

    if lines > 800 && domain_count >= 3 {
        return RecommendationSeverity::High;
    }

    // MEDIUM: Proactive improvement opportunity
    if struct_count >= 8 || lines > 400 {
        return RecommendationSeverity::Medium;
    }

    // LOW: Informational only
    RecommendationSeverity::Low
}

/// Suggest module splits based on struct name patterns (domain-based grouping)
pub fn suggest_module_splits_by_domain(structs: &[StructMetrics]) -> Vec<ModuleSplit> {
    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
    let mut line_estimates: HashMap<String, usize> = HashMap::new();
    let mut method_counts: HashMap<String, usize> = HashMap::new();

    for struct_metrics in structs {
        let domain = classify_struct_domain(&struct_metrics.name);
        grouped
            .entry(domain.clone())
            .or_default()
            .push(struct_metrics.name.clone());
        *line_estimates.entry(domain.clone()).or_insert(0) +=
            struct_metrics.line_span.1 - struct_metrics.line_span.0;
        *method_counts.entry(domain).or_insert(0) += struct_metrics.method_count;
    }

    grouped
        .into_iter()
        .filter(|(_, structs)| structs.len() > 1)
        .map(|(domain, structs)| {
            let estimated_lines = line_estimates.get(&domain).copied().unwrap_or(0);
            let method_count = method_counts.get(&domain).copied().unwrap_or(0);
            let suggested_name = format!("config/{}", domain);
            ModuleSplit::validate_name(&suggested_name);
            ModuleSplit {
                suggested_name,
                methods_to_move: vec![],
                structs_to_move: structs,
                responsibility: domain.clone(),
                estimated_lines,
                method_count,
                warning: None,
                priority: Priority::Medium,
                cohesion_score: None,
                dependencies_in: vec![],
                dependencies_out: vec![],
                domain: domain.clone(),
                rationale: Some(format!(
                    "Structs grouped by '{}' domain to improve organization",
                    domain
                )),
                method: SplitAnalysisMethod::CrossDomain,
                severity: None, // Will be set by caller based on overall analysis
                interface_estimate: None,
                classification_evidence: None,
                representative_methods: vec![],
                fields_needed: vec![],
                trait_suggestion: None,
                behavior_category: None,
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
            }
        })
        .collect()
}

/// Classify struct into a domain based on naming patterns
pub fn classify_struct_domain(struct_name: &str) -> String {
    let lower = struct_name.to_lowercase();

    if lower.contains("weight")
        || lower.contains("multiplier")
        || lower.contains("factor")
        || lower.contains("scoring")
    {
        "scoring".to_string()
    } else if lower.contains("threshold") || lower.contains("limit") || lower.contains("bound") {
        "thresholds".to_string()
    } else if lower.contains("detection") || lower.contains("detector") || lower.contains("checker")
    {
        "detection".to_string()
    } else if lower.contains("config") || lower.contains("settings") || lower.contains("options") {
        "core_config".to_string()
    } else if lower.contains("data") || lower.contains("info") || lower.contains("metrics") {
        "data".to_string()
    } else {
        // Extract first meaningful word from struct name as domain
        extract_domain_from_name(struct_name)
    }
}

/// Extract domain name from struct/type name by taking first meaningful word
fn extract_domain_from_name(name: &str) -> String {
    // Handle camelCase and PascalCase
    let mut domain = String::new();
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            break;
        }
        domain.push(c);
    }

    if !domain.is_empty() {
        domain
    } else {
        // Fallback to snake_case extraction
        name.split('_')
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Core".to_string())
    }
}

/// Calculate domain diversity metrics from struct metrics (Spec 152).
///
/// Creates struct domain classifications and computes diversity metrics.
pub fn calculate_domain_diversity_from_structs(
    structs: &[StructMetrics],
    is_god_object: bool,
) -> Result<crate::organization::DomainDiversityMetrics, anyhow::Error> {
    use crate::organization::{DomainDiversityMetrics, StructDomainClassification};

    // Create classifications for each struct
    let classifications: Vec<StructDomainClassification> = structs
        .iter()
        .map(|s| {
            let domain = classify_struct_domain(&s.name);
            StructDomainClassification::simple(s.name.clone(), domain)
        })
        .collect();

    // Calculate metrics
    DomainDiversityMetrics::from_struct_classifications(&classifications, is_god_object)
}

/// Sanitize module name to be valid across all languages.
///
/// Transforms human-readable responsibility names into valid module identifiers
/// by replacing invalid characters and normalizing whitespace.
///
/// # Character Transformations
///
/// - `&` → `and`
/// - `'` → removed
/// - `-` → `_`
/// - `/` → `_` (except when part of directory path)
/// - Multiple spaces → single `_`
/// - Multiple underscores → single `_`
/// - Leading/trailing underscores removed
/// - Convert to lowercase
/// - Preserve only alphanumeric characters and underscores
///
/// # Examples
///
/// ```
/// # use debtmap::organization::god_object_analysis::sanitize_module_name;
/// assert_eq!(sanitize_module_name("parsing"), "parsing");
/// assert_eq!(sanitize_module_name("Data  Access"), "data_access");
/// assert_eq!(sanitize_module_name("I/O Utilities"), "i_o_utilities");
/// assert_eq!(sanitize_module_name("User's Profile"), "users_profile");
/// assert_eq!(sanitize_module_name("Data-Access-Layer"), "data_access_layer");
/// ```
pub fn sanitize_module_name(name: &str) -> String {
    let sanitized = name
        .to_lowercase()
        .replace('&', "and")
        .replace(['/', '-'], "_")
        .replace('\'', "")
        .replace(' ', "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_");

    ensure_not_reserved(sanitized)
}

/// Ensure uniqueness by appending numeric suffix if needed.
///
/// If the name already exists in the set of existing names, appends a numeric
/// suffix starting from 1 until a unique name is found.
///
/// # Arguments
///
/// * `name` - The proposed module name
/// * `existing_names` - Set of already-used names
///
/// # Returns
///
/// A unique name, either the original or with a numeric suffix
///
/// # Examples
///
/// ```
/// # use std::collections::HashSet;
/// # use debtmap::organization::god_object_analysis::ensure_unique_name;
/// let mut existing = HashSet::new();
/// existing.insert("utilities".to_string());
///
/// assert_eq!(ensure_unique_name("utilities".to_string(), &existing), "utilities_1");
///
/// existing.insert("utilities_1".to_string());
/// assert_eq!(ensure_unique_name("utilities".to_string(), &existing), "utilities_2");
/// ```
pub fn ensure_unique_name(
    name: String,
    existing_names: &std::collections::HashSet<String>,
) -> String {
    if !existing_names.contains(&name) {
        return name;
    }

    let mut counter = 1;
    loop {
        let candidate = format!("{}_{}", name, counter);
        if !existing_names.contains(&candidate) {
            return candidate;
        }
        counter += 1;
    }
}

/// Suggest module splits using enhanced struct ownership analysis.
///
/// This function uses the struct ownership analyzer to create more accurate
/// recommendations based on which methods belong to which structs.
///
/// # Arguments
///
/// * `structs` - Per-struct metrics
/// * `ownership` - Struct ownership analyzer (optional for backward compatibility)
/// * `file_path` - Path to the source file (optional, for call graph analysis)
/// * `ast` - Parsed AST (optional, for call graph analysis)
///
/// # Returns
///
/// Vector of validated module splits with cohesion scores and dependencies when available
pub fn suggest_splits_by_struct_grouping(
    structs: &[StructMetrics],
    ownership: Option<&crate::organization::struct_ownership::StructOwnershipAnalyzer>,
    file_path: Option<&std::path::Path>,
    ast: Option<&syn::File>,
) -> Vec<ModuleSplit> {
    use crate::organization::domain_classifier::classify_struct_domain_enhanced;
    use crate::organization::split_validator::validate_and_refine_splits;

    // If no ownership info, fall back to basic domain-based grouping
    let ownership = match ownership {
        Some(o) => o,
        None => return suggest_module_splits_by_domain(structs),
    };

    // Group structs by domain using enhanced classification
    let mut domain_groups: HashMap<String, Vec<StructWithMethods>> = HashMap::new();

    for struct_metrics in structs {
        let methods = ownership.get_struct_methods(&struct_metrics.name);
        let domain = classify_struct_domain_enhanced(&struct_metrics.name, methods);

        domain_groups
            .entry(domain)
            .or_default()
            .push(StructWithMethods {
                name: struct_metrics.name.clone(),
                methods: methods.to_vec(),
                line_span: struct_metrics.line_span,
            });
    }

    // Convert domain groups to module splits
    let splits: Vec<ModuleSplit> = domain_groups
        .into_iter()
        .map(|(domain, structs_with_methods)| {
            let struct_names: Vec<String> = structs_with_methods
                .iter()
                .map(|s| s.name.clone())
                .collect();

            let total_methods: usize = structs_with_methods.iter().map(|s| s.methods.len()).sum();

            let estimated_lines: usize = structs_with_methods
                .iter()
                .map(|s| s.line_span.1.saturating_sub(s.line_span.0))
                .sum();

            ModuleSplit {
                suggested_name: format!("{}_{}", "module", domain),
                methods_to_move: vec![],
                structs_to_move: struct_names,
                responsibility: domain.clone(),
                estimated_lines: estimated_lines.max(total_methods * 15), // Estimate if line_span not available
                method_count: total_methods,
                warning: None,
                priority: Priority::Medium,
                cohesion_score: None,
                dependencies_in: vec![],
                dependencies_out: vec![],
                domain: domain.clone(),
                rationale: Some(format!(
                    "Structs grouped by '{}' domain using struct ownership analysis",
                    domain
                )),
                method: SplitAnalysisMethod::CrossDomain,
                severity: None,
                interface_estimate: None,
                classification_evidence: None,
                representative_methods: vec![],
                fields_needed: vec![],
                trait_suggestion: None,
                behavior_category: None,
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
            }
        })
        .collect();

    // Validate and refine splits (filters too small, splits too large)
    let validated_splits = validate_and_refine_splits(splits);

    // Enhance with cohesion scores and dependencies if ast and file_path are available
    if let (Some(path), Some(ast_file)) = (file_path, ast) {
        use crate::organization::call_graph_cohesion::enhance_splits_with_cohesion;
        enhance_splits_with_cohesion(validated_splits, path, ast_file, ownership)
    } else {
        validated_splits
    }
}

#[cfg(test)]
mod tests {
    use super::super::god_object::thresholds::is_reserved_keyword;
    use super::*;

    // Helper function matching old behavior for backward compatibility with tests
    // This mimics the deprecated infer_responsibility_from_method function
    fn infer_category(method_name: &str) -> String {
        let result = infer_responsibility_with_confidence(method_name, None);
        result.category.unwrap_or_else(|| {
            // Fall back to behavioral categorization (like the old function did)
            use crate::organization::BehavioralCategorizer;
            let category = BehavioralCategorizer::categorize_method(method_name);
            category.display_name()
        })
    }

    #[test]
    fn test_format_prefix_recognized() {
        assert_eq!(infer_category("format_output"), "Rendering");
        assert_eq!(infer_category("format_json"), "Rendering");
        assert_eq!(infer_category("FORMAT_DATA"), "Rendering");
    }

    #[test]
    fn test_render_prefix_recognized() {
        assert_eq!(infer_category("render_table"), "Rendering");
    }

    #[test]
    fn test_write_prefix_recognized() {
        // Per spec 208: "write_*" is categorized as Persistence (file I/O), not Rendering
        assert_eq!(infer_category("write_to_file"), "Persistence");
    }

    #[test]
    fn test_print_prefix_recognized() {
        assert_eq!(infer_category("print_results"), "Rendering");
    }

    #[test]
    fn test_parse_prefix_recognized() {
        assert_eq!(infer_category("parse_input"), "Parsing");
        assert_eq!(infer_category("parse_json"), "Parsing");
    }

    #[test]
    fn test_read_prefix_recognized() {
        assert_eq!(infer_category("read_config"), "Parsing");
    }

    #[test]
    fn test_extract_prefix_recognized() {
        assert_eq!(infer_category("extract_data"), "Parsing");
    }

    #[test]
    fn test_filter_prefix_recognized() {
        assert_eq!(infer_category("filter_results"), "Filtering");
    }

    #[test]
    fn test_select_prefix_recognized() {
        assert_eq!(infer_category("select_items"), "Filtering");
    }

    #[test]
    fn test_find_prefix_recognized() {
        assert_eq!(infer_category("find_element"), "Filtering");
    }

    #[test]
    fn test_transform_prefix_recognized() {
        assert_eq!(infer_category("transform_data"), "Transformation");
    }

    #[test]
    fn test_convert_prefix_recognized() {
        assert_eq!(infer_category("convert_to_json"), "Transformation");
    }

    #[test]
    fn test_map_prefix_recognized() {
        assert_eq!(infer_category("map_values"), "Transformation");
    }

    #[test]
    fn test_apply_prefix_recognized() {
        assert_eq!(infer_category("apply_mapping"), "Transformation");
    }

    #[test]
    fn test_get_prefix_recognized() {
        assert_eq!(infer_category("get_value"), "Data Access");
    }

    #[test]
    fn test_set_prefix_recognized() {
        assert_eq!(infer_category("set_value"), "Data Access");
    }

    #[test]
    fn test_is_prefix_recognized() {
        assert_eq!(infer_category("is_valid"), "Validation");
        assert_eq!(infer_category("is_empty"), "Validation");
    }

    #[test]
    fn test_validate_prefix_recognized() {
        assert_eq!(infer_category("validate_input"), "Validation");
    }

    #[test]
    fn test_check_prefix_recognized() {
        assert_eq!(infer_category("check_constraints"), "Validation");
    }

    #[test]
    fn test_verify_prefix_recognized() {
        assert_eq!(infer_category("verify_signature"), "Validation");
    }

    #[test]
    fn test_catch_all_uses_behavioral_categorization() {
        // Spec 178: Avoid "utilities", use behavioral categorization
        // "unknown_function" -> Domain("Unknown") based on first word
        assert_eq!(infer_category("unknown_function"), "Unknown");
        // "some_helper" -> Domain("Some") based on first word
        assert_eq!(infer_category("some_helper"), "Some");
    }

    #[test]
    fn test_responsibility_grouping_not_empty() {
        let methods = vec!["format_a".to_string(), "format_b".to_string()];
        let groups = group_methods_by_responsibility(&methods);
        assert!(!groups.is_empty());
        assert_eq!(groups.len(), 1);
        // Per spec 208: format_* methods are now categorized as "Rendering" (Title Case)
        assert_eq!(groups.get("Rendering").unwrap().len(), 2);
    }

    #[test]
    fn test_multiple_responsibility_groups() {
        let methods = vec![
            "format_output".to_string(),
            "format_json".to_string(),
            "parse_input".to_string(),
            "get_value".to_string(),
            "is_valid".to_string(),
        ];
        let groups = group_methods_by_responsibility(&methods);
        // Per spec 208: Category names now use Title Case (e.g., "Rendering" not "output")
        assert_eq!(groups.len(), 4); // Rendering, Parsing, Data Access, Validation
        assert!(groups.contains_key("Rendering")); // format_* methods
        assert!(groups.contains_key("Parsing")); // parse_* methods
        assert!(groups.contains_key("Data Access")); // get_* methods
        assert!(groups.contains_key("Validation")); // is_* methods
    }

    #[test]
    fn test_case_insensitive_matching() {
        assert_eq!(infer_category("FORMAT_OUTPUT"), "Rendering");
        assert_eq!(infer_category("Parse_Input"), "Parsing");
        assert_eq!(infer_category("IS_VALID"), "Validation");
    }

    #[test]
    fn test_calculate_prefix_recognized() {
        assert_eq!(infer_category("calculate_total"), "Computation");
        assert_eq!(infer_category("calculate_sum"), "Computation");
    }

    #[test]
    fn test_compute_prefix_recognized() {
        assert_eq!(infer_category("compute_result"), "Computation");
    }

    #[test]
    fn test_create_prefix_recognized() {
        assert_eq!(infer_category("create_instance"), "Construction");
    }

    #[test]
    fn test_build_prefix_recognized() {
        assert_eq!(infer_category("build_object"), "Construction");
    }

    #[test]
    fn test_new_prefix_recognized() {
        assert_eq!(infer_category("new_connection"), "Construction");
    }

    #[test]
    fn test_save_prefix_recognized() {
        assert_eq!(infer_category("save_to_disk"), "Persistence");
    }

    #[test]
    fn test_load_prefix_recognized() {
        assert_eq!(infer_category("load_from_file"), "Persistence");
    }

    #[test]
    fn test_store_prefix_recognized() {
        assert_eq!(infer_category("store_data"), "Persistence");
    }

    #[test]
    fn test_process_prefix_recognized() {
        assert_eq!(infer_category("process_request"), "Processing");
    }

    #[test]
    fn test_handle_prefix_recognized() {
        assert_eq!(infer_category("handle_event"), "Event Handling");
    }

    #[test]
    fn test_send_prefix_recognized() {
        assert_eq!(infer_category("send_message"), "Communication");
    }

    #[test]
    fn test_receive_prefix_recognized() {
        assert_eq!(infer_category("receive_data"), "Communication");
    }

    #[test]
    fn test_empty_string_returns_operations() {
        // Spec 178: Empty string defaults to "Operations" via Domain fallback
        assert_eq!(infer_category(""), "Operations");
    }

    #[test]
    fn test_underscore_only_returns_operations() {
        // Spec 178: Underscores default to "Operations" via Domain fallback
        assert_eq!(infer_category("_"), "Operations");
        assert_eq!(infer_category("__"), "Operations");
    }

    #[test]
    fn test_special_chars_return_first_word_domain() {
        // Spec 178: Special chars extracted as domain name
        assert_eq!(infer_category("@#$%"), "@#$%");
    }

    #[test]
    fn test_function_is_deterministic() {
        let input = "calculate_average";
        let result1 = infer_category(input);
        let result2 = infer_category(input);
        assert_eq!(result1, result2);
    }

    // Spec 134: Tests for metric validation
    #[test]
    fn test_validation_passes_consistent_metrics() {
        let analysis = GodObjectAnalysis {
            is_god_object: true,
            method_count: 3,
            field_count: 5,
            responsibility_count: 2,
            lines_of_code: 100,
            complexity_sum: 20,
            god_object_score: 75.0,
            recommended_splits: vec![],
            confidence: GodObjectConfidence::Probable,
            responsibilities: vec!["data_access".to_string(), "validation".to_string()],
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodClass,
            visibility_breakdown: Some(FunctionVisibilityBreakdown {
                public: 1,
                pub_crate: 1,
                pub_super: 0,
                private: 1,
            }),
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
        };

        assert!(analysis.validate().is_ok());
    }

    #[test]
    fn test_validation_detects_visibility_mismatch() {
        let analysis = GodObjectAnalysis {
            is_god_object: true,
            method_count: 10,
            field_count: 5,
            responsibility_count: 2,
            lines_of_code: 100,
            complexity_sum: 20,
            god_object_score: 75.0,
            recommended_splits: vec![],
            confidence: GodObjectConfidence::Probable,
            responsibilities: vec!["data_access".to_string(), "validation".to_string()],
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodClass,
            visibility_breakdown: Some(FunctionVisibilityBreakdown {
                public: 2,
                pub_crate: 1,
                pub_super: 0,
                private: 1,
            }),
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
        };

        let result = analysis.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MetricInconsistency::VisibilityMismatch { .. }
        ));
    }

    #[test]
    fn test_validation_detects_responsibility_count_mismatch() {
        let analysis = GodObjectAnalysis {
            is_god_object: true,
            method_count: 3,
            field_count: 5,
            responsibility_count: 5, // Says 5 but only provides 2 names
            lines_of_code: 100,
            complexity_sum: 20,
            god_object_score: 75.0,
            recommended_splits: vec![],
            confidence: GodObjectConfidence::Probable,
            responsibilities: vec!["data_access".to_string(), "validation".to_string()],
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodClass,
            visibility_breakdown: Some(FunctionVisibilityBreakdown {
                public: 1,
                pub_crate: 1,
                pub_super: 0,
                private: 1,
            }),
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
        };

        let result = analysis.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MetricInconsistency::ResponsibilityCountMismatch { .. }
        ));
    }

    #[test]
    fn test_validation_detects_missing_responsibilities() {
        let analysis = GodObjectAnalysis {
            is_god_object: true,
            method_count: 10, // Has methods
            field_count: 5,
            responsibility_count: 0,
            lines_of_code: 100,
            complexity_sum: 20,
            god_object_score: 75.0,
            recommended_splits: vec![],
            confidence: GodObjectConfidence::Probable,
            responsibilities: vec![], // But no responsibilities
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodClass,
            visibility_breakdown: Some(FunctionVisibilityBreakdown {
                public: 5,
                pub_crate: 3,
                pub_super: 0,
                private: 2,
            }),
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
        };

        let result = analysis.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MetricInconsistency::MissingResponsibilities { .. }
        ));
    }

    #[test]
    fn test_function_visibility_breakdown_total() {
        let breakdown = FunctionVisibilityBreakdown {
            public: 5,
            pub_crate: 3,
            pub_super: 2,
            private: 10,
        };
        assert_eq!(breakdown.total(), 20);
    }

    #[test]
    fn test_count_distinct_domains() {
        let structs = vec![
            StructMetrics {
                name: "ThresholdConfig".to_string(),
                line_span: (0, 10),
                method_count: 2,
                field_count: 5,
                responsibilities: vec!["configuration".to_string()],
            },
            StructMetrics {
                name: "ThresholdValidator".to_string(),
                line_span: (11, 20),
                method_count: 3,
                field_count: 4,
                responsibilities: vec!["validation".to_string()],
            },
            StructMetrics {
                name: "ScoringWeight".to_string(),
                line_span: (21, 30),
                method_count: 4,
                field_count: 3,
                responsibilities: vec!["calculation".to_string()],
            },
            StructMetrics {
                name: "ScoringMultiplier".to_string(),
                line_span: (31, 40),
                method_count: 2,
                field_count: 6,
                responsibilities: vec!["configuration".to_string()],
            },
        ];

        // Should identify 2 domains: "thresholds" and "scoring"
        assert_eq!(count_distinct_domains(&structs), 2);
    }

    #[test]
    fn test_count_distinct_domains_single() {
        let structs = vec![
            StructMetrics {
                name: "ConfigA".to_string(),
                line_span: (0, 10),
                method_count: 2,
                field_count: 5,
                responsibilities: vec!["configuration".to_string()],
            },
            StructMetrics {
                name: "ConfigB".to_string(),
                line_span: (11, 20),
                method_count: 3,
                field_count: 4,
                responsibilities: vec!["configuration".to_string()],
            },
        ];

        // Should identify 1 domain: "config"
        assert_eq!(count_distinct_domains(&structs), 1);
    }

    #[test]
    fn test_calculate_struct_ratio() {
        // Normal case
        assert_eq!(calculate_struct_ratio(10, 20), 0.5);

        // More structs than functions
        assert_eq!(calculate_struct_ratio(15, 10), 1.5);

        // Single struct
        assert_eq!(calculate_struct_ratio(1, 10), 0.1);
    }

    #[test]
    fn test_calculate_struct_ratio_edge_cases() {
        // Zero functions should return 0.0 to avoid division by zero
        assert_eq!(calculate_struct_ratio(10, 0), 0.0);

        // Zero structs
        assert_eq!(calculate_struct_ratio(0, 10), 0.0);

        // Both zero
        assert_eq!(calculate_struct_ratio(0, 0), 0.0);
    }

    #[test]
    fn test_determine_cross_domain_severity_critical() {
        // Critical: God object with cross-domain mixing
        assert!(matches!(
            determine_cross_domain_severity(10, 3, 600, true),
            RecommendationSeverity::Critical
        ));

        // Critical: Massive cross-domain mixing
        assert!(matches!(
            determine_cross_domain_severity(16, 5, 400, false),
            RecommendationSeverity::Critical
        ));
    }

    #[test]
    fn test_determine_cross_domain_severity_high() {
        // High: Significant cross-domain issues
        assert!(matches!(
            determine_cross_domain_severity(10, 4, 500, false),
            RecommendationSeverity::High
        ));

        // High: Large file with multiple domains
        assert!(matches!(
            determine_cross_domain_severity(8, 3, 850, false),
            RecommendationSeverity::High
        ));
    }

    #[test]
    fn test_determine_cross_domain_severity_medium() {
        // Medium: Proactive improvement opportunity (8+ structs)
        assert!(matches!(
            determine_cross_domain_severity(8, 2, 300, false),
            RecommendationSeverity::Medium
        ));

        // Medium: Larger file
        assert!(matches!(
            determine_cross_domain_severity(6, 2, 450, false),
            RecommendationSeverity::Medium
        ));
    }

    #[test]
    fn test_determine_cross_domain_severity_low() {
        // Low: Small file with few structs
        assert!(matches!(
            determine_cross_domain_severity(5, 2, 200, false),
            RecommendationSeverity::Low
        ));

        // Low: Minimal mixing
        assert!(matches!(
            determine_cross_domain_severity(3, 2, 150, false),
            RecommendationSeverity::Low
        ));
    }

    #[test]
    fn test_struct_heavy_detection() {
        // Struct-heavy: 8 structs, 10 functions, ratio = 0.8
        let ratio = calculate_struct_ratio(8, 10);
        assert!(ratio > 0.3);

        // Not struct-heavy: 3 structs, 20 functions, ratio = 0.15
        let ratio = calculate_struct_ratio(3, 20);
        assert!(ratio < 0.3);

        // Edge case: Exactly at threshold
        let ratio = calculate_struct_ratio(5, 15);
        assert_eq!(ratio, 5.0 / 15.0);
    }

    // Tests for I/O-based responsibility detection (Spec 141)
    #[test]
    fn test_io_detection_file_io() {
        use crate::analysis::io_detection::Language;

        let method_name = "load_config";
        let method_body = r#"
            fn load_config() -> String {
                std::fs::read_to_string("config.toml").unwrap()
            }
        "#;

        let responsibility =
            infer_responsibility_with_io_detection(method_name, Some(method_body), Language::Rust);

        assert_eq!(responsibility, "File I/O");
    }

    #[test]
    fn test_io_detection_network_io() {
        use crate::analysis::io_detection::Language;

        let method_name = "fetch_data";
        let method_body = r#"
            fn fetch_data() {
                let client = reqwest::blocking::Client::new();
                let response = client.get("https://api.example.com").send();
            }
        "#;

        let responsibility =
            infer_responsibility_with_io_detection(method_name, Some(method_body), Language::Rust);

        assert_eq!(responsibility, "Network I/O");
    }

    #[test]
    fn test_io_detection_console_io() {
        use crate::analysis::io_detection::Language;

        let method_name = "display_results";
        let method_body = r#"
            fn display_results(data: &str) {
                println!("Results: {}", data);
            }
        "#;

        let responsibility =
            infer_responsibility_with_io_detection(method_name, Some(method_body), Language::Rust);

        assert_eq!(responsibility, "Console I/O");
    }

    #[test]
    fn test_io_detection_pure_computation() {
        use crate::analysis::io_detection::Language;

        let method_name = "calculate_sum";
        let method_body = r#"
            fn calculate_sum(a: i32, b: i32) -> i32 {
                a + b
            }
        "#;

        let responsibility =
            infer_responsibility_with_io_detection(method_name, Some(method_body), Language::Rust);

        // Pure functions fall back to name-based heuristics
        assert_eq!(responsibility, "Computation");
    }

    #[test]
    fn test_io_detection_fallback_to_name() {
        use crate::analysis::io_detection::Language;

        let method_name = "format_output";
        let method_body = None; // No body provided

        let responsibility =
            infer_responsibility_with_io_detection(method_name, method_body, Language::Rust);

        // Without body, falls back to name-based detection
        assert_eq!(responsibility, "Rendering");
    }

    #[test]
    fn test_io_detection_python_file_io() {
        use crate::analysis::io_detection::Language;

        let method_name = "read_data";
        let method_body = r#"
            def read_data():
                with open('data.json') as f:
                    return f.read()
        "#;

        let responsibility = infer_responsibility_with_io_detection(
            method_name,
            Some(method_body),
            Language::Python,
        );

        assert_eq!(responsibility, "File I/O");
    }

    #[test]
    fn test_map_io_to_traditional_responsibility() {
        assert_eq!(
            map_io_to_traditional_responsibility("File I/O"),
            "persistence"
        );
        assert_eq!(
            map_io_to_traditional_responsibility("Network I/O"),
            "persistence"
        );
        assert_eq!(
            map_io_to_traditional_responsibility("Database I/O"),
            "persistence"
        );
        assert_eq!(
            map_io_to_traditional_responsibility("Console I/O"),
            "output"
        );
        assert_eq!(
            map_io_to_traditional_responsibility("Mixed I/O"),
            "processing"
        );
    }

    // Tests for call pattern-based responsibility detection (Spec 137)
    #[test]
    fn test_call_pattern_support_detection() {
        let function_name = "escape_html";
        let callees = vec![];
        let callers = vec!["format_output".to_string(), "render_table".to_string()];

        let resp = infer_responsibility_from_call_patterns(function_name, &callees, &callers);
        // Per spec 208: "format_*" and "render_*" are now categorized as "Rendering"
        assert_eq!(resp, Some("Rendering Support".to_string()));
    }

    #[test]
    fn test_validation_method_categorization() {
        // Debug test: verify that validation methods are categorized correctly
        let methods = vec!["validate_input", "check_bounds", "verify_format"];

        for method in &methods {
            let result = infer_responsibility_with_confidence(method, None);
            println!(
                "Method '{}': category={:?}, confidence={}",
                method, result.category, result.confidence
            );
            assert_eq!(
                result.category,
                Some("Validation".to_string()),
                "Method '{}' should be categorized as Validation",
                method
            );
        }
    }

    #[test]
    fn test_call_pattern_orchestration_detection() {
        let function_name = "process_data";
        let callees = vec![
            "validate_input".to_string(),
            "check_bounds".to_string(),
            "verify_format".to_string(),
        ];
        let callers = vec![];

        let resp = infer_responsibility_from_call_patterns(function_name, &callees, &callers);
        // Per spec 208: Category names now use proper capitalization ("Validation" not "validation")
        assert_eq!(resp, Some("Validation Orchestration".to_string()));
    }

    #[test]
    fn test_call_pattern_utility_detection() {
        let function_name = "helper_function";
        let callees = vec![];
        let callers = vec![
            "func1".to_string(),
            "func2".to_string(),
            "func3".to_string(),
            "func4".to_string(),
        ];

        let resp = infer_responsibility_from_call_patterns(function_name, &callees, &callers);
        assert_eq!(resp, Some("utilities".to_string()));
    }

    #[test]
    fn test_call_pattern_no_clear_pattern() {
        let function_name = "mixed_function";
        let callees = vec!["func1".to_string()];
        let callers = vec!["func2".to_string()];

        let resp = infer_responsibility_from_call_patterns(function_name, &callees, &callers);
        assert_eq!(resp, None);
    }

    #[test]
    fn test_categorize_functions() {
        let functions = vec![
            "format_output".to_string(),
            "format_json".to_string(),
            "parse_input".to_string(),
            "validate_data".to_string(),
        ];

        let categories = categorize_functions(&functions);
        // Per spec 208: "format_*" methods are now categorized as "Rendering" (not "output")
        assert_eq!(categories.get("Rendering"), Some(&2));
        assert_eq!(categories.get("Parsing"), Some(&1));
        assert_eq!(categories.get("Validation"), Some(&1));
    }

    #[test]
    fn test_find_dominant_category() {
        let mut categories = std::collections::HashMap::new();
        categories.insert("output".to_string(), 5);
        categories.insert("parsing".to_string(), 2);
        categories.insert("validation".to_string(), 1);

        let dominant = find_dominant_category(&categories);
        assert_eq!(dominant, Some(("output".to_string(), 5)));
    }

    #[test]
    fn test_split_names_have_no_extensions() {
        // Valid names without extensions should pass
        ModuleSplit::validate_name("config/misc");
        ModuleSplit::validate_name("module_name");
        ModuleSplit::validate_name("some/path/to/module");
    }

    #[test]
    #[ignore = "slow panic test for debug_assert (~400ms), run manually with --ignored if needed"]
    #[should_panic(expected = "should not include file extension")]
    fn test_split_name_validation_catches_rs_extension() {
        ModuleSplit::validate_name("config/misc.rs");
    }

    #[test]
    #[ignore = "slow panic test for debug_assert (~400ms), run manually with --ignored if needed"]
    #[should_panic(expected = "should not include file extension")]
    fn test_split_name_validation_catches_py_extension() {
        ModuleSplit::validate_name("module.py");
    }

    #[test]
    #[ignore = "slow panic test for debug_assert (~400ms), run manually with --ignored if needed"]
    #[should_panic(expected = "should not include file extension")]
    fn test_split_name_validation_catches_js_extension() {
        ModuleSplit::validate_name("handler.js");
    }

    #[test]
    #[ignore = "slow panic test for debug_assert (~400ms), run manually with --ignored if needed"]
    #[should_panic(expected = "should not include file extension")]
    fn test_split_name_validation_catches_ts_extension() {
        ModuleSplit::validate_name("component.ts");
    }

    // Tests for module name sanitization (Spec 172)
    #[test]
    fn test_sanitize_ampersand_replacement() {
        // Per spec 208: Returns lowercase snake_case
        assert_eq!(sanitize_module_name("parsing"), "parsing");
        assert_eq!(sanitize_module_name("Read & Write"), "read_and_write");
        assert_eq!(
            sanitize_module_name("data_access & validation"),
            "data_access_and_validation"
        );
    }

    #[test]
    fn test_sanitize_multiple_spaces() {
        // Per spec 208: Returns lowercase snake_case
        assert_eq!(sanitize_module_name("data  access"), "data_access");
        // I/O → i_o (slash is converted to underscore, preserving letter boundaries)
        assert_eq!(sanitize_module_name("I/O   utilities"), "i_o_utilities");
        assert_eq!(
            sanitize_module_name("formatting    &    output"),
            "formatting_and_output"
        );
    }

    #[test]
    fn test_sanitize_special_characters() {
        assert_eq!(sanitize_module_name("User's Profile"), "users_profile");
        assert_eq!(
            sanitize_module_name("Data-Access-Layer"),
            "data_access_layer"
        );
        assert_eq!(sanitize_module_name("Config/Settings"), "config_settings");
        // I/O → i_o (slash is converted to underscore, preserving letter boundaries)
        assert_eq!(sanitize_module_name("I/O Utilities"), "i_o_utilities");
    }

    #[test]
    fn test_sanitize_leading_trailing_underscores() {
        assert_eq!(sanitize_module_name("_utilities_"), "utilities");
        assert_eq!(sanitize_module_name("__internal__"), "internal");
        // Per spec 208: Returns lowercase snake_case
        assert_eq!(sanitize_module_name("_data_access_"), "data_access");
    }

    #[test]
    fn test_sanitize_empty_and_whitespace() {
        assert_eq!(sanitize_module_name(""), "");
        assert_eq!(sanitize_module_name("   "), "");
        assert_eq!(sanitize_module_name("___"), "");
    }

    #[test]
    fn test_sanitize_consecutive_underscores() {
        // Per spec 208: Returns lowercase snake_case
        assert_eq!(sanitize_module_name("data__access"), "data_access");
        assert_eq!(
            sanitize_module_name("multiple___underscores"),
            "multiple_underscores"
        );
    }

    #[test]
    fn test_sanitize_mixed_case() {
        assert_eq!(sanitize_module_name("MixedCase"), "mixedcase");
        assert_eq!(sanitize_module_name("CamelCase"), "camelcase");
        assert_eq!(sanitize_module_name("UPPERCASE"), "uppercase");
    }

    #[test]
    fn test_sanitize_numbers() {
        assert_eq!(sanitize_module_name("version2"), "version2");
        assert_eq!(sanitize_module_name("data_v2"), "data_v2");
        assert_eq!(sanitize_module_name("test123"), "test123");
    }

    #[test]
    fn test_sanitize_all_special_chars() {
        assert_eq!(
            sanitize_module_name("@#$%^*()!~`"),
            "" // All special chars removed
        );
        assert_eq!(sanitize_module_name("data@access"), "dataaccess");
    }

    #[test]
    fn test_sanitize_real_world_examples() {
        // Per spec 208: sanitize_module_name returns lowercase snake_case
        assert_eq!(sanitize_module_name("parsing"), "parsing");
        assert_eq!(sanitize_module_name("data_access"), "data_access");
        assert_eq!(sanitize_module_name("utilities"), "utilities");
        // Note: "output" is not automatically mapped to "rendering" - that's done by normalize_category_name
        assert_eq!(sanitize_module_name("output"), "output");
    }

    #[test]
    fn test_sanitize_already_valid_names() {
        // Per spec 208: Names are converted to lowercase snake_case
        assert_eq!(sanitize_module_name("utilities"), "utilities");
        assert_eq!(sanitize_module_name("data_access"), "data_access");
        assert_eq!(sanitize_module_name("io_handler"), "io_handler");
    }

    #[test]
    fn test_sanitize_complex_combinations() {
        assert_eq!(
            sanitize_module_name("User's Data & Config Settings"),
            "users_data_and_config_settings"
        );
        // I/O → i_o (slash is converted to underscore, preserving letter boundaries)
        assert_eq!(
            sanitize_module_name("I/O - Read & Write"),
            "i_o_read_and_write"
        );
    }

    #[test]
    fn test_sanitize_deterministic() {
        // Same input should always produce same output
        let input = "parsing";
        let result1 = sanitize_module_name(input);
        let result2 = sanitize_module_name(input);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_reserved_keyword_rust() {
        assert_eq!(ensure_not_reserved("mod".to_string()), "mod_module");
        assert_eq!(ensure_not_reserved("type".to_string()), "type_module");
        assert_eq!(ensure_not_reserved("impl".to_string()), "impl_module");
        assert_eq!(ensure_not_reserved("trait".to_string()), "trait_module");
    }

    #[test]
    fn test_reserved_keyword_python() {
        assert_eq!(ensure_not_reserved("import".to_string()), "import_module");
        assert_eq!(ensure_not_reserved("class".to_string()), "class_module");
        assert_eq!(ensure_not_reserved("def".to_string()), "def_module");
    }

    #[test]
    fn test_reserved_keyword_not_reserved() {
        assert_eq!(ensure_not_reserved("utilities".to_string()), "utilities");
        assert_eq!(ensure_not_reserved("data".to_string()), "data");
        assert_eq!(
            ensure_not_reserved("my_function".to_string()),
            "my_function"
        );
    }

    #[test]
    fn test_is_reserved_keyword() {
        assert!(is_reserved_keyword("mod"));
        assert!(is_reserved_keyword("import"));
        assert!(!is_reserved_keyword("utilities"));
        assert!(!is_reserved_keyword("data_access"));
    }

    #[test]
    fn test_ensure_unique_name_no_collision() {
        use std::collections::HashSet;
        let existing = HashSet::new();
        assert_eq!(
            ensure_unique_name("utilities".to_string(), &existing),
            "utilities"
        );
    }

    #[test]
    fn test_ensure_unique_name_single_collision() {
        use std::collections::HashSet;
        let mut existing = HashSet::new();
        existing.insert("utilities".to_string());

        assert_eq!(
            ensure_unique_name("utilities".to_string(), &existing),
            "utilities_1"
        );
    }

    #[test]
    fn test_ensure_unique_name_multiple_collisions() {
        use std::collections::HashSet;
        let mut existing = HashSet::new();
        existing.insert("utilities".to_string());
        existing.insert("utilities_1".to_string());
        existing.insert("utilities_2".to_string());

        assert_eq!(
            ensure_unique_name("utilities".to_string(), &existing),
            "utilities_3"
        );
    }

    #[test]
    fn test_ensure_unique_name_deterministic() {
        use std::collections::HashSet;
        let mut existing = HashSet::new();
        existing.insert("data".to_string());

        let result1 = ensure_unique_name("data".to_string(), &existing);
        let result2 = ensure_unique_name("data".to_string(), &existing);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_sanitize_no_valid_characters() {
        // When all characters are removed, should result in empty string
        assert_eq!(sanitize_module_name("@#$%"), "");
        assert_eq!(sanitize_module_name("!!!"), "");
    }

    #[test]
    fn test_sanitize_unicode_characters() {
        // Unicode emojis should be filtered out, returns lowercase snake_case
        assert_eq!(sanitize_module_name("data_🔥_access"), "data_access");
        // Unicode letters (like é) are preserved by is_alphanumeric()
        assert_eq!(sanitize_module_name("café"), "café");
    }

    #[test]
    fn test_sanitize_single_character() {
        assert_eq!(sanitize_module_name("a"), "a");
        // & → and → and_module (since "and" is a Python reserved keyword)
        assert_eq!(sanitize_module_name("&"), "and_module");
        assert_eq!(sanitize_module_name("1"), "1");
    }

    #[test]
    fn test_sanitize_very_long_name() {
        let long_name =
            "This Is A Very Long Module Name With Many Words And Special Characters & Symbols";
        let result = sanitize_module_name(long_name);
        assert!(!result.contains("  "));
        assert!(!result.contains("&"));
        assert!(!result.starts_with('_'));
        assert!(!result.ends_with('_'));
    }

    #[test]
    fn test_sanitize_preserves_alphanumeric() {
        assert_eq!(sanitize_module_name("abc123xyz"), "abc123xyz");
        assert_eq!(sanitize_module_name("test_123_data"), "test_123_data");
    }

    #[test]
    fn test_sanitize_no_consecutive_underscores_in_output() {
        let result = sanitize_module_name("data___access");
        assert!(!result.contains("__"));

        let result = sanitize_module_name("multiple   spaces");
        assert!(!result.contains("__"));
    }

    #[test]
    fn test_sanitize_integration_with_module_split() {
        // Test that sanitized names work in real module split creation
        let responsibility = "parsing";
        let sanitized = sanitize_module_name(responsibility);
        let module_name = format!("mytype_{}", sanitized);

        assert_eq!(module_name, "mytype_parsing");
        assert!(!module_name.contains('&'));
        assert!(!module_name.contains("  "));
    }

    #[test]
    fn test_recommend_module_splits_uses_sanitization() {
        let mut responsibility_groups = HashMap::new();
        responsibility_groups.insert(
            "parsing".to_string(),
            vec![
                "parse_a".to_string(),
                "parse_b".to_string(),
                "parse_c".to_string(),
                "parse_d".to_string(),
                "parse_e".to_string(),
                "parse_f".to_string(),
            ],
        );

        let splits = recommend_module_splits("MyType", &[], &responsibility_groups);

        assert_eq!(splits.len(), 1);
        assert_eq!(splits[0].suggested_name, "mytype_parsing");
        assert!(!splits[0].suggested_name.contains('&'));
    }

    // Tests for confidence-based classification (Spec 174)

    #[test]
    fn test_high_confidence_classification() {
        let result = infer_responsibility_with_confidence("parse_json", None);
        assert!(result.category.is_some());
        assert_eq!(result.category.unwrap(), "Parsing");
        assert!(result.confidence >= MINIMUM_CONFIDENCE);
    }

    #[test]
    fn test_minimum_confidence_threshold() {
        // Domain-specific methods get lower confidence
        let result = infer_responsibility_with_confidence("populate_registry", None);
        // Should be below threshold if it's domain-specific
        if result.confidence < MINIMUM_CONFIDENCE {
            assert!(result.category.is_none());
        }
    }

    #[test]
    fn test_recognized_patterns_have_high_confidence() {
        // Per spec 208: Category names now use Title Case (e.g., "Rendering" not "output")
        let test_cases = vec![
            ("format_output", "Rendering"),   // format_* is now Rendering
            ("parse_input", "Parsing"),       // parse_* is Parsing
            ("validate_data", "Validation"),  // validate_* is Validation
            ("calculate_sum", "Computation"), // calculate_* is Computation
        ];

        for (method_name, expected_category) in test_cases {
            let result = infer_responsibility_with_confidence(method_name, None);
            assert!(
                result.category.is_some(),
                "Expected {} to be classified",
                method_name
            );
            assert_eq!(
                result.category.unwrap(),
                expected_category,
                "Wrong category for {}",
                method_name
            );
            assert!(
                result.confidence >= 0.85,
                "Expected high confidence for {}",
                method_name
            );
        }
    }

    #[test]
    fn test_domain_category_low_confidence() {
        // Domain-specific categories (fallback) should have low confidence
        // and be rejected by the confidence threshold
        let result = infer_responsibility_with_confidence("helper_function", None);

        // Should be refused due to low confidence
        assert!(
            result.category.is_none(),
            "Domain fallback should have low confidence and be refused"
        );
        assert!(
            result.confidence < MINIMUM_CONFIDENCE,
            "Domain category confidence should be below minimum threshold"
        );
    }

    #[test]
    fn test_classification_result_structure() {
        let result = infer_responsibility_with_confidence("validate_input", None);

        // Check all fields are populated correctly
        assert!(!result.signals_used.is_empty());
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
        if result.category.is_some() {
            assert!(result.confidence >= MINIMUM_CONFIDENCE);
        }
    }

    #[test]
    fn test_behavioral_categorization_confidence() {
        // Behavioral patterns should have medium-high confidence
        let behavioral_methods = vec!["render_ui", "handle_event", "save_data"];

        for method_name in behavioral_methods {
            let result = infer_responsibility_with_confidence(method_name, None);
            if result.category.is_some() {
                // Behavioral patterns should be above minimum but may vary
                assert!(
                    result.confidence >= MINIMUM_CONFIDENCE,
                    "{} confidence too low: {}",
                    method_name,
                    result.confidence
                );
            }
        }
    }

    #[test]
    fn test_confidence_thresholds_enforced() {
        // Test that confidence thresholds are actually enforced
        let test_methods = vec![
            "parse_json",
            "format_output",
            "helper_method",
            "do_something",
        ];

        for method_name in test_methods {
            let result = infer_responsibility_with_confidence(method_name, None);

            if let Some(_category) = &result.category {
                // If category is set, confidence must be >= MINIMUM_CONFIDENCE
                assert!(
                    result.confidence >= MINIMUM_CONFIDENCE,
                    "{} classified with confidence {} below minimum {}",
                    method_name,
                    result.confidence,
                    MINIMUM_CONFIDENCE
                );
            } else {
                // If category is None, confidence must be < MINIMUM_CONFIDENCE
                assert!(
                    result.confidence < MINIMUM_CONFIDENCE,
                    "{} refused classification but confidence {} >= minimum {}",
                    method_name,
                    result.confidence,
                    MINIMUM_CONFIDENCE
                );
            }
        }
    }
}
