# /prodigy-docs-drift-analyze

Analyzes documentation drift between code implementation and various documentation sources (README, CLAUDE.md, inline docs, etc.).

## Usage

```
/prodigy-docs-drift-analyze [focus areas...]
```

Examples:
- `/prodigy-docs-drift-analyze` - Analyze all documentation drift
- `/prodigy-docs-drift-analyze README API` - Focus on README and API documentation
- `/prodigy-docs-drift-analyze workflows commands` - Check workflow and command documentation
- `/prodigy-docs-drift-analyze architecture` - Focus on architectural documentation

## What This Command Does

1. **Documentation Source Discovery**
   - Identifies all documentation files (*.md, *.rst, *.txt)
   - Locates inline documentation (rustdoc comments, docstrings)
   - Finds configuration documentation (YAML comments, TOML descriptions)
   - Discovers example files and tutorials
   - Identifies Claude-specific documentation (CLAUDE.md, .claude/commands/)

2. **Implementation Analysis**
   - Scans source code for actual functionality
   - Identifies public APIs and their signatures
   - Maps configuration options and their defaults
   - Traces workflow definitions and command structures
   - Analyzes feature flags and capabilities

3. **Drift Detection**
   - Compares documented features vs implemented features
   - Identifies outdated examples that no longer work
   - Finds deprecated functionality still in documentation
   - Detects undocumented new features
   - Checks for incorrect parameter descriptions
   - Validates command-line examples and usage patterns
   - Verifies configuration examples match current schema

4. **Consistency Analysis**
   - Cross-references documentation sources for conflicts
   - Checks naming consistency across docs
   - Validates version information alignment
   - Ensures example code matches current APIs
   - Verifies workflow definitions match available commands

5. **Report Generation**
   - Creates detailed drift report in JSON format
   - Saves to `.prodigy/documentation-drift.json`
   - Prioritizes issues by severity and impact
   - Groups related drift items together
   - Provides specific file locations and line numbers

## Drift Categories

- **Missing Documentation**: Features without any documentation
- **Outdated Documentation**: Docs describing old behavior
- **Incorrect Documentation**: Docs with wrong information
- **Inconsistent Documentation**: Conflicting information across sources
- **Broken Examples**: Code examples that don't compile/run
- **Deprecated References**: Docs referring to removed features

## Analysis Scope

The command analyzes:

1. **README Files**: Project READMEs and subdirectory READMEs
2. **CLAUDE.md**: Project-specific Claude instructions
3. **Command Documentation**: .claude/commands/*.md files
4. **Workflow Documentation**: workflows/*.yml comments and README
5. **API Documentation**: Rustdoc comments and generated docs
6. **Configuration Docs**: Config file comments and schemas
7. **Example Files**: Examples directory and embedded examples
8. **CLI Help Text**: Command-line help strings in code

## Output Format

Creates `.prodigy/documentation-drift.json`:

```json
{
  "timestamp": "2024-01-01T12:00:00Z",
  "summary": {
    "total_issues": 42,
    "critical": 5,
    "major": 15,
    "minor": 22
  },
  "categories": {
    "missing": [...],
    "outdated": [...],
    "incorrect": [...],
    "inconsistent": [...],
    "broken_examples": [...]
  },
  "files": {
    "README.md": [...],
    "CLAUDE.md": [...],
    ".claude/commands/...": [...]
  }
}
```

## Severity Levels

- **Critical**: Completely wrong information that could break systems
- **Major**: Significant discrepancies affecting usability
- **Minor**: Small inconsistencies or typos
- **Info**: Suggestions for improvement

## Integration with Update Command

This command produces output consumed by `/prodigy-docs-drift-update`:
- Structured JSON format for automated processing
- Specific file locations for targeted updates
- Prioritized issues for systematic resolution
- Grouped changes for logical commits

## Focus Areas

When provided, limits analysis to specific areas:
- `README`: Main README and feature documentation
- `API`: API documentation and rustdoc comments
- `workflows`: Workflow definitions and documentation
- `commands`: Claude command documentation
- `config`: Configuration documentation
- `examples`: Example code and tutorials
- `architecture`: High-level architecture docs

## Implementation Notes

1. Uses ripgrep for fast file scanning
2. Parses Rust code with regex for API extraction
3. Validates YAML/TOML against actual schemas
4. Tests example code snippets for compilation
5. Cross-references all documentation sources
6. Generates machine-readable output for automation

## Documentation Best Practices

When analyzing documentation:
1. **Preserve existing style** - Maintain the project's documentation voice and format
2. **Check semantic accuracy** - Ensure technical descriptions match implementation
3. **Validate examples** - All code examples should compile and run
4. **Maintain consistency** - Use consistent terminology across all docs
5. **Follow project conventions** - Respect existing documentation patterns

## Commit Format

When creating commits (if required):
- Use format: `docs: analyze documentation drift`
- Include summary of findings in commit body
- **IMPORTANT**: Do NOT add any attribution text like "🤖 Generated with [Claude Code]" or "Co-Authored-By: Claude" to commit messages