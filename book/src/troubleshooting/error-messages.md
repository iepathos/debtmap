# Error Messages Reference

Understanding common error messages, error codes, and how to resolve them.

## Error Code System

Debtmap uses structured error codes for programmatic handling and documentation lookup. Error codes are assigned by category (from `src/debtmap_error.rs:62-116`):

| Code Range | Category | Description |
|------------|----------|-------------|
| E001-E009 | I/O | File system and I/O errors |
| E010-E019 | Parse | Source code parsing errors |
| E020-E029 | Config | Configuration file errors |
| E030-E039 | Analysis | Analysis algorithm errors |
| E040-E049 | CLI | Command-line argument errors |
| E050-E059 | Validation | Input validation errors |

### I/O Error Codes (E001-E009)

| Code | Name | Description |
|------|------|-------------|
| E001 | `IO_FILE_NOT_FOUND` | File or directory does not exist |
| E002 | `IO_PERMISSION_DENIED` | Insufficient permissions to read file |
| E003 | `IO_RESOURCE_BUSY` | File is locked or resource temporarily unavailable |
| E009 | `IO_GENERIC` | Other I/O errors |

### Parse Error Codes (E010-E019)

| Code | Name | Description |
|------|------|-------------|
| E010 | `PARSE_SYNTAX` | Syntax error in source file |
| E011 | `PARSE_UNSUPPORTED` | Unsupported language or feature |
| E012 | `PARSE_ENCODING` | Invalid file encoding |
| E019 | `PARSE_GENERIC` | Other parse errors |

### Configuration Error Codes (E020-E029)

| Code | Name | Description |
|------|------|-------------|
| E020 | `CONFIG_INVALID` | Invalid configuration value |
| E021 | `CONFIG_MISSING` | Missing required configuration field |
| E022 | `CONFIG_FILE_NOT_FOUND` | Configuration file not found |
| E029 | `CONFIG_GENERIC` | Other configuration errors |

### Analysis Error Codes (E030-E039)

| Code | Name | Description |
|------|------|-------------|
| E030 | `ANALYSIS_COMPLEXITY` | Complexity calculation failed |
| E031 | `ANALYSIS_COVERAGE` | Coverage data loading failed |
| E032 | `ANALYSIS_SCORING` | Debt scoring calculation failed |
| E039 | `ANALYSIS_GENERIC` | Other analysis errors |

### CLI Error Codes (E040-E049)

| Code | Name | Description |
|------|------|-------------|
| E040 | `CLI_INVALID_COMMAND` | Unknown or invalid command |
| E041 | `CLI_MISSING_ARG` | Required argument not provided |
| E042 | `CLI_INVALID_ARG` | Invalid argument value |
| E049 | `CLI_GENERIC` | Other CLI errors |

### Validation Error Codes (E050-E059)

| Code | Name | Description |
|------|------|-------------|
| E050 | `VALIDATION_GENERIC` | General validation failure |
| E051 | `VALIDATION_THRESHOLD` | Threshold validation exceeded |
| E052 | `VALIDATION_CONSTRAINT` | Constraint violation |

## Error Classification

Debtmap classifies errors to help determine appropriate action (from `src/debtmap_error.rs:466-519`):

### Retryable vs Non-Retryable Errors

**Retryable errors** may succeed on a subsequent attempt:
- Resource busy / file locks (`E003`)
- Network timeouts
- Connection issues
- "Temporarily unavailable" errors

**Non-retryable errors** require user intervention:
- Parse/syntax errors
- Configuration errors
- File not found (permanent)
- Validation errors

### User-Fixable vs System Errors

**User-fixable errors** can be resolved by modifying input:
- Configuration errors (fix config file)
- CLI errors (fix command arguments)
- Validation errors (fix input values)
- Parse errors (fix source code syntax)

**System errors** are outside user control:
- I/O errors (system issues)
- Analysis errors (internal algorithm issues)

## Exit Codes

Debtmap returns specific exit codes for CI/CD integration (from `src/debtmap_error.rs:521-532`):

| Exit Code | Meaning | When Returned |
|-----------|---------|---------------|
| 0 | Success | Analysis completed successfully |
| 1 | Analysis/I/O Error | Analysis failed or I/O error occurred |
| 2 | CLI Error | Invalid command-line usage |
| 3 | Config Error | Configuration file or value error |
| 4 | Validation Error | Validation threshold exceeded |
| 5 | Parse Error | Source code parsing failed |

**CI/CD Integration Example:**

```bash
#!/bin/bash
debtmap analyze .
EXIT_CODE=$?

case $EXIT_CODE in
  0)
    echo "Analysis passed"
    ;;
  1)
    echo "Analysis or I/O error - check logs"
    exit 1
    ;;
  2)
    echo "CLI usage error - check command syntax"
    exit 1
    ;;
  3)
    echo "Configuration error - check .debtmap.toml"
    exit 1
    ;;
  4)
    echo "Validation failed - thresholds exceeded"
    exit 1
    ;;
  5)
    echo "Parse error - source code issues"
    exit 1
    ;;
esac
```

## File System Errors

**Message**: `[E001] I/O error: No such file or directory`

**Meaning**: File or directory does not exist

**Solutions**:
- Verify path is correct: `ls -la <path>`
- Check current working directory: `pwd`
- Use absolute paths if needed

---

**Message**: `[E002] I/O error: Permission denied`

**Meaning**: Cannot read file or directory due to permissions

**Solutions**:
- Check file permissions: `ls -la <file>`
- Ensure user has read access
- Run from appropriate directory

---

**Message**: `[E003] I/O error: Resource busy`

**Meaning**: File is locked or temporarily unavailable (retryable)

**Solutions**:
- Wait and retry the analysis
- Close other programs using the file
- Check for file locks

## Parse Errors

**Message**: `[E010] Parse error in file.rs: unexpected token at line 42, column 10`

**Meaning**: Syntax that debtmap cannot parse

**Solutions**:
```bash
# Try fallback mode without semantic analysis
debtmap analyze . --semantic-off

# For Rust macro issues, enable verbose warnings
debtmap analyze . --verbose-macro-warnings --show-macro-stats
```

---

**Message**: `[E011] Parse error: unsupported language`

**Meaning**: The file type is not supported or has unsupported features

**Solutions**:
- Check that the file is a supported language (Rust, Python, JavaScript, TypeScript)
- For Rust, complex procedural macros may require `--semantic-off`

## Configuration Errors

**Message**: `[E020] Configuration error: invalid config value`

**Meaning**: Invalid configuration in `.debtmap.toml` or CLI flags

**Configuration File Locations** (checked in order):
1. `.debtmap.toml` in project root (project-level)
2. `~/.config/debtmap/config.toml` (global user config)

**Solutions**:
- Check `.debtmap.toml` syntax with a TOML validator
- Review CLI flag values
- Check for typos in configuration keys

**Example valid configuration:**
```toml
# .debtmap.toml
[thresholds]
complexity = 10
max_file_length = 500

[scoring]
coverage = 0.40
complexity = 0.30
```

---

**Message**: `[E022] Configuration error: file not found`

**Meaning**: Specified configuration file does not exist

**Solutions**:
- Create `.debtmap.toml` from the example: `cp .debtmap.toml.example .debtmap.toml`
- Verify the configuration path if using `--config` flag

## Validation Errors

**Message**: `[E051] Validation error: threshold validation failed`

**Meaning**: Analysis results exceed configured thresholds

**Solutions**:
- Check threshold values in `.debtmap.toml` under `[thresholds.validation]`
- Ensure `--min-priority` is in valid range (0-10)
- Use `--threshold-preset` with a valid preset name

**Validation thresholds example:**
```toml
[thresholds.validation]
max_average_complexity = 10.0
max_high_complexity_count = 100
max_debt_items = 2000
max_total_debt_score = 10000
max_codebase_risk_score = 7.0
```

## Analysis Errors

**Message**: `[E039] Analysis error: internal analysis failure`

**Meaning**: Internal error during analysis phase

**Solutions**:
```bash
# Try fallback mode
debtmap analyze . --semantic-off

# Report with debug info
debtmap analyze . -vvv 2>&1 | tee error.log

# Isolate problem file
debtmap analyze . --max-files 1 path/to/suspected/file
```

---

**Message**: `[E031] Analysis error: Coverage error`

**Meaning**: Failed to load or process coverage data

**Solutions**:
```bash
# Analyze without coverage data
debtmap analyze . --no-coverage

# Check coverage file format
debtmap explain-coverage <coverage-file>

# Verify function name matching
debtmap analyze . -vvv | grep -i coverage
```

## Dependency Errors

**Message**: `Dependency error: cannot resolve dependency graph`

**Meaning**: Cannot build dependency relationships

**Solutions**:
```bash
# Disable dependency provider
debtmap analyze . --context --disable-context dependency

# Try without context
debtmap analyze .
```

## Concurrency Errors

**Message**: `Concurrency error: parallel processing failure`

**Meaning**: Error during parallel execution

**Solutions**:
```bash
# Disable parallel processing
debtmap analyze . --no-parallel

# Reduce thread count
debtmap analyze . --jobs 1
```

## Pattern Errors

**Message**: `Pattern error: invalid glob pattern`

**Meaning**: Invalid glob pattern in configuration or CLI

**Solutions**:
- Check glob pattern syntax
- Escape special characters if needed
- Use simpler patterns or path prefixes

## Handling False Positives

Debtmap includes context-aware false positive reduction, enabled by default. This uses pattern-based classification to reduce spurious debt reports (from `src/cli/args.rs:202-204`).

### Controlling False Positive Reduction

```bash
# Default: context-aware analysis enabled
debtmap analyze .

# Disable context-aware analysis for raw results
debtmap analyze . --no-context-aware
```

### When to Expect False Positives

False positives are more likely with:
- Complex macro-generated code
- Unusual code patterns
- Generated or vendored code
- Test utilities with intentionally complex patterns

**Reducing false positives:**
1. Use context-aware analysis (default)
2. Configure exclusion patterns in `.debtmap.toml`
3. Use `--verbose-macro-warnings` to identify macro issues

## Boilerplate Detection Messages

Debtmap identifies boilerplate patterns that may be better suited for macros or code generation (from `src/organization/boilerplate_detector.rs:33-58`).

### Understanding Boilerplate Reports

**Message**: `Boilerplate detected: high trait implementation density`

**Meaning**: File contains many similar trait implementations with low complexity, suggesting macro-ification would reduce maintenance burden.

**Detection criteria:**
- 20+ impl blocks (configurable via `min_impl_blocks`)
- 70%+ method uniformity across implementations
- Average complexity below 2.0 (simple, repetitive code)
- 70%+ confidence threshold

### Acting on Boilerplate Recommendations

When a file is flagged as boilerplate:

1. **Consider macro extraction**: If implementing the same trait for many types, use a declarative macro
2. **Consider code generation**: For very large patterns, use build.rs or proc-macros
3. **Review the recommendation**: Debtmap provides specific suggestions for each pattern

**Boilerplate is NOT the same as complex code:**
- Complex code (high cyclomatic complexity) → split into modules
- Boilerplate code (low complexity, high repetition) → macro-ify or generate

## Language-Specific Issues

### Rust Macro Handling

Rust macros may produce parse warnings or analysis limitations:

```bash
# Enable verbose macro warnings
debtmap analyze . --verbose-macro-warnings

# Show macro expansion statistics
debtmap analyze . --show-macro-stats

# Both together for full diagnostics
debtmap analyze . --verbose-macro-warnings --show-macro-stats
```

### Language Support Status

| Language | Support Level | Notes |
|----------|--------------|-------|
| Rust | Full | Primary language, best analysis |
| Python | Stub | Basic complexity analysis |
| JavaScript | Stub | Basic complexity analysis |
| TypeScript | Stub | Basic complexity analysis |

## See Also

- [Quick Fixes](quick-fixes.md) - Common problems with immediate solutions
- [Debug Mode](debug-mode.md) - Verbosity levels for diagnostics
- [FAQ](faq.md) - Frequently asked questions
