# Example Outputs

This page demonstrates realistic examples of debtmap's terminal and JSON output formats using the unified format (spec 108).

## High Complexity Function (Needs Refactoring)

**Terminal Output:**
```
#1 SCORE: 9.2 [CRITICAL]
├─ COMPLEXITY: ./src/payments/processor.rs:145 process_transaction()
├─ ACTION: Refactor into 4 smaller functions
├─ IMPACT: Reduce complexity from 25 to 8, improve testability
├─ COMPLEXITY: cyclomatic=25, branches=25, cognitive=38, nesting=5, lines=120
├─ DEPENDENCIES: 3 upstream, 8 downstream
└─ WHY: Exceeds all complexity thresholds, difficult to test and maintain
```

**JSON Output (Unified Format):**
```json
{
  "type": "Function",
  "score": 92.5,
  "category": "CodeQuality",
  "priority": "critical",
  "location": {
    "file": "src/payments/processor.rs",
    "line": 145,
    "function": "process_transaction"
  },
  "metrics": {
    "cyclomatic_complexity": 25,
    "cognitive_complexity": 38,
    "length": 120,
    "nesting_depth": 5,
    "coverage": 0.15,
    "uncovered_lines": [145, 147, 152, 158, 165, 172, 180, 185]
  },
  "debt_type": {
    "ComplexityHotspot": {
      "cyclomatic": 25,
      "cognitive": 38,
      "adjusted_cyclomatic": null
    }
  },
  "function_role": "Orchestrator",
  "purity_analysis": {
    "is_pure": false,
    "confidence": 0.15,
    "side_effects": ["mutates_state", "io_operations", "database_access"]
  },
  "dependencies": {
    "upstream_count": 3,
    "downstream_count": 8,
    "upstream_callers": [
      "handle_payment",
      "handle_subscription",
      "handle_refund"
    ],
    "downstream_callees": [
      "validate",
      "calculate_fees",
      "record_transaction",
      "send_receipt",
      "update_balance",
      "log_transaction",
      "check_fraud",
      "notify_user"
    ]
  },
  "recommendation": {
    "action": "Refactor into 4 smaller, focused functions",
    "implementation_steps": [
      "Extract validation logic into validate_payment_request",
      "Create calculate_payment_totals for fee calculation",
      "Move side effects to separate transaction recorder",
      "Keep process_transaction as thin orchestrator"
    ]
  },
  "impact": {
    "coverage_improvement": 0.55,
    "complexity_reduction": 68.0,
    "risk_reduction": 7.8
  },
  "scoring_details": {
    "coverage_score": 45.0,
    "complexity_score": 38.5,
    "dependency_score": 9.0,
    "base_score": 92.5,
    "role_multiplier": 1.0,
    "final_score": 92.5
  }
}
```

**Source**: Structure from `src/output/unified.rs:FunctionDebtItemOutput` (lines 158-183)

**Key Fields Explained:**
- `type`: Always `"Function"` for function-level debt items
- `score`: Unified debt score (same path for File and Function items)
- `category`: One of `CodeQuality`, `Architecture`, `Testing`, `Performance`
- `priority`: Derived from score (`critical` >= 100, `high` >= 50, `medium` >= 20, `low` < 20)
- `location`: Unified location structure with file, line, and function name
- `function_role`: Classification from `FunctionRole` enum (see below)
- `debt_type`: Tagged enum with variant-specific data

## Function Role Classification

The `function_role` field helps prioritize testing and refactoring efforts based on the function's architectural purpose.

**Source**: `src/priority/semantic_classifier/mod.rs:25-33`

```json
{
  "function_role": "PureLogic"
}
```

**Available Roles:**
- `PureLogic` - Business logic, high test priority
- `Orchestrator` - Coordinates other functions (like the example above)
- `IOWrapper` - Thin I/O layer, lower test priority
- `EntryPoint` - Main entry points (main, CLI handlers)
- `PatternMatch` - Pattern matching function (often low complexity)
- `Debug` - Debug/diagnostic functions (low test priority)
- `Unknown` - Cannot classify automatically

## File-Level Debt (God Object)

**Terminal Output:**
```
#2 SCORE: 8.7 [HIGH]
├─ GOD OBJECT: ./src/services/user_manager.rs
├─ ACTION: Split into 4 focused modules
├─ METRICS: 1250 lines, 45 functions, avg complexity 12.3
├─ INDICATORS: High responsibility count (8), excessive dependencies
└─ WHY: File has too many responsibilities, difficult to maintain
```

**JSON Output (Unified Format):**
```json
{
  "type": "File",
  "score": 87.0,
  "category": "Architecture",
  "priority": "high",
  "location": {
    "file": "src/services/user_manager.rs"
  },
  "metrics": {
    "lines": 1250,
    "functions": 45,
    "classes": 3,
    "avg_complexity": 12.3,
    "max_complexity": 28,
    "total_complexity": 554,
    "coverage": 0.62,
    "uncovered_lines": 125
  },
  "god_object_indicators": {
    "responsibility_count": 8,
    "data_class_count": 12,
    "method_groups": [
      "authentication",
      "authorization",
      "profile_management",
      "session_handling",
      "notification_preferences",
      "audit_logging",
      "password_management",
      "role_management"
    ],
    "coupling_score": 0.78,
    "cohesion_score": 0.34
  },
  "recommendation": {
    "action": "Split into focused modules by responsibility",
    "implementation_steps": [
      "Extract authentication into auth_service.rs",
      "Move authorization to permission_service.rs",
      "Create profile_service.rs for user data management",
      "Separate audit concerns into audit_logger.rs"
    ]
  },
  "impact": {
    "complexity_reduction": 45.0,
    "maintainability_improvement": 0.68,
    "test_effort": 8.5
  }
}
```

**Source**: Structure from `src/output/unified.rs:FileDebtItemOutput` (lines 110-123) and `src/priority/file_metrics.rs:GodObjectIndicators`

## Test Gap (Needs Testing)

**Terminal Output:**
```
#3 SCORE: 8.9 [CRITICAL]
├─ TEST GAP: ./src/analyzers/rust_call_graph.rs:38 add_function_to_graph()
├─ ACTION: Add 6 unit tests for full coverage
├─ IMPACT: Full test coverage, -3.7 risk reduction
├─ COMPLEXITY: cyclomatic=6, branches=6, cognitive=8, nesting=2, lines=32
├─ DEPENDENCIES: 0 upstream, 11 downstream
├─ TEST EFFORT: Simple (2-3 hours)
└─ WHY: Business logic with 0% coverage, manageable complexity (cyclo=6, cog=8)
    High impact - 11 functions depend on this
```

**JSON Output (Unified Format):**
```json
{
  "type": "Function",
  "score": 89.0,
  "category": "Testing",
  "priority": "critical",
  "location": {
    "file": "src/analyzers/rust_call_graph.rs",
    "line": 38,
    "function": "add_function_to_graph"
  },
  "metrics": {
    "cyclomatic_complexity": 6,
    "cognitive_complexity": 8,
    "length": 32,
    "nesting_depth": 2,
    "coverage": 0.0,
    "uncovered_lines": [38, 39, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66]
  },
  "debt_type": {
    "TestingGap": {
      "coverage": 0.0,
      "cyclomatic": 6,
      "cognitive": 8
    }
  },
  "function_role": "PureLogic",
  "purity_analysis": {
    "is_pure": false,
    "confidence": 0.65
  },
  "dependencies": {
    "upstream_count": 0,
    "downstream_count": 11,
    "downstream_callees": [
      "get_function_name",
      "extract_parameters",
      "parse_return_type",
      "add_to_registry",
      "update_call_sites",
      "resolve_types",
      "track_visibility",
      "record_location",
      "increment_counter",
      "validate_signature",
      "log_registration"
    ]
  },
  "recommendation": {
    "action": "Add unit tests for core business logic",
    "implementation_steps": [
      "Test happy path with valid function definition",
      "Test error cases: null input, invalid syntax",
      "Test edge cases: complex generics, lifetimes",
      "Test integration with registry updates",
      "Verify correct handling of visibility modifiers",
      "Test type resolution edge cases"
    ]
  },
  "impact": {
    "coverage_improvement": 1.0,
    "complexity_reduction": 0.0,
    "risk_reduction": 3.7
  }
}
```

**Source**: Structure from `src/output/unified.rs:FunctionDebtItemOutput` with `debt_type` from `src/priority/mod.rs:158-171`

## Entropy-Dampened Validation Function

This example shows how debtmap's entropy analysis reduces false positives for repetitive code patterns.

**Terminal Output:**
```
Function: validate_config
  File: src/config/validator.rs:23
  Cyclomatic: 20 → Effective: 7 (65% dampened)
  Risk: LOW

  Entropy Analysis:
    ├─ Token Entropy: 0.28 (low variety - repetitive patterns)
    ├─ Pattern Repetition: 0.88 (high similarity between checks)
    ├─ Branch Similarity: 0.91 (consistent validation structure)
    └─ Reasoning: Complexity reduced by 65% due to pattern-based code

  This appears complex but is actually a repetitive validation pattern.
  Lower priority for refactoring.
```

**JSON Output (Unified Format):**
```json
{
  "type": "Function",
  "score": 15.2,
  "category": "CodeQuality",
  "priority": "low",
  "location": {
    "file": "src/config/validator.rs",
    "line": 23,
    "function": "validate_config"
  },
  "metrics": {
    "cyclomatic_complexity": 20,
    "cognitive_complexity": 18,
    "length": 85,
    "nesting_depth": 3,
    "coverage": 0.95,
    "entropy_score": 0.28
  },
  "debt_type": {
    "ComplexityHotspot": {
      "cyclomatic": 20,
      "cognitive": 18,
      "adjusted_cyclomatic": 7
    }
  },
  "adjusted_complexity": {
    "dampened_cyclomatic": 7.0,
    "dampening_factor": 0.65
  },
  "function_role": "PatternMatch",
  "recommendation": {
    "action": "Low priority - repetitive validation pattern"
  },
  "impact": {
    "coverage_improvement": 0.05,
    "complexity_reduction": 0.0,
    "risk_reduction": 0.8
  },
  "scoring_details": {
    "coverage_score": 2.5,
    "complexity_score": 7.0,
    "dependency_score": 0.0,
    "base_score": 43.5,
    "entropy_dampening": 0.65,
    "role_multiplier": 0.35,
    "final_score": 15.2
  }
}
```

**Source**: `adjusted_complexity` from `src/output/unified.rs:186-190`, entropy dampening spec 182

**Key Points:**
- `adjusted_cyclomatic`: Entropy-dampened complexity value (7 vs original 20)
- `dampening_factor`: Amount of reduction applied (0.65 = 65% reduction)
- `entropy_score`: Low value (0.28) indicates repetitive patterns
- Score reduced from 43.5 to 15.2 due to entropy analysis

## Pattern Detection Example

When debtmap detects a specific complexity pattern, it includes pattern metadata.

**JSON Output:**
```json
{
  "type": "Function",
  "score": 65.0,
  "category": "CodeQuality",
  "priority": "high",
  "location": {
    "file": "src/state/workflow_executor.rs",
    "line": 78,
    "function": "execute_transition"
  },
  "pattern_type": "state_machine",
  "pattern_confidence": 0.87,
  "pattern_details": {
    "state_count": 12,
    "transition_count": 34,
    "branch_entropy": 0.82,
    "state_cohesion": 0.91
  },
  "complexity_pattern": "State machine with 12 states, high cohesion"
}
```

**Source**: `pattern_type` and `pattern_confidence` from `src/output/unified.rs:178-182`

**Available Pattern Types:**
- `state_machine` - State transition logic
- `coordinator` - Function orchestrating multiple operations
- Pattern detection threshold: 0.7 confidence (from `src/io/writers/pattern_display.rs:PATTERN_CONFIDENCE_THRESHOLD`)

## Test File Detection

Debtmap automatically labels test files using the `file_context_label` field (spec 166).

**JSON Output:**
```json
{
  "type": "Function",
  "location": {
    "file": "tests/integration/payment_test.rs",
    "line": 45,
    "function": "test_payment_processing",
    "file_context_label": "TEST FILE"
  }
}
```

**Labels:**
- `"TEST FILE"` - File is definitively a test file
- `"PROBABLE TEST"` - File likely contains tests but not confirmed

**Source**: `file_context_label` from `src/output/unified.rs:106`

## Summary Statistics

The unified format includes summary statistics at the top level.

**JSON Output:**
```json
{
  "format_version": "1.0.0",
  "metadata": {
    "debtmap_version": "0.5.0",
    "generated_at": "2025-12-04T22:15:00Z",
    "project_root": "/home/user/myproject",
    "analysis_type": "full"
  },
  "summary": {
    "total_items": 127,
    "total_debt_score": 2845.6,
    "debt_density": 0.18,
    "total_loc": 15823,
    "by_type": {
      "File": 8,
      "Function": 119
    },
    "by_category": {
      "CodeQuality": 67,
      "Architecture": 12,
      "Testing": 42,
      "Performance": 6
    },
    "score_distribution": {
      "critical": 15,
      "high": 34,
      "medium": 58,
      "low": 20
    }
  },
  "items": []
}
```

**Source**: `UnifiedOutput` structure from `src/output/unified.rs:18-24` and `DebtSummary` from lines 36-45

**Key Summary Fields:**
- `debt_density`: Total debt score per 1000 lines of code
- `by_type`: Count of File vs Function debt items
- `by_category`: Count by debt category
- `score_distribution`: Count of items by priority level

## Before/After Refactoring Comparison

**Before:**
```
Function: process_order
  Cyclomatic: 22
  Cognitive: 35
  Coverage: 15%
  Risk Score: 52.3 (CRITICAL)
  Debt Score: 50 (Critical Complexity)
```

**After:**
```
Function: process_order (refactored)
  Cyclomatic: 5
  Cognitive: 6
  Coverage: 92%
  Risk Score: 2.1 (LOW)
  Debt Score: 0 (no debt)

Extracted functions:
  - validate_order (Cyclomatic: 4, Coverage: 100%)
  - calculate_totals (Cyclomatic: 3, Coverage: 95%)
  - apply_discounts (Cyclomatic: 6, Coverage: 88%)
  - finalize_order (Cyclomatic: 4, Coverage: 90%)

Impact:
  ✓ Complexity reduced by 77%
  ✓ Coverage improved by 513%
  ✓ Risk reduced by 96%
  ✓ Created 4 focused, testable functions
```

## Well-Tested Complex Function (Good Example)

Not all complexity is bad. This example shows a legitimately complex function with excellent test coverage.

**Terminal Output:**
```
Function: calculate_tax (WELL TESTED - Good Example!)
  File: src/tax/calculator.rs:78
  Complexity: Cyclomatic=18, Cognitive=22
  Coverage: 98%
  Risk: LOW

  Why this is good:
  - High complexity is necessary (tax rules are complex)
  - Thoroughly tested with 45 test cases
  - Clear documentation of edge cases
  - Good example to follow for other complex logic
```

## Next Steps

- **[Output Formats](../output-formats.md)** - Complete JSON schema and format documentation
- **[Configuration](../configuration.md)** - Customize thresholds and analysis behavior
- **[Advanced Features](../analysis-guide/advanced-features.md)** - Purity analysis, entropy dampening, pattern detection

For questions or issues, visit [GitHub Issues](https://github.com/iepathos/debtmap/issues).
