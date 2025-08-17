# Debt Pattern Integration Status

## Current State
While debtmap has detectors for most debt patterns, they are NOT fully integrated into the unified scoring system.

## Integration Gaps

### 1. Score Integration Gap
**Problem**: Detected issues (security, performance, organization) don't influence unified scores
**Current**: Only function name patterns and basic metrics affect security/organization scores
**Needed**: Aggregate detected issues per function and include in score calculation

### 2. Data Flow Gap
**Problem**: DebtItems from detectors don't connect to FunctionMetrics scoring
**Current Flow**:
```
Detectors → DebtItem → Separate reporting
FunctionMetrics → UnifiedScore → Priority ranking
```
**Needed Flow**:
```
Detectors → DebtItem → Aggregate by function → Include in UnifiedScore
```

### 3. Missing Aggregation Layer
Need to:
1. Collect all DebtItems per function
2. Calculate security score based on actual security issues found
3. Calculate organization score based on actual organization issues found
4. Calculate performance score based on actual performance issues found

## Implementation Required

### Step 1: Add Debt Aggregation
```rust
struct FunctionDebtProfile {
    security_issues: Vec<DebtItem>,
    performance_issues: Vec<DebtItem>,
    organization_issues: Vec<DebtItem>,
    test_issues: Vec<DebtItem>,
    resource_issues: Vec<DebtItem>,
}
```

### Step 2: Enhance Score Calculation
```rust
fn calculate_security_factor(func: &FunctionMetrics, debt_profile: &FunctionDebtProfile) -> f64 {
    // Current: Just looks at function name
    // Needed: Also consider actual detected security issues
    let detected_score = debt_profile.security_issues.len() as f64 * 2.0;
    // ... combine with pattern matching
}
```

### Step 3: Update UnifiedDebtItem Creation
- Pass debt profile to score calculation
- Include detected issues in scoring
- Ensure all debt types influence final priority

## Debt Patterns Coverage

### Fully Detected (25 patterns)
✅ All major patterns have detectors

### Partially Integrated (4 patterns)
⚠️ Code duplication, circular deps, coupling, large files

### Not Integrated into Scoring (Most)
❌ Detected issues don't affect unified priority scores

## Priority Actions

1. **Create debt aggregation by function**
2. **Update score calculations to use detected issues**
3. **Ensure all DebtTypes map to appropriate score categories**
4. **Test that detected issues influence priority rankings**