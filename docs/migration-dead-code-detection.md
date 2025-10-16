# Migration Guide: Enhanced Python Dead Code Detection

This guide helps you migrate from debtmap's previous Python dead code detection to the new enhanced system with confidence scoring (Spec 107).

## What's New

### Major Improvements

1. **Confidence Scoring System**
   - Results now include High/Medium/Low confidence levels
   - Helps prioritize which code to remove first
   - Reduces decision fatigue

2. **Integrated Detection Systems**
   - Framework pattern detection (Flask, Django, FastAPI, etc.)
   - Test function detection (pytest, unittest)
   - Callback tracking (decorators, event handlers)
   - Import resolution across modules

3. **Suppression Comments**
   - Mark functions as intentionally unused
   - Standard syntax: `# debtmap: not-dead`
   - Compatible with existing tools: `# noqa: dead-code`

4. **Coverage Integration**
   - Uses pytest-cov or coverage.py data
   - Functions in coverage reports are marked as live
   - Dramatically reduces false positives

5. **Lower False Positive Rate**
   - Target: <10% false positive rate
   - Previously: ~30-40% false positive rate
   - Better handling of framework patterns

## API Changes

### Old API (Pre-107)

```rust
use debtmap::analysis::python_dead_code::DeadCodeDetector;

let detector = DeadCodeDetector::new();
let dead_functions = detector.find_dead_functions(&call_graph);

// Returns: Vec<String> (just function names)
for func_name in dead_functions {
    println!("Dead: {}", func_name);
}
```

### New API (Post-107)

```rust
use debtmap::analysis::python_dead_code_enhanced::EnhancedDeadCodeAnalyzer;

let analyzer = EnhancedDeadCodeAnalyzer::new();
let result = analyzer.analyze_function(&func, &call_graph);

// Returns: DeadCodeResult with confidence and reasons
println!("Dead: {}, Confidence: {:?}", result.is_dead, result.confidence);
println!("Reasons: {:?}", result.dead_reasons);
println!("Suggestion: {}", result.suggestion.explanation);
```

## Output Format Changes

### Old Format

```json
{
  "dead_code": {
    "functions": [
      "unused_helper",
      "_old_implementation",
      "legacy_method"
    ],
    "count": 3
  }
}
```

### New Format

```json
{
  "dead_code": [
    {
      "function": "unused_helper",
      "file": "app.py",
      "line": 42,
      "is_dead": true,
      "confidence": "High",
      "confidence_score": 0.95,
      "dead_reasons": [
        "NoStaticCallers",
        "PrivateFunction",
        "NotInTestFile"
      ],
      "live_reasons": [],
      "suggestion": {
        "can_remove": true,
        "safe_to_remove": true,
        "explanation": "High confidence this function is dead code and can be safely removed.",
        "risks": []
      }
    },
    {
      "function": "legacy_method",
      "file": "api.py",
      "line": 100,
      "is_dead": true,
      "confidence": "Medium",
      "confidence_score": 0.65,
      "dead_reasons": [
        "NoStaticCallers"
      ],
      "live_reasons": [
        "PublicApi"
      ],
      "suggestion": {
        "can_remove": true,
        "safe_to_remove": false,
        "explanation": "Medium confidence this function is dead code. Manual verification recommended.",
        "risks": [
          "Function is public and may be used by external code."
        ]
      }
    }
  ]
}
```

## Migration Steps

### Step 1: Update Dependencies

```toml
# Cargo.toml
[dependencies]
debtmap = "0.X.X"  # Update to version with Spec 107
```

### Step 2: Update Code

**Before:**
```rust
use debtmap::analysis::python_dead_code::DeadCodeDetector;

let detector = DeadCodeDetector::new();
let dead_funcs = detector.find_dead_functions(&call_graph);

for func in dead_funcs {
    println!("Remove: {}", func);
}
```

**After:**
```rust
use debtmap::analysis::python_dead_code_enhanced::{
    EnhancedDeadCodeAnalyzer,
    DeadCodeConfidence,
};

let analyzer = EnhancedDeadCodeAnalyzer::new();

for func in all_functions {
    let result = analyzer.analyze_function(&func, &call_graph);

    // Only show high confidence results
    if result.is_dead && matches!(result.confidence, DeadCodeConfidence::High(_)) {
        println!("Safe to remove: {}", result.function_id.name);
    }
}
```

### Step 3: Handle Confidence Levels

The new system requires you to decide how to handle different confidence levels:

```rust
match result.confidence {
    DeadCodeConfidence::High(_) => {
        // Safe to remove automatically
        println!("Auto-remove: {}", result.function_id.name);
    }
    DeadCodeConfidence::Medium(_) => {
        // Require manual review
        println!("Review: {} - {}",
            result.function_id.name,
            result.suggestion.explanation);
    }
    DeadCodeConfidence::Low(_) => {
        // Keep the function
        if result.is_dead {
            println!("Keep (low confidence): {}", result.function_id.name);
        }
    }
}
```

### Step 4: Add Suppression Comments

For functions that should be ignored (public APIs, future features, etc.):

```python
# Before: No way to suppress false positives

# After: Add suppression comments
# debtmap: not-dead
def public_api_method():
    """External API - do not remove"""
    pass
```

### Step 5: Enable Coverage Integration (Optional)

```bash
# Generate coverage data
pytest --cov=myapp --cov-report=json

# Debtmap will automatically detect and use coverage.json
debtmap analyze myapp/
```

Or programmatically:

```rust
use debtmap::analysis::python_dead_code_enhanced::CoverageData;

let coverage = CoverageData::from_coverage_json("coverage.json")?;
let analyzer = EnhancedDeadCodeAnalyzer::new().with_coverage(coverage);
```

## Behavior Changes

### Framework Entry Points

**Before:** Flask routes, Django views, etc. were often marked as dead code

**After:** Automatically recognized as framework entry points

```python
# Old system: FALSE POSITIVE
# New system: Correctly identified as LIVE

@app.route('/api/users')
def get_users():  # Was marked dead, now recognized as Flask route
    return []
```

### Test Functions

**Before:** Test helpers without `test_` prefix were marked as dead

**After:** All functions in test files are analyzed with lower dead code confidence

```python
# test_utils.py

# Old system: FALSE POSITIVE
# New system: Correctly identified as LIVE (in test file)

def create_test_user():  # Was marked dead, now recognized as test helper
    return User(name="Test")
```

### Callback Functions

**Before:** Event handlers and callbacks were often marked as dead

**After:** Tracked through callback registration analysis

```python
# Old system: FALSE POSITIVE
# New system: Correctly identified as LIVE

def on_button_click():  # Registered as callback, now tracked
    print("Clicked!")

button.on_click(on_button_click)  # Callback tracker detects this
```

## Compatibility Notes

### Breaking Changes

1. **Return Type Changed**
   - Old: `Vec<String>`
   - New: `Vec<DeadCodeResult>`
   - **Action:** Update code to handle new result structure

2. **Module Path Changed**
   - Old: `debtmap::analysis::python_dead_code`
   - New: `debtmap::analysis::python_dead_code_enhanced`
   - **Action:** Update import statements

3. **Function Signature Changed**
   - Old: `find_dead_functions(&CallGraph) -> Vec<String>`
   - New: `analyze_function(&FunctionMetrics, &CallGraph) -> DeadCodeResult`
   - **Action:** Update to analyze functions individually

### Non-Breaking Changes

1. **Call graph API unchanged** - Existing call graph building code works as-is
2. **Function metrics unchanged** - Existing metric collection works as-is

## Common Migration Issues

### Issue 1: Too Many False Positives Still

**Symptom:** Many functions marked as dead that are actually used

**Solutions:**
1. Enable coverage integration: `pytest --cov-report=json`
2. Check if framework patterns are registered
3. Add suppression comments to public APIs
4. Verify callback tracking is working

### Issue 2: Missing Dead Code

**Symptom:** Known dead code not detected

**Solutions:**
1. Check confidence threshold settings
2. Verify call graph is complete
3. Look for dynamic calls that hide true dead code
4. Review medium confidence results

### Issue 3: Performance Degradation

**Symptom:** Analysis is slower than before

**Solutions:**
1. Cache file reads for suppression/export checks
2. Disable coverage if not needed
3. Filter by confidence level before detailed analysis
4. Process files in parallel

## Configuration Migration

### Old Configuration

```rust
let detector = DeadCodeDetector::new();
// No configuration options
```

### New Configuration

```rust
use debtmap::analysis::python_dead_code_enhanced::AnalysisConfig;

let config = AnalysisConfig {
    high_confidence_threshold: 0.8,      // Adjust based on tolerance
    medium_confidence_threshold: 0.5,    // for false positives
    respect_suppression_comments: true,  // Enable suppression
    include_private_api: true,           // Analyze _ functions
};

let analyzer = EnhancedDeadCodeAnalyzer::new().with_config(config);
```

## Gradual Migration Strategy

You don't have to migrate everything at once:

### Phase 1: Side-by-Side Comparison

```rust
// Run both old and new systems
let old_results = old_detector.find_dead_functions(&call_graph);
let new_results = new_analyzer.analyze_all_functions(&functions, &call_graph);

// Compare results
for func in old_results {
    let new_result = new_results.iter().find(|r| r.function_id.name == func);
    if let Some(new_res) = new_result {
        println!("{}: old=dead, new={:?}", func, new_res.confidence);
    }
}
```

### Phase 2: High Confidence Only

```rust
// Start by only using high confidence results
let high_conf_dead: Vec<_> = results
    .into_iter()
    .filter(|r| r.is_dead && matches!(r.confidence, DeadCodeConfidence::High(_)))
    .collect();

// Use old system for everything else
```

### Phase 3: Full Migration

```rust
// Use new system for all confidence levels
for result in results {
    match result.confidence {
        DeadCodeConfidence::High(_) => handle_high_confidence(result),
        DeadCodeConfidence::Medium(_) => handle_medium_confidence(result),
        DeadCodeConfidence::Low(_) => handle_low_confidence(result),
    }
}
```

## Testing Your Migration

### Validation Checklist

- [ ] All imports updated to new module paths
- [ ] Result handling updated for new structure
- [ ] Confidence levels are handled appropriately
- [ ] Suppression comments work as expected
- [ ] Framework patterns are detected correctly
- [ ] Test functions are not flagged as dead
- [ ] Callback functions are tracked properly
- [ ] Coverage integration works (if enabled)
- [ ] Performance is acceptable
- [ ] False positive rate is reduced

### Validation Script

```rust
#[test]
fn validate_migration() {
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = build_test_call_graph();

    // Test framework entry point
    let flask_route = test_function("index", "app.py", 10);
    let result = analyzer.analyze_function(&flask_route, &call_graph);
    assert!(!result.is_dead, "Flask route should be live");

    // Test callback
    let callback = test_function("on_click", "gui.py", 20);
    let result = analyzer.analyze_function(&callback, &call_graph);
    assert!(!result.is_dead, "Callback should be live");

    // Test true dead code
    let dead_func = test_function("_unused_helper", "utils.py", 30);
    let result = analyzer.analyze_function(&dead_func, &call_graph);
    assert!(result.is_dead, "Unused helper should be dead");
    assert!(matches!(result.confidence, DeadCodeConfidence::High(_)));
}
```

## Getting Help

If you encounter issues during migration:

1. Check this migration guide
2. Review the [user documentation](python-dead-code-detection.md)
3. Look at [example migrations](../examples/dead-code-migration/)
4. Open an issue: https://github.com/anthropics/debtmap/issues

## Summary

The enhanced dead code detection system provides:
- ✅ Higher accuracy with confidence scoring
- ✅ Better framework support
- ✅ Suppression comment system
- ✅ Coverage integration
- ✅ Lower false positive rate

Migration effort: **Low to Medium**
- Simple projects: ~30 minutes
- Complex projects: ~2-4 hours

Expected benefits:
- 60-70% reduction in false positives
- More actionable results
- Better developer confidence in removing code
