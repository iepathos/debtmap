# Error Messages Reference

Understanding common error messages and how to resolve them.

## File System Errors

**Message**: `File system error: Permission denied`

**Meaning**: Cannot read file or directory due to permissions

**Solutions**:
- Check file permissions: `ls -la <file>`
- Ensure user has read access

---

**Message**: `File system error: No such file or directory`

**Meaning**: File or directory does not exist

**Solutions**:
- Verify path is correct
- Check current working directory: `pwd`
- Use absolute paths if needed

## Parse Errors

**Message**: `Parse error in file.rs:line:column: unexpected token`

**Meaning**: Syntax debtmap cannot parse

**Solutions**:
```bash
# Try fallback mode
debtmap analyze . --semantic-off

# For Rust macros
debtmap analyze . --verbose-macro-warnings --show-macro-stats
```

## Analysis Errors

**Message**: `Analysis error: internal analysis failure`

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

## Configuration Errors

**Message**: `Configuration error: invalid config value`

**Meaning**: Invalid configuration in `.debtmap/config.toml` or CLI

**Solutions**:
- Check `.debtmap/config.toml` syntax
- Review CLI flag values
- Check for typos in flag names

## Validation Errors

**Message**: `Validation error: threshold validation failed`

**Meaning**: Threshold configuration is invalid

**Solutions**:
- Check threshold values in config
- Ensure `--min-priority` is in valid range (0-10)
- Use `--threshold-preset` with valid preset name

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

## See Also

- [Quick Fixes](quick-fixes.md) - Common problems with immediate solutions
- [Debug Mode](debug-mode.md) - Verbosity levels for diagnostics
