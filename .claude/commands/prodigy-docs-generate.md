# /prodigy-docs-generate

Generates comprehensive Rust documentation for undocumented or poorly documented code, using project context from .prodigy/ analysis.

## Usage

```
/prodigy-docs-generate [documentation type description...]
```

Examples:
- `/prodigy-docs-generate` - Generate docs for all undocumented Rust code
- `/prodigy-docs-generate public API documentation` - Focus on public API documentation
- `/prodigy-docs-generate usage examples and tutorials` - Generate usage examples and tutorials
- `/prodigy-docs-generate architecture overview` - Create architecture documentation
- `/prodigy-docs-generate module documentation for core components` - Document specific modules

## What This Command Does

1. **Project Context Gathering**
   - Reads .prodigy/context/ analysis data for project understanding
   - Reviews architecture patterns and conventions
   - Analyzes dependency graph and module structure
   - Identifies high-priority undocumented areas

2. **Rust Documentation Analysis**
   - Scans for missing /// doc comments on public items
   - Identifies undocumented public APIs (functions, structs, enums, traits)
   - Finds missing module-level documentation
   - Detects outdated or incomplete documentation
   - Evaluates documentation quality against Rust standards

3. **Rust Documentation Generation**
   - Creates comprehensive rustdoc-compatible documentation
   - Generates usage examples with proper code blocks
   - Documents design decisions and architectural patterns
   - Adds inline code comments following Rust conventions
   - Ensures examples compile and follow project patterns

4. **Automatic Commit**
   - Commits generated documentation changes
   - Creates descriptive commit message
   - Preserves git history with documentation improvements

## Rust Documentation Types

- **API Documentation**: Rustdoc comments (///) for public functions, structs, enums, traits
- **Module Documentation**: //! comments explaining module purpose and architecture
- **Usage Examples**: Code blocks with `# Examples` sections that compile
- **Crate Documentation**: Top-level crate documentation in lib.rs
- **Error Documentation**: Documenting error types and handling patterns
- **Feature Documentation**: Documenting optional features and their usage

## Rust Documentation Standards

- Follow rustdoc conventions (/// for items, //! for modules)
- Include `# Examples` sections with working code
- Document all public APIs with clear descriptions
- Use `# Panics`, `# Errors`, and `# Safety` sections when appropriate
- Cross-reference related items with `[item]` syntax
- Include doctests that actually compile and run

## Context Integration

Before generating documentation, the command:

1. **Reads MMM Context**: Checks for .prodigy/context/ directory and loads:
   - `analysis.json` - Complete project analysis
   - `architecture.json` - Architectural patterns and violations
   - `conventions.json` - Code conventions and naming patterns
   - `technical_debt.json` - Priority areas needing documentation

2. **Prioritizes Documentation**: Uses context data to:
   - Target high-impact undocumented areas
   - Follow existing project conventions
   - Address architectural documentation gaps
   - Focus on frequently changed or complex modules

## Commit Format

Automatically commits changes with format:

```
docs: generate [documentation type] documentation

- Add rustdoc comments for [specific areas]
- Include usage examples and code samples
- Follow project conventions from .prodigy/context analysis
```

## Implementation Workflow

1. **Context Analysis**: Read .prodigy/context/ data for project understanding
2. **Scope Determination**: Parse multi-word prompt to determine documentation scope
3. **Gap Analysis**: Identify missing or inadequate documentation
4. **Content Generation**: Create rustdoc-compatible documentation
5. **Quality Check**: Ensure examples compile and follow conventions
6. **Commit Changes**: Automatically commit with descriptive message

**IMPORTANT**: Never add Claude attribution or emoji to git commits. Keep commits clean and professional.
