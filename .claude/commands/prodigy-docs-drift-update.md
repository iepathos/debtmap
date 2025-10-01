# /prodigy-docs-drift-update

Updates documentation to resolve drift issues identified by `/prodigy-docs-drift-analyze`.

## Usage

```
/prodigy-docs-drift-update [update strategy...]
```

Examples:
- `/prodigy-docs-drift-update` - Update all documentation drift issues
- `/prodigy-docs-drift-update critical only` - Fix only critical issues
- `/prodigy-docs-drift-update README examples` - Update README and fix examples
- `/prodigy-docs-drift-update incremental` - Make small, focused updates
- `/prodigy-docs-drift-update comprehensive` - Full documentation overhaul

## What This Command Does

1. **Drift Report Loading**
   - Reads `.prodigy/documentation-drift.json` from analysis
   - Validates report freshness (warns if >24 hours old)
   - Prioritizes issues based on severity
   - Groups related changes for logical commits
   - Plans update sequence to avoid conflicts

2. **Update Strategy Selection**
   - Determines scope based on provided parameters
   - Applies filters for severity levels (critical/major/minor)
   - Groups updates by file or logical area
   - Sequences updates to maintain consistency
   - Respects existing documentation style

3. **Documentation Updates**
   - **README Updates**: Corrects feature lists, usage examples, installation steps
   - **API Documentation**: Updates rustdoc comments to match implementations
   - **Command Documentation**: Fixes command syntax and parameter descriptions
   - **Workflow Documentation**: Updates workflow examples and descriptions
   - **Configuration Docs**: Corrects config options and defaults
   - **Example Code**: Fixes broken examples to compile and run
   - **CLAUDE.md**: Updates project-specific instructions

4. **Consistency Enforcement**
   - Ensures terminology consistency across all docs
   - Aligns version numbers and compatibility info
   - Standardizes code formatting in examples
   - Updates cross-references between documents
   - Fixes broken links and references

5. **Validation and Testing**
   - Tests updated code examples for compilation
   - Validates YAML/TOML examples against schemas
   - Checks markdown formatting and structure
   - Ensures command examples are executable
   - Verifies workflow definitions are valid

6. **Commit Strategy**
   - Creates atomic commits for each logical change group
   - Uses descriptive commit messages with drift details
   - References specific issues being resolved
   - Maintains clean git history

## Update Strategies

- **comprehensive**: Full documentation overhaul, all issues
- **incremental**: Small, focused updates, one area at a time
- **critical**: Only critical severity issues
- **major**: Critical and major severity issues
- **all**: All issues regardless of severity
- **quick**: Fast updates without extensive testing

## Update Priorities

1. **Critical Issues First**: Wrong information that could break systems
2. **User-Facing Docs**: README, getting started, CLI help
3. **API Documentation**: Public interfaces and functions
4. **Internal Docs**: Architecture, design, implementation notes
5. **Examples Last**: Code examples and tutorials

## File Update Patterns

### README.md Updates
- Feature list accuracy
- Installation instructions
- Usage examples
- Configuration options
- Troubleshooting sections

### CLAUDE.md Updates
- Command availability
- Workflow descriptions
- Context requirements
- Integration instructions

### Command Documentation Updates
- Parameter descriptions
- Usage examples
- Output formats
- Error conditions

### Workflow Documentation Updates
- Step descriptions
- Variable usage
- Error handling
- Prerequisites

## Commit Format

Creates focused commits:

```
docs: fix [category] documentation drift in [file]

- Update [specific sections]
- Fix [broken examples]
- Correct [incorrect information]
- Align with implementation in [source files]

Resolves drift issues: [critical: N, major: N, minor: N]
```

## Safety Features

1. **Backup Creation**: Saves original files before updates
2. **Incremental Updates**: Makes changes in small batches
3. **Validation Steps**: Tests changes before committing
4. **Rollback Support**: Can revert if issues detected
5. **Dry Run Mode**: Preview changes without applying

## Integration with Analysis

Requires output from `/prodigy-docs-drift-analyze`:
- Reads `.prodigy/documentation-drift.json`
- Uses structured drift data for targeted updates
- Maintains traceability between analysis and fixes
- Updates report after successful changes

## Quality Checks

Before committing each update:
1. Validates markdown syntax
2. Tests code examples (if applicable)
3. Checks for consistency with other docs
4. Ensures no new drift introduced
5. Verifies links and references

## Update Scope Control

Parameters to limit scope:
- `README`: Only update README files
- `commands`: Only update command documentation
- `workflows`: Only update workflow documentation
- `examples`: Only fix code examples
- `inline`: Only update inline documentation
- `critical`: Only critical severity issues
- `major`: Critical and major issues only

## Error Handling

- Skips files with merge conflicts
- Reports files that couldn't be updated
- Continues with remaining updates on error
- Logs all actions for troubleshooting
- Creates partial update report

## Post-Update Actions

After completing updates:
1. Generates update summary report
2. Updates drift analysis timestamp
3. Commits changes with detailed messages
4. Suggests re-running analysis to verify
5. Provides statistics on resolved issues

## Implementation Notes

1. Preserves existing documentation style
2. Maintains file formatting conventions
3. Respects project-specific terminology
4. Keeps examples idiomatic to project
5. Ensures backward compatibility mentions
6. Updates incrementally to maintain readability

## Documentation Update Best Practices

1. **Preserve Documentation Voice** - Maintain consistent tone and style
2. **Incremental Updates** - Make focused changes to avoid conflicts
3. **Semantic Accuracy** - Ensure all technical details are correct
4. **Example Validation** - Test all code examples before committing
5. **Cross-Reference Consistency** - Update all related documentation together
6. **Version Awareness** - Include compatibility notes where relevant
7. **Clear Explanations** - Focus on clarity over brevity for complex topics

## Automation Behavior

When running in automation mode (`PRODIGY_AUTOMATION=true`):
1. Automatically commits documentation updates after validation
2. Groups related changes for logical commits
3. Provides clear summary of changes made
4. Reports any issues that couldn't be resolved

**IMPORTANT**: Do NOT add any attribution text like "🤖 Generated with [Claude Code]" or "Co-Authored-By: Claude" to commit messages. Keep commits clean and focused on the actual changes.