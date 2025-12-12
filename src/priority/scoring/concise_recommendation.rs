// Concise recommendation generation (spec 138a, spec 176)
//
// This module generates actionable recommendations with:
// - Maximum 5 high-level steps per recommendation
// - Clear impact estimates for each step
// - Difficulty indicators (Easy/Medium/Hard)
// - Executable commands for each step
// - Estimated total effort in hours
// - Pattern-based complexity recommendations (spec 176)

use crate::core::FunctionMetrics;
use crate::priority::complexity_patterns::{ComplexityMetrics, ComplexityPattern};
use crate::priority::refactoring_impact::RefactoringImpact;
use crate::priority::semantic_classifier::FunctionRole;
use crate::priority::{
    ActionStep, ActionableRecommendation, DebtType, Difficulty, FunctionVisibility,
    TransitiveCoverage,
};

use super::test_calculation::{calculate_tests_needed, ComplexityTier as TestComplexityTier};

/// Complexity tier classification for tier-appropriate recommendations
///
/// # Tier Definitions
///
/// - **Low** (cyclo < 8, cognitive < 15): Well-structured, easy to understand
///   - Recommendation: Maintain current patterns
///   - Example: Simple validation, accessors, small functions
///
/// - **Moderate** (cyclo 8-14, cognitive 15-24): Manageable but approaching limits
///   - Recommendation: Optional preventive refactoring
///   - Example: Business logic with moderate branching
///
/// - **High** (cyclo 15-24, cognitive 25-39): Exceeds maintainability thresholds
///   - Recommendation: Refactoring required
///   - Example: Complex orchestration, large case statements
///
/// - **VeryHigh** (cyclo >= 25, cognitive >= 40): Critical complexity
///   - Recommendation: Significant refactoring required
///   - Example: God functions, tangled logic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecommendationComplexityTier {
    /// Low complexity: cyclo < 8, cognitive < 15
    Low,
    /// Moderate complexity: cyclo 8-14, cognitive 15-24
    Moderate,
    /// High complexity: cyclo 15-24, cognitive 25-39
    High,
    /// Very high complexity: cyclo >= 25, cognitive >= 40
    VeryHigh,
}

/// Classify complexity tier based on cyclomatic and cognitive complexity
fn classify_complexity_tier(cyclomatic: u32, cognitive: u32) -> RecommendationComplexityTier {
    // Use the higher of the two complexities to determine tier
    if cyclomatic >= 25 || cognitive >= 40 {
        RecommendationComplexityTier::VeryHigh
    } else if cyclomatic >= 15 || cognitive >= 25 {
        RecommendationComplexityTier::High
    } else if cyclomatic >= 8 || cognitive >= 15 {
        RecommendationComplexityTier::Moderate
    } else {
        RecommendationComplexityTier::Low
    }
}

/// Calculate appropriate complexity reduction target based on current tier
///
/// # Target Selection Strategy
///
/// - **Low tier**: Maintain or slight improvement (current - 1, min 3)
/// - **Moderate tier**: Aim for single-digit (8 if >= 10, else current - 3)
/// - **High tier**: Aim for moderate complexity (min(10, current - 2))
/// - **Very High tier**: Significant reduction (half current, capped at 10-15)
///
/// # Examples
///
/// - complexity=6 → target=5 (maintain)
/// - complexity=9 → target=6 (preventive)
/// - complexity=12 → target=8 (reduce to single-digit)
/// - complexity=20 → target=10 (significant reduction)
/// - complexity=40 → target=15 (very high → high tier)
fn calculate_target_complexity(current: u32, tier: RecommendationComplexityTier) -> u32 {
    match tier {
        RecommendationComplexityTier::Low => {
            // Already low - maintain or slightly improve
            current.saturating_sub(1).max(3)
        }
        RecommendationComplexityTier::Moderate => {
            // Aim for single-digit complexity
            if current >= 10 {
                // 10-14 → target 8
                8
            } else {
                // 8-9 → target 5-6 (reduce by 2-3)
                current.saturating_sub(3).max(5)
            }
        }
        RecommendationComplexityTier::High => {
            // Aim for moderate complexity, but never exceed current
            // For low cyclomatic but high cognitive, reduce modestly
            10.min(current.saturating_sub(2).max(5))
        }
        RecommendationComplexityTier::VeryHigh => {
            // Significant reduction needed (aim for 10-15)
            let half_current = current / 2;
            let clamped = half_current.clamp(10, 15);
            clamped.min(current.saturating_sub(5))
        }
    }
}

/// Generate concise recommendation from debt type and metrics (spec 201)
/// Returns None if the debt pattern doesn't warrant a recommendation (e.g., clean dispatcher)
pub fn generate_concise_recommendation(
    debt_type: &DebtType,
    metrics: &FunctionMetrics,
    role: FunctionRole,
    coverage: &Option<TransitiveCoverage>,
) -> Option<ActionableRecommendation> {
    Some(match debt_type {
        DebtType::TestingGap {
            coverage: cov,
            cyclomatic,
            cognitive,
        } => generate_testing_gap_steps(*cov, *cyclomatic, *cognitive, metrics, role, coverage),
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        } => generate_complexity_steps(*cyclomatic, *cognitive, metrics)?,
        DebtType::DeadCode {
            visibility,
            cyclomatic,
            cognitive,
            ..
        } => generate_dead_code_steps(visibility, *cyclomatic, *cognitive, metrics),
        _ => {
            // Fallback for other debt types - use legacy format
            ActionableRecommendation {
                primary_action: "Address technical debt".to_string(),
                rationale: "This item requires attention".to_string(),
                implementation_steps: vec!["Review and address the issue".to_string()],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            }
        }
    })
}

/// Generate testing gap recommendation with max 5 steps (spec 183)
fn generate_testing_gap_steps(
    coverage_pct: f64,
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
    _role: FunctionRole,
    _transitive_cov: &Option<TransitiveCoverage>,
) -> ActionableRecommendation {
    // Use adjusted complexity if available (spec 183)
    let effective_cyclomatic = metrics
        .adjusted_complexity
        .map(|adj| adj.round() as u32)
        .unwrap_or(cyclomatic);

    let tier = if effective_cyclomatic > 30 {
        TestComplexityTier::High
    } else if effective_cyclomatic > 10 {
        TestComplexityTier::Moderate
    } else {
        TestComplexityTier::Simple
    };

    let test_result = calculate_tests_needed(effective_cyclomatic, coverage_pct, Some(tier));
    let tests_needed = test_result.count;
    let coverage_gap = (100.0 - coverage_pct * 100.0) as u32;

    let mut steps = vec![];

    // Step 1: Add tests (highest impact)
    if tests_needed > 0 {
        steps.push(ActionStep {
            description: format!(
                "Add {} tests for {}% coverage gap",
                tests_needed, coverage_gap
            ),
            impact: format!("+{} tests, reduce risk", tests_needed),
            difficulty: Difficulty::for_testing(tests_needed, effective_cyclomatic),
            commands: vec![
                format!("cargo test {}::", metrics.name),
                "# Write focused tests covering critical paths".to_string(),
            ],
        });
    }

    // Step 2: Refactoring (only if complex)
    if effective_cyclomatic > 15 || cognitive > 20 {
        let target_complexity = 10;
        let complexity_reduction = (effective_cyclomatic.saturating_sub(target_complexity)).max(5);

        steps.push(ActionStep {
            description: "Extract complex branches into focused functions".to_string(),
            impact: format!("-{} complexity", complexity_reduction),
            difficulty: Difficulty::for_refactoring(effective_cyclomatic, cognitive),
            commands: vec!["cargo clippy -- -W clippy::cognitive_complexity".to_string()],
        });
    }

    // Step 3: Verify (always include if we have steps)
    if !steps.is_empty() {
        steps.push(ActionStep {
            description: "Verify tests pass and coverage improved".to_string(),
            impact: if tests_needed > 0 {
                format!("Confirmed +{}% coverage", coverage_gap)
            } else {
                "Confirmed refactoring safe".to_string()
            },
            difficulty: Difficulty::Easy,
            commands: vec![
                "cargo test --all".to_string(),
                "# Run coverage tool to verify improvement".to_string(),
            ],
        });
    }

    let estimated_effort = estimate_effort(effective_cyclomatic, tests_needed);

    ActionableRecommendation {
        primary_action: if tests_needed > 0 {
            format!("Add {} tests for untested branches", tests_needed)
        } else {
            "Maintain test coverage".to_string()
        },
        rationale: format!(
            "Function has {}% coverage with complexity {}/{}. Needs {} tests minimum.",
            (coverage_pct * 100.0) as u32,
            effective_cyclomatic,
            cognitive,
            tests_needed
        ),
        implementation_steps: vec![], // Legacy field - empty for new recommendations
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(estimated_effort),
    }
}

/// Generate complexity hotspot recommendation using pattern detection (spec 176, spec 201)
/// Returns None if the complexity pattern doesn't warrant a recommendation (e.g., clean dispatcher)
fn generate_complexity_steps(
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
) -> Option<ActionableRecommendation> {
    // Extract pattern signals from Rust-specific pattern data
    let (validation_signals, state_signals, coordinator_signals) = metrics
        .language_specific
        .as_ref()
        .map_or((None, None, None), |lang_data| {
            let crate::core::LanguageSpecificData::Rust(rust_patterns) = lang_data;
            (
                rust_patterns.validation_signals.clone(),
                rust_patterns.state_machine_signals.clone(),
                rust_patterns.coordinator_signals.clone(),
            )
        });

    // Detect complexity pattern
    let complexity_metrics = ComplexityMetrics {
        cyclomatic,
        cognitive,
        nesting: metrics.nesting,
        // Use token_entropy (Shannon entropy) for pattern detection, not effective_complexity
        entropy_score: metrics.entropy_score.as_ref().map(|e| e.token_entropy),
        state_signals,
        coordinator_signals,
        validation_signals,
    };

    let pattern = ComplexityPattern::detect(&complexity_metrics);

    // Generate pattern-specific recommendation
    match pattern {
        ComplexityPattern::StateMachine {
            state_transitions,
            match_expression_count,
            cyclomatic: cyclo,
            cognitive: cog,
            nesting,
        } => {
            // Spec 203: Returns None for clean patterns
            generate_state_machine_recommendation(
                state_transitions,
                match_expression_count,
                cyclo,
                cog,
                nesting,
                metrics,
            )
        }
        ComplexityPattern::Coordinator {
            action_count,
            comparison_count,
            cyclomatic: cyclo,
            cognitive: cog,
        } => Some(generate_coordinator_recommendation(
            action_count,
            comparison_count,
            cyclo,
            cog,
            metrics,
        )),
        ComplexityPattern::Dispatcher {
            branch_count,
            cognitive_ratio,
            inline_logic_branches,
        } => generate_dispatcher_recommendation(
            branch_count,
            cognitive_ratio,
            inline_logic_branches,
            cyclomatic,
            cognitive,
            metrics,
        ),
        ComplexityPattern::RepetitiveValidation {
            validation_count,
            entropy,
            cyclomatic: _,
        } => Some(generate_repetitive_validation_recommendation(
            validation_count,
            entropy,
            metrics,
        )),
        ComplexityPattern::HighNesting {
            nesting_depth,
            cognitive_score,
            ratio,
        } => Some(generate_nesting_recommendation(
            nesting_depth,
            cognitive_score,
            ratio,
            metrics,
        )),
        ComplexityPattern::HighBranching {
            branch_count,
            cyclomatic: _,
        } => Some(generate_branching_recommendation(
            branch_count,
            cyclomatic,
            metrics,
        )),
        ComplexityPattern::MixedComplexity {
            nesting_depth,
            cyclomatic: cyclo,
            cognitive: cog,
        } => Some(generate_mixed_recommendation(
            nesting_depth,
            cyclo,
            cog,
            metrics,
        )),
        ComplexityPattern::ChaoticStructure { entropy, .. } => Some(
            generate_chaotic_recommendation(entropy, cyclomatic, cognitive, metrics),
        ),
        ComplexityPattern::ModerateComplexity { .. } => Some(generate_moderate_recommendation(
            cyclomatic, cognitive, metrics,
        )),
    }
}

/// Generate recommendation for high nesting pattern
fn generate_nesting_recommendation(
    nesting: u32,
    cognitive: u32,
    ratio: f64,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    let early_return_impact = RefactoringImpact::early_returns(nesting);
    let predicate_impact = RefactoringImpact::predicate_functions(3); // Estimate 3 conditionals
    let language = crate::core::Language::from_path(&metrics.file);

    let steps = vec![
        ActionStep {
            description: "Apply early returns for error conditions".to_string(),
            impact: format!(
                "-{} cognitive ({} impact)",
                early_return_impact.complexity_reduction,
                early_return_impact.confidence.as_str()
            ),
            difficulty: Difficulty::Medium,
            commands: add_language_hints_for_early_returns(&language),
        },
        ActionStep {
            description: "Extract nested conditionals into predicate functions".to_string(),
            impact: format!(
                "-{} cognitive ({} impact)",
                predicate_impact.complexity_reduction,
                predicate_impact.confidence.as_str()
            ),
            difficulty: Difficulty::Medium,
            commands: vec![
                "# Find: nested if within if/match".to_string(),
                "# Create: is_valid(), should_process() functions".to_string(),
            ],
        },
        ActionStep {
            description: "Verify nesting reduced to < 3 levels".to_string(),
            impact: "Target: nesting < 3, cognitive < 25".to_string(),
            difficulty: Difficulty::Easy,
            commands: add_language_verification_commands(&language),
        },
    ];

    let estimated_effort = (nesting as f32 - 2.0) * 0.5; // ~30min per nesting level

    ActionableRecommendation {
        primary_action: format!(
            "Reduce nesting from {} to 2 levels (primary impact: -{})",
            nesting,
            early_return_impact.complexity_reduction + predicate_impact.complexity_reduction
        ),
        rationale: format!(
            "Deep nesting (depth {}) drives cognitive complexity to {}. \
             Cognitive/Cyclomatic ratio of {:.1}x confirms nesting is primary issue.",
            nesting, cognitive, ratio
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(estimated_effort.max(0.5)),
    }
}

/// Add language-specific hints for early returns (spec 176)
fn add_language_hints_for_early_returns(language: &crate::core::Language) -> Vec<String> {
    match language {
        crate::core::Language::Rust => vec![
            "# Use `?` operator for Result propagation".to_string(),
            "# Pattern: if let ... else { return Err(...) }".to_string(),
            "# Replace nested matches with guard patterns".to_string(),
        ],
        crate::core::Language::Python => vec![
            "# Use early returns for validation".to_string(),
            "# Pattern: if not valid: return error".to_string(),
            "# Reduce try-except nesting with early checks".to_string(),
        ],
        _ => vec![
            "# Move validation checks to function start".to_string(),
            "# Return early on invalid states".to_string(),
            "# Pattern: nested if/match -> guard + early return".to_string(),
        ],
    }
}

/// Add language-specific verification commands (spec 176)
fn add_language_verification_commands(language: &crate::core::Language) -> Vec<String> {
    match language {
        crate::core::Language::Rust => vec![
            "cargo clippy -- -W clippy::cognitive_complexity".to_string(),
            "cargo test --all".to_string(),
        ],
        crate::core::Language::Python => vec![
            "# Run pylint or flake8 for complexity checks".to_string(),
            "pytest".to_string(),
        ],
        _ => vec!["# Run your test suite".to_string()],
    }
}

/// Generate recommendation for high branching pattern
fn generate_branching_recommendation(
    branch_count: u32,
    cyclomatic: u32,
    _metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    let functions_to_extract = calculate_functions_to_extract(cyclomatic, 0);
    let extraction_impact =
        RefactoringImpact::extract_function(branch_count / functions_to_extract);

    let steps = vec![
        ActionStep {
            description: "Identify decision clusters (related conditional logic)".to_string(),
            impact: format!(
                "-{} complexity per extraction ({} impact)",
                extraction_impact.complexity_reduction,
                extraction_impact.confidence.as_str()
            ),
            difficulty: Difficulty::Medium,
            commands: vec![
                "# Group related if/match statements".to_string(),
                "# Each cluster becomes focused function".to_string(),
            ],
        },
        ActionStep {
            description: "Extract setup/validation logic to separate function".to_string(),
            impact: format!("-{} complexity", extraction_impact.complexity_reduction),
            difficulty: Difficulty::Medium,
            commands: vec!["# Returns Result<PreparedState, Error>".to_string()],
        },
        ActionStep {
            description: "Verify cyclomatic < 10 per function".to_string(),
            impact: "Target: cyclomatic < 10".to_string(),
            difficulty: Difficulty::Easy,
            commands: vec!["cargo test --all".to_string()],
        },
    ];

    let estimated_effort = (cyclomatic as f32 / 10.0) * 1.5; // ~1.5hr per 10 complexity

    ActionableRecommendation {
        primary_action: format!(
            "Split into {} focused functions by decision clusters",
            functions_to_extract
        ),
        rationale: format!(
            "Many decision points ({} branches) drive cyclomatic complexity. \
             Extract cohesive logic into focused functions.",
            branch_count
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(estimated_effort),
    }
}

/// Generate recommendation for mixed complexity pattern
fn generate_mixed_recommendation(
    nesting: u32,
    cyclomatic: u32,
    cognitive: u32,
    _metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    let early_return_impact = RefactoringImpact::early_returns(nesting);
    let extraction_impact = RefactoringImpact::extract_function(cyclomatic / 3);

    let steps = vec![
        ActionStep {
            description: "Phase 1: Apply early returns and guard clauses".to_string(),
            impact: format!(
                "-{} cognitive (makes branching clearer)",
                early_return_impact.complexity_reduction
            ),
            difficulty: Difficulty::Medium,
            commands: vec!["# Flatten nesting first".to_string()],
        },
        ActionStep {
            description: "Phase 2: Extract functions from flattened structure".to_string(),
            impact: format!("-{} cyclomatic", extraction_impact.complexity_reduction),
            difficulty: Difficulty::Hard,
            commands: vec!["# Identify decision clusters after flattening".to_string()],
        },
        ActionStep {
            description: "Verify: nesting < 3, cyclomatic < 10".to_string(),
            impact: "Both metrics in healthy range".to_string(),
            difficulty: Difficulty::Easy,
            commands: vec![
                "cargo clippy -- -W clippy::cognitive_complexity".to_string(),
                "cargo test --all".to_string(),
            ],
        },
    ];

    let estimated_effort = ((nesting as f32 - 2.0) * 0.5) + ((cyclomatic as f32 / 10.0) * 1.5);

    ActionableRecommendation {
        primary_action: "Reduce nesting FIRST, then extract functions (two-phase approach)"
            .to_string(),
        rationale: format!(
            "Both nesting ({} levels) AND branching ({} branches) drive complexity to {}/{}. \
             Mixed complexity requires phased refactoring.",
            nesting, cyclomatic, cyclomatic, cognitive
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(estimated_effort),
    }
}

/// Generate recommendation for chaotic structure pattern
fn generate_chaotic_recommendation(
    entropy: f64,
    cyclomatic: u32,
    cognitive: u32,
    _metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    let steps = vec![
        ActionStep {
            description: "Standardize error handling patterns".to_string(),
            impact: "More predictable control flow".to_string(),
            difficulty: Difficulty::Medium,
            commands: vec![
                "# Convert all error handling to Result<?> propagation".to_string(),
                "# Replace unwrap()/expect() with proper error handling".to_string(),
            ],
        },
        ActionStep {
            description: "Group related state transitions".to_string(),
            impact: "Clear state evolution, fewer bugs".to_string(),
            difficulty: Difficulty::Medium,
            commands: vec!["# Collect scattered state changes into cohesive blocks".to_string()],
        },
        ActionStep {
            description: "Re-run entropy calculation after standardization".to_string(),
            impact: format!("Target: entropy < 0.35 (currently {:.2})", entropy),
            difficulty: Difficulty::Easy,
            commands: vec!["# Then proceed with complexity reduction".to_string()],
        },
    ];

    ActionableRecommendation {
        primary_action: "Standardize control flow patterns before refactoring".to_string(),
        rationale: format!(
            "High token entropy ({:.2}) indicates inconsistent structure. \
             Standardize patterns to enable safe refactoring of {}/{} complexity.",
            entropy, cyclomatic, cognitive
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(2.0), // Chaotic code takes longer
    }
}

/// Generate recommendation for state machine pattern (spec 203: enhanced with arm-level analysis)
fn generate_state_machine_recommendation(
    _transitions: u32,
    _match_expression_count: u32,
    cyclomatic: u32,
    cognitive: u32,
    nesting: u32,
    metrics: &FunctionMetrics,
) -> Option<ActionableRecommendation> {
    // Extract signals from metrics (spec 203)
    let signals = metrics
        .language_specific
        .as_ref()
        .and_then(|lang| match lang {
            crate::core::LanguageSpecificData::Rust(rust) => rust.state_machine_signals.as_ref(),
        });

    // If no signals or no complex arms detected, fall back to generic recommendation
    // Don't filter out - complexity thresholds already determined this is debt
    let signals = match signals {
        Some(s) if s.complex_inline_arms > 0 => s,
        _ => {
            // Fall back to generic complexity recommendation
            return Some(generate_moderate_recommendation(
                cyclomatic, cognitive, metrics,
            ));
        }
    };
    let _ = nesting; // Used in fallback path

    let language = crate::core::Language::from_path(&metrics.file);

    // Build state type description
    let state_type = if metrics.name.contains("main") {
        "commands"
    } else if metrics.name.contains("handle") {
        "states"
    } else {
        "transitions"
    };

    // Build breakdown explanation (spec 203)
    let breakdown = if signals.primary_match_arms > 0 {
        format!(
            "{} {} ({} already extracted, {} trivial, {} need extraction)",
            signals.primary_match_arms,
            state_type,
            signals.delegated_arms,
            signals.trivial_arms,
            signals.complex_inline_arms
        )
    } else {
        format!("{} state transitions", signals.transition_count)
    };

    // Calculate realistic complexity reduction (spec 203)
    let reduction_per_arm = 3;
    let cognitive_reduction_per_arm = 5;

    let total_cyclo_reduction = signals.complex_inline_arms * reduction_per_arm;
    let total_cog_reduction = signals.complex_inline_arms * cognitive_reduction_per_arm;

    let baseline_match = signals.primary_match_arms.saturating_sub(1);

    let projected_cyclo = cyclomatic
        .saturating_sub(total_cyclo_reduction)
        .max(baseline_match);
    let projected_cog = cognitive
        .saturating_sub(total_cog_reduction)
        .max(baseline_match * 2);

    // Generate extraction impact
    let extraction_impact =
        RefactoringImpact::state_transition_extraction(signals.complex_inline_arms);

    let steps = vec![
        ActionStep {
            description: format!(
                "Extract {} inline {} into handler functions",
                signals.complex_inline_arms,
                if signals.complex_inline_arms == 1 {
                    "handler"
                } else {
                    "handlers"
                }
            ),
            impact: format!(
                "-{} complexity ({} inline LOC moved, {} impact)",
                total_cyclo_reduction,
                signals.total_inline_lines,
                extraction_impact.confidence.as_str()
            ),
            difficulty: Difficulty::Medium,
            commands: vec![
                "# Pattern: Commands::Foo { fields } => handle_foo_command(fields)?".to_string(),
                "# Move config building into handle_foo_command()".to_string(),
            ],
        },
        ActionStep {
            description: "Verify all command arms delegate to handlers".to_string(),
            impact: format!(
                "Consistent pattern: {} of {} arms delegated",
                signals.delegated_arms + signals.complex_inline_arms,
                signals.primary_match_arms
            ),
            difficulty: Difficulty::Easy,
            commands: add_language_verification_commands(&language),
        },
    ];

    let estimated_effort = (signals.complex_inline_arms as f32) * 0.75;

    Some(ActionableRecommendation {
        primary_action: format!(
            "Extract {} inline {} (state machine cleanup)",
            signals.complex_inline_arms,
            if signals.complex_inline_arms == 1 {
                "handler"
            } else {
                "handlers"
            }
        ),
        rationale: format!(
            "State machine pattern with {}. \
             Extracting {} inline handlers will reduce complexity from {}/{} to ~{}/{} \
             and establish consistent delegation pattern.",
            breakdown,
            signals.complex_inline_arms,
            cyclomatic,
            cognitive,
            projected_cyclo,
            projected_cog
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(estimated_effort.max(0.5)),
    })
}

/// Generate recommendation for coordinator pattern
fn generate_coordinator_recommendation(
    action_count: u32,
    comparison_count: u32,
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    let extraction_impact =
        RefactoringImpact::coordinator_extraction(action_count, comparison_count);
    let language = crate::core::Language::from_path(&metrics.file);

    let steps = vec![
        ActionStep {
            description: "Extract action selection logic into pure functions".to_string(),
            impact: format!(
                "-{} complexity ({} impact)",
                extraction_impact.complexity_reduction,
                extraction_impact.confidence.as_str()
            ),
            difficulty: Difficulty::Medium,
            commands: vec![
                "# Extract: fn select_actions_for_state_diff(...) -> Vec<Action>".to_string(),
                "# Pure functions easier to test and reason about".to_string(),
            ],
        },
        ActionStep {
            description: "Replace state comparisons with diff calculation".to_string(),
            impact: format!("-{} comparisons (single diff pass)", comparison_count),
            difficulty: Difficulty::Medium,
            commands: vec![
                "# Create: fn calculate_state_diff(current, target) -> StateDiff".to_string(),
                "# Pattern match on diff instead of individual field checks".to_string(),
            ],
        },
        ActionStep {
            description: "Verify actions with property-based tests".to_string(),
            impact: "Ensure action correctness across state combinations".to_string(),
            difficulty: Difficulty::Medium,
            commands: add_language_verification_commands(&language),
        },
    ];

    let estimated_effort = (action_count as f32 + comparison_count as f32) * 0.3; // ~20min per action/comparison

    ActionableRecommendation {
        primary_action: "Extract state reconciliation logic into transition functions".to_string(),
        rationale: format!(
            "Coordinator pattern detected with {} actions and {} state comparisons. \
             Extracting transitions will reduce complexity from {}/{} to ~{}/{}.",
            action_count,
            comparison_count,
            cyclomatic,
            cognitive,
            cyclomatic.saturating_sub(extraction_impact.complexity_reduction / 2),
            cognitive.saturating_sub(extraction_impact.complexity_reduction / 2)
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(estimated_effort),
    }
}

/// Generate recommendation for dispatcher pattern (spec 189, spec 201, spec 206)
/// Returns None for clean dispatchers that need no refactoring
fn generate_dispatcher_recommendation(
    branch_count: u32,
    cognitive_ratio: f64,
    inline_logic_branches: u32,
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
) -> Option<ActionableRecommendation> {
    let language = crate::core::Language::from_path(&metrics.file);

    // Spec 206: Clean dispatcher with no inline logic and flat structure needs no refactoring
    // This is a well-structured dispatcher pattern - suppress recommendation
    if inline_logic_branches == 0 && metrics.nesting <= 2 {
        // Clean dispatcher: all arms delegate, flat structure, no inline logic
        // This is intentional architecture, not debt
        return None;
    }

    // If no inline logic but higher nesting, fall back to generic recommendation
    if inline_logic_branches == 0 {
        return Some(generate_moderate_recommendation(
            cyclomatic, cognitive, metrics,
        ));
    }
    let _ = (branch_count, cognitive_ratio); // Used for specific recommendation below

    // Dispatcher with inline logic - recommend extraction
    let extraction_impact = RefactoringImpact::extract_function(inline_logic_branches);

    let steps = vec![
        ActionStep {
            description: format!(
                "Extract inline logic from {} branches into helper functions",
                inline_logic_branches
            ),
            impact: format!(
                "-{} cognitive complexity ({} impact)",
                extraction_impact.complexity_reduction,
                extraction_impact.confidence.as_str()
            ),
            difficulty: Difficulty::Medium,
            commands: vec![
                "# Identify branches with >2 lines of logic".to_string(),
                "# Extract into focused helper functions".to_string(),
                "# Keep dispatcher as thin router (1-2 lines per branch)".to_string(),
            ],
        },
        ActionStep {
            description: "Maintain shallow dispatcher structure".to_string(),
            impact: format!(
                "Target: {} branches with simple delegation only",
                branch_count
            ),
            difficulty: Difficulty::Easy,
            commands: vec![
                "# Each branch should be: Some(Cmd) => handle_cmd()".to_string(),
                "# No conditionals or loops within branches".to_string(),
            ],
        },
        ActionStep {
            description: "Verify cognitive complexity reduced".to_string(),
            impact: format!(
                "Target: cognitive < {} (from {})",
                (branch_count as f64 * 0.4) as u32,
                cognitive
            ),
            difficulty: Difficulty::Easy,
            commands: add_language_verification_commands(&language),
        },
    ];

    let estimated_effort = (inline_logic_branches as f32) * 0.3; // ~20min per extraction

    let severity = match inline_logic_branches {
        1..=3 => "Low",
        4..=8 => "Medium",
        _ => "High",
    };

    Some(ActionableRecommendation {
        primary_action: format!(
            "Extract inline logic from {} branches (dispatcher pattern)",
            inline_logic_branches
        ),
        rationale: format!(
            "Dispatcher pattern detected with {} branches (ratio: {:.2}). \
             {} branches have inline logic that should be extracted into helper functions. \
             Dispatcher complexity: {}/{} (severity: {}).",
            branch_count, cognitive_ratio, inline_logic_branches, cyclomatic, cognitive, severity
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(estimated_effort.max(0.5)),
    })
}

/// Generate recommendation for repetitive validation pattern (spec 180)
fn generate_repetitive_validation_recommendation(
    validation_count: u32,
    entropy: f64,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    let boilerplate_reduction = RefactoringImpact::validation_extraction(validation_count);
    let language = crate::core::Language::from_path(&metrics.file);

    let steps = vec![
        ActionStep {
            description: "Replace imperative validation with declarative pattern".to_string(),
            impact: format!(
                "-{} LOC boilerplate, improved maintainability ({} impact)",
                validation_count * 2,
                boilerplate_reduction.confidence.as_str()
            ),
            difficulty: Difficulty::Medium,
            commands: add_declarative_validation_examples(&language, validation_count),
        },
        ActionStep {
            description: "Extract validation rules into data structure".to_string(),
            impact: format!(
                "Single source of truth for {} validation rules",
                validation_count
            ),
            difficulty: Difficulty::Medium,
            commands: vec![
                "# Define validation schema/rules declaratively".to_string(),
                format!(
                    "# Example: [{} required fields in config]",
                    validation_count
                ),
            ],
        },
        ActionStep {
            description: "Add comprehensive validation tests".to_string(),
            impact: "Ensure all validation rules covered by tests".to_string(),
            difficulty: Difficulty::Easy,
            commands: vec![
                "cargo test validate_*".to_string(),
                "# Test each validation rule independently".to_string(),
            ],
        },
    ];

    let estimated_effort = (validation_count as f32 / 10.0) * 1.5; // ~1.5hr per 10 validations

    ActionableRecommendation {
        primary_action: format!(
            "Replace {} repetitive validation checks with declarative pattern",
            validation_count
        ),
        rationale: format!(
            "Repetitive validation pattern detected (entropy {:.2}, {} checks). \
             Low entropy indicates boilerplate, not genuine complexity - \
             cognitive load is dampened accordingly. \
             Refactoring improves maintainability and reduces error-prone boilerplate.",
            entropy, validation_count
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(estimated_effort.max(0.5)),
    }
}

/// Add language-specific declarative validation examples (spec 180)
fn add_declarative_validation_examples(
    language: &crate::core::Language,
    _count: u32,
) -> Vec<String> {
    match language {
        crate::core::Language::Rust => vec![
            "# Option 1: Builder pattern with validation".to_string(),
            "# ConfigBuilder::new().required(\"output_dir\").build()?".to_string(),
            "".to_string(),
            "# Option 2: Validation trait".to_string(),
            "# impl Validate for Config { fn validate(&self) -> Result<()> }".to_string(),
            "".to_string(),
            "# Option 3: Macro-based validation".to_string(),
            "# #[validate(required = [\"output_dir\", \"max_workers\", ...])]".to_string(),
        ],
        crate::core::Language::Python => vec![
            "# Option 1: Pydantic model".to_string(),
            "# class Config(BaseModel):".to_string(),
            "#     output_dir: str".to_string(),
            "#     max_workers: int".to_string(),
            "".to_string(),
            "# Option 2: attrs with validators".to_string(),
            "# @define".to_string(),
            "# class Config:".to_string(),
            "#     output_dir: str = field(validator=instance_of(str))".to_string(),
        ],
        _ => vec!["# Use declarative validation approach for your language".to_string()],
    }
}

/// Generate recommendation for moderate complexity (spec 178, spec 183)
fn generate_moderate_recommendation(
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    // Use adjusted complexity if available (spec 183)
    let effective_cyclomatic = metrics
        .adjusted_complexity
        .map(|adj| adj.round() as u32)
        .unwrap_or(cyclomatic);

    let tier = classify_complexity_tier(effective_cyclomatic, cognitive);
    let target = calculate_target_complexity(effective_cyclomatic, tier);
    let reduction = effective_cyclomatic.saturating_sub(target);

    match tier {
        RecommendationComplexityTier::Low => {
            // Already below thresholds - maintenance recommendation
            let steps = vec![ActionStep {
                description: "Add tests to preserve behavior during future changes".to_string(),
                impact: "+safety for refactoring".to_string(),
                difficulty: Difficulty::Easy,
                commands: vec![format!("cargo test {}::", metrics.name)],
            }];

            ActionableRecommendation {
                primary_action: "Maintain current low complexity".to_string(),
                rationale: format!(
                    "Function has low complexity ({}/{}). \
                     Continue following current patterns to keep it maintainable.",
                    effective_cyclomatic, cognitive
                ),
                implementation_steps: vec![],
                related_items: vec![],
                steps: Some(steps),
                estimated_effort_hours: Some(0.5),
            }
        }

        RecommendationComplexityTier::Moderate => {
            // Near threshold - preventive refactoring
            let steps = vec![
                ActionStep {
                    description: "Add tests before refactoring (if coverage < 80%)".to_string(),
                    impact: "+safety net for refactoring".to_string(),
                    difficulty: Difficulty::Medium,
                    commands: vec![format!("cargo test {}::", metrics.name)],
                },
                ActionStep {
                    description: "Extract most complex section into focused function".to_string(),
                    impact: format!("-{} complexity", reduction),
                    difficulty: Difficulty::for_refactoring(effective_cyclomatic, cognitive),
                    commands: vec!["cargo clippy".to_string()],
                },
                ActionStep {
                    description: "Verify tests still pass".to_string(),
                    impact: "Confirmed refactoring safe".to_string(),
                    difficulty: Difficulty::Easy,
                    commands: vec!["cargo test --all".to_string()],
                },
            ];

            let estimated_effort = (effective_cyclomatic as f32 / 10.0) * 1.5;

            // Generate appropriate rationale and action based on adjusted vs raw complexity
            let (primary_action, rationale) = if effective_cyclomatic < 10 && cyclomatic >= 10 {
                // Adjusted complexity is below threshold but raw is not - focus on cognitive
                (
                    "Focus on reducing cognitive complexity through early returns and guard clauses"
                        .to_string(),
                    format!(
                        "Moderate cognitive complexity ({}). \
                         Cyclomatic complexity is manageable (adjusted: {}).",
                        cognitive, effective_cyclomatic
                    ),
                )
            } else {
                // Both need attention or adjusted is still above threshold
                (
                    if effective_cyclomatic >= 10 {
                        format!(
                            "Reduce complexity from {} to ~{}",
                            effective_cyclomatic, target
                        )
                    } else {
                        format!(
                            "Optional: Reduce complexity from {} to ~{} for future-proofing",
                            effective_cyclomatic, target
                        )
                    },
                    format!(
                        "Moderate complexity ({}/{}). {} threshold but maintainable. \
                         Preventive refactoring will ease future changes.",
                        effective_cyclomatic,
                        cognitive,
                        if effective_cyclomatic >= 10 {
                            "Approaching"
                        } else {
                            "Below"
                        }
                    ),
                )
            };

            ActionableRecommendation {
                primary_action,
                rationale,
                implementation_steps: vec![],
                related_items: vec![],
                steps: Some(steps),
                estimated_effort_hours: Some(estimated_effort),
            }
        }

        RecommendationComplexityTier::High | RecommendationComplexityTier::VeryHigh => {
            // High complexity - significant refactoring required
            let steps = vec![
                ActionStep {
                    description: "Add tests before refactoring (if coverage < 80%)".to_string(),
                    impact: "+safety net for refactoring".to_string(),
                    difficulty: Difficulty::Medium,
                    commands: vec![format!("cargo test {}::", metrics.name)],
                },
                ActionStep {
                    description: "Extract most complex section into focused function".to_string(),
                    impact: format!("-{} complexity", reduction),
                    difficulty: Difficulty::for_refactoring(effective_cyclomatic, cognitive),
                    commands: vec!["cargo clippy".to_string()],
                },
                ActionStep {
                    description: "Verify tests still pass".to_string(),
                    impact: "Confirmed refactoring safe".to_string(),
                    difficulty: Difficulty::Easy,
                    commands: vec!["cargo test --all".to_string()],
                },
            ];

            let estimated_effort = (effective_cyclomatic as f32 / 10.0) * 2.0;

            ActionableRecommendation {
                primary_action: format!(
                    "Reduce complexity from {} to ~{}",
                    effective_cyclomatic, target
                ),
                rationale: format!(
                    "High complexity ({}/{}). Exceeds maintainability thresholds. \
                     Refactoring required.",
                    effective_cyclomatic, cognitive
                ),
                implementation_steps: vec![],
                related_items: vec![],
                steps: Some(steps),
                estimated_effort_hours: Some(estimated_effort),
            }
        }
    }
}

/// Generate dead code recommendation
fn generate_dead_code_steps(
    visibility: &FunctionVisibility,
    cyclomatic: u32,
    cognitive: u32,
    _metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    let steps = match visibility {
        FunctionVisibility::Public => vec![
            ActionStep {
                description: "Verify function is not used by external crates".to_string(),
                impact: "Reduced public API surface".to_string(),
                difficulty: Difficulty::Medium,
                commands: vec!["cargo tree --all-features".to_string()],
            },
            ActionStep {
                description: "Remove the function if truly unused".to_string(),
                impact: format!("-{} lines, -{} complexity", cyclomatic * 3, cyclomatic),
                difficulty: Difficulty::Easy,
                commands: vec![],
            },
        ],
        FunctionVisibility::Private | FunctionVisibility::Crate => vec![
            ActionStep {
                description: "Confirm no callers in codebase".to_string(),
                impact: "Safe to remove".to_string(),
                difficulty: Difficulty::Easy,
                commands: vec!["rg \"function_name\"".to_string()],
            },
            ActionStep {
                description: "Remove the function".to_string(),
                impact: format!("-{} lines, -{} complexity", cyclomatic * 3, cyclomatic),
                difficulty: Difficulty::Easy,
                commands: vec![],
            },
        ],
    };

    let estimated_effort = 0.5; // 30 minutes for dead code removal

    ActionableRecommendation {
        primary_action: "Remove unused function".to_string(),
        rationale: format!(
            "Unused {:?} function with complexity {}/{}",
            visibility, cyclomatic, cognitive
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(estimated_effort),
    }
}

/// Estimate effort in hours based on metrics
fn estimate_effort(cyclomatic: u32, tests_needed: u32) -> f32 {
    // Base: 10-15 min per test
    let test_effort = tests_needed as f32 * 0.2;

    // Refactoring effort (if needed)
    let refactor_effort = if cyclomatic > 15 {
        (cyclomatic as f32 - 10.0) / 10.0 * 1.5 // ~1.5hr per 10 complexity reduction
    } else {
        0.0
    };

    // Round to nearest 0.5 hours
    ((test_effort + refactor_effort) * 2.0).round() / 2.0
}

/// Calculate number of functions to extract based on complexity
fn calculate_functions_to_extract(cyclomatic: u32, cognitive: u32) -> u32 {
    if cyclomatic > 30 || cognitive > 40 {
        4
    } else if cyclomatic > 20 || cognitive > 30 {
        3
    } else if cyclomatic > 15 || cognitive > 20 {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_metrics(cyclomatic: u32, cognitive: u32) -> FunctionMetrics {
        FunctionMetrics {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic,
            cognitive,
            nesting: 2,
            length: 50,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        }
    }

    #[test]
    fn test_max_5_steps_per_recommendation() {
        // Spec 206: Use nesting > 2 to avoid clean dispatcher (returns None)
        let mut metrics = create_test_metrics(20, 25);
        metrics.nesting = 3; // Avoid dispatcher pattern
        let rec = generate_concise_recommendation(
            &DebtType::ComplexityHotspot {
                cyclomatic: 20,
                cognitive: 25,
            },
            &metrics,
            FunctionRole::PureLogic,
            &None,
        )
        .expect("Test should generate recommendation");

        if let Some(steps) = &rec.steps {
            assert!(
                steps.len() <= 5,
                "Should have at most 5 steps, got {}",
                steps.len()
            );
        }
    }

    #[test]
    fn test_all_steps_have_impact() {
        let metrics = create_test_metrics(15, 18);
        let rec = generate_testing_gap_steps(0.5, 15, 18, &metrics, FunctionRole::PureLogic, &None);

        if let Some(steps) = &rec.steps {
            for step in steps {
                assert!(
                    !step.impact.is_empty(),
                    "Step '{}' missing impact",
                    step.description
                );
            }
        }
    }

    #[test]
    fn test_steps_ordered_by_impact() {
        let metrics = create_test_metrics(25, 30);
        let rec = generate_testing_gap_steps(0.3, 25, 30, &metrics, FunctionRole::PureLogic, &None);

        if let Some(steps) = &rec.steps {
            // First step should be testing (highest impact for testing gap)
            assert!(
                steps[0].description.contains("test"),
                "First step should address testing: {}",
                steps[0].description
            );
        }
    }

    #[test]
    fn test_effort_estimation_reasonable() {
        let metrics = create_test_metrics(15, 20);
        let rec = generate_testing_gap_steps(0.5, 15, 20, &metrics, FunctionRole::PureLogic, &None);

        if let Some(effort) = rec.estimated_effort_hours {
            assert!(effort > 0.0);
            assert!(effort < 10.0, "Effort seems too high: {}", effort);
        }
    }

    #[test]
    fn test_difficulty_matches_complexity() {
        // Simple case: Easy difficulty
        let simple_difficulty = Difficulty::for_testing(2, 5);
        assert_eq!(simple_difficulty, Difficulty::Easy);

        // Complex case: Hard difficulty
        let hard_difficulty = Difficulty::for_testing(15, 40);
        assert_eq!(hard_difficulty, Difficulty::Hard);
    }

    #[test]
    fn test_estimate_effort() {
        // Simple case: few tests, low complexity
        let effort1 = estimate_effort(10, 3);
        assert!((0.5..=2.0).contains(&effort1));

        // Complex case: many tests, high complexity
        let effort2 = estimate_effort(30, 10);
        assert!(effort2 > 2.0);
    }

    #[test]
    fn test_calculate_functions_to_extract() {
        assert_eq!(calculate_functions_to_extract(12, 15), 1);
        assert_eq!(calculate_functions_to_extract(18, 25), 2);
        assert_eq!(calculate_functions_to_extract(25, 35), 3);
        assert_eq!(calculate_functions_to_extract(35, 45), 4);
    }

    // Tests for spec 178: Fix Moderate Complexity Recommendation Logic

    #[test]
    fn test_low_complexity_gets_maintenance_recommendation() {
        let metrics = create_test_metrics(6, 8);
        let rec = generate_moderate_recommendation(6, 8, &metrics);

        assert!(
            rec.primary_action.contains("Maintain"),
            "Low complexity should get maintenance recommendation, got: {}",
            rec.primary_action
        );
        assert!(
            !rec.primary_action.contains("Reduce from 6 to ~10"),
            "Should NOT suggest increasing complexity"
        );
        assert!(rec.rationale.contains("low complexity"));
    }

    #[test]
    fn test_moderate_complexity_below_10_suggests_lower_target() {
        let metrics = create_test_metrics(9, 16);
        let rec = generate_moderate_recommendation(9, 16, &metrics);

        // Should suggest reducing to 5-6, not 10
        assert!(
            rec.primary_action.contains("Optional")
                || rec.primary_action.contains("to ~5")
                || rec.primary_action.contains("to ~6"),
            "Should suggest optional reduction to lower target, got: {}",
            rec.primary_action
        );
        assert!(
            !rec.primary_action.contains("to ~10"),
            "Should NOT suggest reducing to 10 when current is 9"
        );
    }

    #[test]
    fn test_moderate_at_threshold_suggests_reduction() {
        let metrics = create_test_metrics(12, 20);
        let rec = generate_moderate_recommendation(12, 20, &metrics);

        // Should suggest reducing to 8
        assert!(
            rec.primary_action.contains("to ~8"),
            "Should suggest reducing to 8, got: {}",
            rec.primary_action
        );
        assert!(
            !rec.primary_action.contains("to ~10")
                && !rec.primary_action.contains("to ~12")
                && !rec.primary_action.contains("to ~14"),
            "Target should be 8, not 10+ for moderate complexity at threshold"
        );
    }

    #[test]
    fn test_high_complexity_suggests_target_10() {
        let metrics = create_test_metrics(20, 30);
        let rec = generate_moderate_recommendation(20, 30, &metrics);

        // Should suggest reducing to 10
        assert!(
            rec.primary_action.contains("from 20 to ~10"),
            "High complexity should suggest target 10, got: {}",
            rec.primary_action
        );
    }

    #[test]
    fn test_impact_matches_target() {
        let metrics = create_test_metrics(12, 20);
        let rec = generate_moderate_recommendation(12, 20, &metrics);

        // If recommending "from 12 to ~8", impact should be "-4 complexity"
        if rec.primary_action.contains("to ~8") {
            assert!(rec.steps.is_some());
            let steps = rec.steps.unwrap();
            let extract_step = steps.iter().find(|s| s.description.contains("Extract"));
            assert!(extract_step.is_some(), "Should have extract step");
            assert!(
                extract_step.unwrap().impact.contains("-4")
                    || extract_step.unwrap().impact.contains("4"),
                "Impact should match reduction, got: {}",
                extract_step.unwrap().impact
            );
        }
    }

    #[test]
    fn test_complexity_tier_classification() {
        // Low tier
        assert_eq!(
            classify_complexity_tier(5, 10),
            RecommendationComplexityTier::Low
        );
        assert_eq!(
            classify_complexity_tier(7, 14),
            RecommendationComplexityTier::Low
        );

        // Moderate tier
        assert_eq!(
            classify_complexity_tier(8, 15),
            RecommendationComplexityTier::Moderate
        );
        assert_eq!(
            classify_complexity_tier(12, 20),
            RecommendationComplexityTier::Moderate
        );
        assert_eq!(
            classify_complexity_tier(14, 24),
            RecommendationComplexityTier::Moderate
        );

        // High tier
        assert_eq!(
            classify_complexity_tier(15, 25),
            RecommendationComplexityTier::High
        );
        assert_eq!(
            classify_complexity_tier(20, 30),
            RecommendationComplexityTier::High
        );

        // Very high tier
        assert_eq!(
            classify_complexity_tier(25, 40),
            RecommendationComplexityTier::VeryHigh
        );
        assert_eq!(
            classify_complexity_tier(40, 60),
            RecommendationComplexityTier::VeryHigh
        );
    }

    #[test]
    fn test_target_complexity_calculation() {
        // Low tier targets
        assert_eq!(
            calculate_target_complexity(5, RecommendationComplexityTier::Low),
            4
        );
        assert_eq!(
            calculate_target_complexity(3, RecommendationComplexityTier::Low),
            3
        );

        // Moderate tier targets
        assert_eq!(
            calculate_target_complexity(8, RecommendationComplexityTier::Moderate),
            5
        );
        assert_eq!(
            calculate_target_complexity(9, RecommendationComplexityTier::Moderate),
            6
        );
        assert_eq!(
            calculate_target_complexity(10, RecommendationComplexityTier::Moderate),
            8
        );
        assert_eq!(
            calculate_target_complexity(12, RecommendationComplexityTier::Moderate),
            8
        );

        // High tier targets
        assert_eq!(
            calculate_target_complexity(20, RecommendationComplexityTier::High),
            10
        );

        // Very high tier targets
        assert_eq!(
            calculate_target_complexity(30, RecommendationComplexityTier::VeryHigh),
            15
        );
        assert_eq!(
            calculate_target_complexity(50, RecommendationComplexityTier::VeryHigh),
            15
        );
    }

    #[test]
    fn test_target_always_less_than_current() {
        // Test various complexity levels to ensure target is always less than current
        for cyclo in 5..50 {
            for cognitive in 10..60 {
                let tier = classify_complexity_tier(cyclo, cognitive);
                let target = calculate_target_complexity(cyclo, tier);
                assert!(
                    target <= cyclo,
                    "Target ({}) should be <= current ({}) for tier {:?}",
                    target,
                    cyclo,
                    tier
                );
            }
        }
    }

    // Regression tests for other complexity patterns (spec 178)

    #[test]
    fn test_high_nesting_pattern_recommendation() {
        let mut metrics = create_test_metrics(10, 35);
        metrics.nesting = 5; // High nesting

        let rec = generate_complexity_steps(10, 35, &metrics)
            .expect("Test should generate recommendation for high nesting");

        assert!(
            rec.primary_action.contains("nesting"),
            "High nesting pattern should mention nesting, got: {}",
            rec.primary_action
        );
        assert!(rec.steps.is_some());
        let steps = rec.steps.unwrap();

        // Should have early returns step
        let has_early_returns = steps.iter().any(|s| s.description.contains("early return"));
        assert!(
            has_early_returns,
            "Should recommend early returns for high nesting"
        );
    }

    #[test]
    fn test_high_branching_pattern_recommendation() {
        // Spec 206: nesting > 2 needed to avoid Dispatcher classification
        let mut metrics = create_test_metrics(25, 45);
        metrics.nesting = 3; // Moderate nesting, high branching (ratio 1.8, but nesting > 2)

        let rec = generate_complexity_steps(25, 45, &metrics)
            .expect("Test should generate recommendation for high branching");

        assert!(
            rec.primary_action.contains("Split") || rec.primary_action.contains("function"),
            "High branching pattern should suggest splitting, got: {}",
            rec.primary_action
        );
        assert!(rec.steps.is_some());
    }

    #[test]
    fn test_mixed_complexity_pattern_recommendation() {
        // For MixedComplexity: cyclo >= 12, cognitive >= 40, ratio 2.5-3.5
        // Using cyclo=15, cognitive=45 gives ratio=3.0 (in range)
        let mut metrics = create_test_metrics(15, 45);
        metrics.nesting = 4; // Both high nesting and high branching

        let rec = generate_complexity_steps(15, 45, &metrics)
            .expect("Test should generate recommendation for mixed complexity");

        assert!(
            rec.primary_action.contains("FIRST") || rec.primary_action.contains("phase"),
            "Mixed complexity should suggest phased approach, got: {}",
            rec.primary_action
        );
        assert!(rec.steps.is_some());
        let steps = rec.steps.unwrap();

        // Should have Phase 1 and Phase 2
        let has_phases = steps.iter().any(|s| s.description.contains("Phase"));
        assert!(
            has_phases,
            "Should have phased approach for mixed complexity"
        );
    }

    #[test]
    fn test_chaotic_structure_pattern_recommendation() {
        use crate::complexity::entropy_core::EntropyScore;

        // Spec 206: need nesting > 2 or ratio >= 2.0 to avoid Dispatcher classification
        let mut metrics = create_test_metrics(20, 30);
        metrics.nesting = 3; // Avoid dispatcher pattern (nesting > 2)
        metrics.entropy_score = Some(EntropyScore {
            token_entropy: 0.45, // High entropy for chaotic detection
            pattern_repetition: 0.2,
            branch_similarity: 0.3,
            effective_complexity: 15.0,
            unique_variables: 10,
            max_nesting: 3,
            dampening_applied: 0.0,
        });

        let rec = generate_complexity_steps(20, 30, &metrics)
            .expect("Test should generate recommendation for chaotic structure");

        assert!(
            rec.primary_action.contains("Standardize")
                || rec.primary_action.contains("control flow"),
            "Chaotic structure should suggest standardization, got: {}",
            rec.primary_action
        );
        assert!(rec.steps.is_some());
        let steps = rec.steps.unwrap();

        // Should mention error handling or state transitions
        let has_standardization = steps
            .iter()
            .any(|s| s.description.contains("error handling") || s.description.contains("state"));
        assert!(
            has_standardization,
            "Should recommend standardization for chaotic structure"
        );
    }

    #[test]
    fn test_pattern_detection_still_works() {
        // Verify that pattern detection correctly identifies different patterns

        // High nesting pattern
        let mut nesting_metrics = create_test_metrics(10, 35);
        nesting_metrics.nesting = 5;
        let nesting_rec = generate_complexity_steps(10, 35, &nesting_metrics)
            .expect("Test should generate recommendation for nesting");
        assert!(nesting_rec.primary_action.contains("nesting"));

        // High branching pattern (spec 206: need nesting > 2 to avoid Dispatcher)
        let mut branching_metrics = create_test_metrics(25, 45);
        branching_metrics.nesting = 3; // Avoid dispatcher pattern
        let branching_rec = generate_complexity_steps(25, 45, &branching_metrics)
            .expect("Test should generate recommendation for branching");
        assert!(
            branching_rec.primary_action.contains("Split")
                || branching_rec.primary_action.contains("function")
        );

        // Moderate complexity pattern (spec 206: cyclo < 10 avoids Dispatcher threshold)
        let moderate_metrics = create_test_metrics(9, 18);
        let moderate_rec = generate_complexity_steps(9, 18, &moderate_metrics)
            .expect("Test should generate recommendation for moderate");
        assert!(
            moderate_rec.primary_action.contains("Reduce")
                || moderate_rec.primary_action.contains("Optional")
                || moderate_rec.primary_action.contains("Maintain")
        );
    }

    // Tests for spec 201: Pattern detection should not filter out debt items
    // Complexity thresholds already determined an item is debt - pattern detection
    // should only tailor the recommendation, not gate whether it appears

    #[test]
    fn test_clean_dispatcher_returns_none() {
        // Spec 206: Clean dispatcher with no inline logic and flat nesting returns None
        // (this is intentional architecture, not debt)
        let mut metrics = create_test_metrics(26, 40);
        metrics.nesting = 1; // Flat structure

        let result = generate_dispatcher_recommendation(
            26,   // branch_count
            1.54, // cognitive_ratio
            0,    // inline_logic_branches (clean dispatcher)
            26,   // cyclomatic
            40,   // cognitive
            &metrics,
        );

        assert!(
            result.is_none(),
            "Clean dispatcher with flat nesting should return None (no refactoring needed)"
        );
    }

    #[test]
    fn test_clean_dispatcher_high_nesting_falls_back() {
        // Clean dispatcher with higher nesting falls back to generic recommendation
        let mut metrics = create_test_metrics(20, 25);
        metrics.nesting = 3; // Higher nesting (not typical dispatcher)

        let result = generate_dispatcher_recommendation(
            20,   // branch_count
            1.25, // cognitive_ratio
            0,    // inline_logic_branches (clean dispatcher)
            20,   // cyclomatic
            25,   // cognitive
            &metrics,
        );

        assert!(
            result.is_some(),
            "Clean dispatcher with higher nesting should return fallback recommendation"
        );
    }

    #[test]
    fn test_dispatcher_with_inline_logic_returns_some() {
        // Dispatcher with inline logic should return a recommendation
        let result = generate_dispatcher_recommendation(
            10,  // branch_count
            0.5, // cognitive_ratio
            3,   // inline_logic_branches (has inline logic)
            15,  // cyclomatic
            10,  // cognitive
            &create_test_metrics(15, 10),
        );

        assert!(
            result.is_some(),
            "Dispatcher with inline logic should return a recommendation"
        );

        let rec = result.unwrap();
        assert!(
            rec.primary_action.contains("Extract inline logic"),
            "Should recommend extracting inline logic, got: {}",
            rec.primary_action
        );
    }

    // Spec 201/206: Pattern detection tailors recommendations.
    // Spec 206 exception: Clean dispatchers (flat structure, no inline logic) return None
    // because they represent intentional architecture, not debt.

    #[test]
    fn complexity_hotspot_always_returns_recommendation() {
        // Test that non-dispatcher patterns always return recommendations
        // Spec 206: cyclo >= 10, nesting <= 2, ratio < 2.0 => Dispatcher (may return None)
        // Use nesting > 2 or cyclo < 10 to avoid dispatcher classification
        let test_cases = [
            (5, 8, 2, "low complexity (below dispatcher threshold)"),
            (9, 16, 2, "moderate complexity (below dispatcher threshold)"),
            (15, 20, 3, "high complexity with nesting > 2"),
            (48, 303, 3, "extreme complexity with nesting > 2"),
        ];

        for (cyclomatic, cognitive, nesting, description) in test_cases {
            let mut metrics = create_test_metrics(cyclomatic, cognitive);
            metrics.nesting = nesting;
            let result = generate_concise_recommendation(
                &DebtType::ComplexityHotspot {
                    cyclomatic,
                    cognitive,
                },
                &metrics,
                FunctionRole::PureLogic,
                &None,
            );

            assert!(
                result.is_some(),
                "ComplexityHotspot should return Some for {} (cyclomatic={}, cognitive={}, nesting={})",
                description,
                cyclomatic,
                cognitive,
                nesting
            );
        }
    }

    #[test]
    fn clean_dispatcher_returns_none_for_complexity_hotspot() {
        // Spec 206: Clean dispatcher (flat structure, no inline logic) returns None
        // This is intentional architecture, not debt
        // Clean dispatcher: cognitive <= cyclomatic * 1.5 (no inline logic)
        // cyclomatic=26, expected_max=39, use cognitive=38 to be under
        let mut metrics = create_test_metrics(26, 38);
        metrics.nesting = 1; // Flat structure

        let result = generate_concise_recommendation(
            &DebtType::ComplexityHotspot {
                cyclomatic: 26,
                cognitive: 38,
            },
            &metrics,
            FunctionRole::PureLogic,
            &None,
        );

        assert!(
            result.is_none(),
            "Clean dispatcher should return None (not debt, intentional architecture)"
        );
    }

    #[test]
    fn complexity_hotspot_without_language_signals_returns_recommendation() {
        // Edge case: function metrics without Rust-specific signals
        // Spec 206: cyclo=12, nesting=2 matches dispatcher, but without inline logic = clean
        // Use cyclo < 10 to avoid dispatcher classification
        let mut metrics = create_test_metrics(9, 18);
        metrics.language_specific = None; // No language-specific data

        let result = generate_concise_recommendation(
            &DebtType::ComplexityHotspot {
                cyclomatic: 9,
                cognitive: 18,
            },
            &metrics,
            FunctionRole::Orchestrator,
            &None,
        );

        assert!(
            result.is_some(),
            "Should return fallback recommendation when language_specific is None"
        );
    }
}
