# Example Outputs

## Example Outputs

### High Complexity Function (Needs Refactoring)

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

**JSON Output:**
```json
{
  "id": "complexity_src_payments_processor_rs_145",
  "debt_type": "Complexity",
  "priority": "Critical",
  "file": "src/payments/processor.rs",
  "line": 145,
  "message": "Function exceeds complexity threshold",
  "context": "Cyclomatic: 25, Cognitive: 38, Nesting: 5",
  "function_metrics": {
    "name": "process_transaction",
    "cyclomatic": 25,
    "cognitive": 38,
    "nesting": 5,
    "length": 120,
    "is_pure": false,
    "purity_confidence": 0.15,
    "upstream_callers": ["handle_payment", "handle_subscription", "handle_refund"],
    "downstream_callees": ["validate", "calculate_fees", "record_transaction", "send_receipt", "update_balance", "log_transaction", "check_fraud", "notify_user"]
  }
}
```

### Well-Tested Complex Function (Good Example)

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

### Test Gap (Needs Testing)

**Terminal Output:**
```
#2 SCORE: 8.9 [CRITICAL]
├─ TEST GAP: ./src/analyzers/rust_call_graph.rs:38 add_function_to_graph()
├─ ACTION: Add 6 unit tests for full coverage
├─ IMPACT: Full test coverage, -3.7 risk reduction
├─ COMPLEXITY: cyclomatic=6, branches=6, cognitive=8, nesting=2, lines=32
├─ DEPENDENCIES: 0 upstream, 11 downstream
├─ TEST EFFORT: Simple (2-3 hours)
└─ WHY: Business logic with 0% coverage, manageable complexity (cyclo=6, cog=8)
    High impact - 11 functions depend on this
```

**JSON Output:**
```json
{
  "function": "add_function_to_graph",
  "file": "src/analyzers/rust_call_graph.rs",
  "line": 38,
  "current_risk": 8.9,
  "potential_risk_reduction": 3.7,
  "recommendation": {
    "action": "Add unit tests",
    "details": "Add 6 unit tests for full coverage",
    "effort_estimate": "2-3 hours"
  },
  "test_effort": {
    "estimated_difficulty": "Simple",
    "cognitive_load": 8,
    "branch_count": 6,
    "recommended_test_cases": 6
  },
  "complexity": {
    "cyclomatic": 6,
    "cognitive": 8,
    "nesting": 2,
    "length": 32
  },
  "dependencies": {
    "upstream_callers": [],
    "downstream_callees": [
      "get_function_name", "extract_parameters", "parse_return_type",
      "add_to_registry", "update_call_sites", "resolve_types",
      "track_visibility", "record_location", "increment_counter",
      "validate_signature", "log_registration"
    ]
  },
  "roi": 4.5
}
```

### Entropy-Dampened Validation Function

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

### Before/After Refactoring Comparison

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

## Next Steps

- **[Output Formats](./output-formats.md)** - Detailed JSON schema and integration patterns
- **[Configuration](./configuration.md)** - Customize thresholds and analysis behavior

For questions or issues, visit [GitHub Issues](https://github.com/iepathos/debtmap/issues).
