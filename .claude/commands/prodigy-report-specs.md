# Report Specs Command

Reports on specifications created during a workflow run, providing a summary of what was generated and their completion status.

Arguments: $ARGUMENTS

## Usage

```
/prodigy-report-specs <spec-files>
```

Examples:
- `/prodigy-report-specs specs/102-auth.md specs/103-session.md`
- `/prodigy-report-specs ${workflow.files_added:specs/*.md}`

## What This Command Does

1. **Analyzes Created Specifications**
   - Reads all spec files provided
   - Extracts key metadata from each spec
   - Determines completion status

2. **Generates Summary Report**
   - Lists all specifications created
   - Shows spec numbers and titles
   - Reports overall completion status
   - Highlights any dependencies between specs

3. **Provides Implementation Guidance**
   - Suggests implementation order based on dependencies
   - Notes any prerequisites
   - Identifies parallel implementation opportunities

## Execution Process

### Step 1: Parse Input

- Extract spec files list from $ARGUMENTS
- Handle empty list gracefully (report no specs created)
- Sort specs by number for consistent reporting

### Step 2: Read Spec Metadata

For each spec file:
- Parse YAML frontmatter for metadata
- Extract spec number, title, category, priority
- Identify dependencies on other specs
- Check completion status

### Step 3: Analyze Specifications

Build comprehensive understanding:
- Total number of specs created
- Categories represented
- Priority distribution
- Dependency graph between specs

### Step 4: Generate Report

Create detailed summary report:

```
═══════════════════════════════════════════════════════
SPECIFICATION REPORT
═══════════════════════════════════════════════════════

Created 3 specifications:

📋 Spec 102: Authentication System
   Category: feature
   Priority: high
   Status: complete
   Dependencies: none

📋 Spec 103: Session Management
   Category: feature
   Priority: high
   Status: complete
   Dependencies: [102]

📋 Spec 104: Authorization Framework
   Category: feature
   Priority: medium
   Status: complete
   Dependencies: [102, 103]

───────────────────────────────────────────────────────
IMPLEMENTATION GUIDANCE
───────────────────────────────────────────────────────

Suggested implementation order:
1. Spec 102 - Authentication System (no dependencies)
2. Spec 103 - Session Management (depends on 102)
3. Spec 104 - Authorization Framework (depends on 102, 103)

Parallel implementation opportunities:
- None identified (specs have sequential dependencies)

───────────────────────────────────────────────────────
SUMMARY
───────────────────────────────────────────────────────

✅ All 3 specifications are complete and ready for implementation
📊 Categories: 3 feature specs
🎯 Priorities: 2 high, 1 medium

Next steps:
1. Run: prodigy cook implement-spec.yml --args 102
2. After completion, run: prodigy cook implement-spec.yml --args 103
3. Finally, run: prodigy cook implement-spec.yml --args 104
```

### Step 5: Handle Edge Cases

#### No Specs Created
```
No specifications were created during this workflow run.
This might indicate an issue with the spec generation process.
```

#### Single Spec Created
```
Created 1 specification:

📋 Spec 102: Simple Feature
   Category: feature
   Priority: medium
   Status: complete

✅ Specification is ready for implementation
Next step: prodigy cook implement-spec.yml --args 102
```

#### Incomplete Specs Present
```
⚠️ Warning: 2 of 3 specifications are incomplete

Incomplete specifications:
- Spec 103: Missing testing strategy
- Spec 104: Vague acceptance criteria

Consider running refinement before implementation.
```

## Report Formatting

### Console Output Structure

The report uses clear visual hierarchy:
- Headers with box drawing characters
- Icons for visual clarity (📋, ✅, ⚠️, 📊, 🎯)
- Indented details for each spec
- Clear sections for different information

### Information Included

For each specification:
- Spec number and title
- Category (feature, bugfix, optimization, etc.)
- Priority (high, medium, low)
- Completion status
- Dependencies on other specs

### Implementation Guidance

Provides actionable next steps:
- Suggested implementation order
- Dependency-aware sequencing
- Parallel opportunities identification
- Specific commands to run

## Integration Features

### Workflow Variables

Designed to work with:
- `${workflow.files_added:specs/*.md}` - All specs created in workflow
- `${step.files_added:specs/*.md}` - Specs from specific step

### Dependency Analysis

Automatically determines:
- Which specs can be implemented in parallel
- Required implementation order
- Circular dependency detection (with warnings)

## Error Handling

- If no specs provided: Report that no specs were created
- If spec files missing: List missing files in error section
- If invalid spec format: Include spec in report with error notation
- If circular dependencies: Warn and suggest resolution

## Advanced Features

### Category Grouping

Groups specs by category for large batches:
```
SPECIFICATIONS BY CATEGORY

Features (5 specs):
- 102: Authentication System
- 103: Session Management
- 104: Authorization Framework
- 105: User Profile Management
- 106: Password Reset Flow

Optimizations (2 specs):
- 107: Query Performance Improvements
- 108: Caching Layer Implementation

Bug Fixes (1 spec):
- 109: Fix Session Timeout Issue
```

### Dependency Visualization

For complex dependency graphs:
```
DEPENDENCY GRAPH

102 (Authentication)
├── 103 (Session Management)
│   └── 104 (Authorization)
└── 105 (User Profiles)

106 (Password Reset) [standalone]
107 (Performance) [standalone]
```

### Quick Reference

Generates implementation checklist:
```
IMPLEMENTATION CHECKLIST

□ Implement spec 102 - Authentication System
□ Implement spec 103 - Session Management
□ Implement spec 104 - Authorization Framework
□ Implement spec 105 - User Profile Management
□ Implement spec 106 - Password Reset Flow
```

## Output Examples

### Success Case
Reports all specs created successfully with clear next steps

### Partial Success
Reports which specs were created, which failed, and remediation steps

### Failure Case
Explains why no specs could be created and suggests troubleshooting