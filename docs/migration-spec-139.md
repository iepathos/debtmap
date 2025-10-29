# Migration Guide: Output Format Improvements (Spec 139)

This guide helps you transition to the improved output format introduced in spec 139.

## What Changed

### Severity Display
**Old format** mixed severity with issue type:
```
#1 SCORE: 85.5 [CRITICAL - FILE - GOD OBJECT]
```

**New format** separates severity from issue type for clarity:
```
#1 SCORE: 85.5 [CRITICAL]
├─ LOCATION: src/file.rs:100 MyStruct
```

**Why**: Cleaner header, easier to scan, severity stands out more clearly.

### Section Reorganization
Sections are now ordered for better readability:

**New order**:
1. **LOCATION**: Where is it?
2. **EVIDENCE**: What metrics support this?
3. **WHY THIS MATTERS**: Why should I care?
4. **RECOMMENDED ACTION**: What should I do?

**Previous order** mixed these in less intuitive ways.

### Verbosity Levels
**New**:
- `--compact` (or `-c`): Minimal output, top 3 metrics only
- Default: Balanced output, top 6 metrics
- `--verbose` (or `-v`): Full detail with all metrics

**Old**:
- No compact mode
- `--verbose` was less structured
- `--explain-score` (now deprecated, use `-v` instead)

### Color Configuration
**New**:
```toml
[output]
use_color = true  # or false, or omit for auto-detection
```

Plus `NO_COLOR` environment variable support.

**Old**:
- No configuration option
- Always auto-detected

## Side-by-Side Comparison

### Old Output (Pre-Spec 139)
```
#1 SCORE: 85.5 [CRITICAL - FILE - GOD OBJECT]
src/user_manager.rs (1200 lines, 45 functions)

COMPLEXITY: cyclomatic=25, cognitive=35, nesting=4
DEPENDENCIES: 15 upstream, 8 downstream

ACTION: Split this god object into focused modules
WHY: High complexity with many responsibilities creates maintenance burden
```

### New Output (Spec 139)
```
#1 SCORE: 85.5 [CRITICAL]
├─ LOCATION: src/user_manager.rs:1 UserManager (1200 lines, 45 functions)
├─ COMPLEXITY: cyclomatic=25, est_branches=25, cognitive=35, nesting=4
├─ WHY THIS MATTERS: High complexity with many responsibilities creates maintenance burden
├─ RECOMMENDED ACTION: Split this god object into focused modules
   - 1. Identify distinct domains (auth, profile, permissions)
   - 2. Extract each domain into separate module
   - 3. Use dependency injection to connect modules
- DEPENDENCIES: 15 upstream, 8 downstream
  - CALLERS: login_handler, registration_flow, profile_update
  - CALLS: database::users, email::notification, auth::validate
```

## Finding Information in New Format

### "Where is my issue type?"
**Old**: `[CRITICAL - FILE - GOD OBJECT]`
**New**: Look at `LOCATION` section - it shows file/function context

### "Where are the complexity metrics?"
**Old**: Scattered in output
**New**: Consolidated under `├─ COMPLEXITY:` section

### "Where's the reasoning?"
**Old**: Mixed with recommendations
**New**: Clearly labeled `├─ WHY THIS MATTERS:`

### "Where are the action items?"
**Old**: Sometimes abbreviated
**New**: `├─ RECOMMENDED ACTION:` with numbered steps

## Updating Your Workflows

### CI/CD Scripts
If you parse debtmap output:

**Old approach**:
```bash
debtmap analyze . | grep "\[CRITICAL -"
```

**New approach**:
```bash
# Use JSON format for reliable parsing
debtmap analyze . --format json | jq '.items[] | select(.severity == "CRITICAL")'

# Or use plain text with updated pattern
debtmap analyze . --plain | grep "\[CRITICAL\]"
```

### Automation
If you have automated checks:

**Update severity regex**:
- Old: `\[(CRITICAL|HIGH|MEDIUM|LOW) - [A-Z]+ - `
- New: `\[(CRITICAL|HIGH|MEDIUM|LOW)\]`

**Use JSON format** for reliable parsing:
```bash
debtmap analyze . --format json --output results.json
```

JSON structure provides stable field names regardless of output format changes.

## Configuration Migration

### Adding Color Control
Add to your `.debtmap.toml`:

```toml
[output]
# Explicitly enable or disable colors
use_color = true

# Or rely on auto-detection (omit the setting)
```

### Environment Variables
Respect `NO_COLOR` in CI:
```bash
# In your CI config
NO_COLOR=1 debtmap analyze .
```

## Backward Compatibility

### JSON Output
JSON format remains **fully backward compatible**. Field names and structure unchanged.

### Plain Text Output
Use `--plain` flag for most stable text output (no colors, no emoji, predictable format).

### Legacy Parsing
If you can't update parsing immediately:

**Option 1**: Pin to pre-spec-139 version
```toml
# Cargo.toml
debtmap = "=0.x.y"  # Replace with version before spec 139
```

**Option 2**: Use JSON format
```bash
debtmap analyze . --format json
```

JSON structure is stable and changes rarely.

## Benefits of New Format

### Improved Scannability
- Clean severity tags `[CRITICAL]` stand out
- Consistent tree structure with `├─` prefixes
- Logical section ordering

### Better Actionability
- Clear separation of evidence vs. action
- Numbered implementation steps
- Explicit rationale for each item

### Enhanced Verbosity Control
- `--compact` for quick scans
- Default for day-to-day use
- `--verbose` for deep analysis

### Clearer Context
- Evidence explains "what"
- Why This Matters explains "why"
- Recommended Action explains "how"

## Examples

### Before (Old Format)
```bash
debtmap analyze . --explain-score
```

### After (New Format)
```bash
# Equivalent to old --explain-score
debtmap analyze . --verbose  # or -v, -vv, -vvv

# Quick scan (new feature)
debtmap analyze . --compact

# Balanced (default)
debtmap analyze .
```

### CI/CD Migration
**Before**:
```bash
#!/bin/bash
debtmap analyze . > results.txt
critical_count=$(grep -c "\[CRITICAL -" results.txt)
if [ "$critical_count" -gt 5 ]; then
    echo "Too many critical issues: $critical_count"
    exit 1
fi
```

**After**:
```bash
#!/bin/bash
# More reliable with JSON
debtmap analyze . --format json --output results.json
critical_count=$(jq '[.items[] | select(.unified_score.final_score >= 8.0)] | length' results.json)
if [ "$critical_count" -gt 5 ]; then
    echo "Too many critical issues: $critical_count"
    exit 1
fi
```

## Getting Help

### Check Your Output
Compare your output to examples in this guide. If sections are missing or ordered differently, ensure you're using the latest version.

### Use JSON for Parsing
If text parsing breaks, switch to JSON format:
```bash
debtmap analyze . --format json
```

### Report Issues
If you encounter problems migrating:
1. Check version: `debtmap --version`
2. Review configuration: `.debtmap.toml`
3. Try `--plain` flag for simplest output
4. Open issue with example output

## Summary

**Key Changes**:
- ✅ Cleaner severity tags (just `[CRITICAL]`, not `[CRITICAL - FILE - GOD OBJECT]`)
- ✅ Logical section ordering (LOCATION → EVIDENCE → WHY → ACTION)
- ✅ New `--compact` flag for minimal output
- ✅ Color configuration in `.debtmap.toml`
- ✅ `NO_COLOR` environment variable support

**Migration Path**:
1. Update scripts to look for `[CRITICAL]` instead of `[CRITICAL -`
2. Switch to JSON format for reliable parsing
3. Add color configuration to `.debtmap.toml` if needed
4. Update documentation to reference new section names

**Backward Compatibility**:
- JSON format unchanged
- `--plain` flag provides stable text output
- Can pin to older version if needed

## See Also

- [Output Format Guide](./output-format-guide.md) - Detailed format documentation
- [CLI Reference](../book/src/cli-reference.md) - All CLI options
- [Configuration Guide](../book/src/configuration.md) - Config file options
