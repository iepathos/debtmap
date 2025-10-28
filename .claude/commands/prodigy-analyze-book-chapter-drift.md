# /prodigy-analyze-book-chapter-drift

Analyze a specific chapter of a project's book for drift against the actual codebase implementation.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy", "Debtmap")
- `--json <chapter>` - JSON object containing chapter details
- `--features <path>` - Path to features.json file (e.g., ".prodigy/book-analysis/features.json")

## Execute

### Phase 1: Understand Context

You are comparing a chapter of a project's book against the actual codebase to identify drift (discrepancies between documentation and implementation).

### Phase 2: Parse Input Arguments

**Extract Parameters:**
- `--project`: The project name (used in output messages and file paths)
- `--json`: JSON object with chapter details
- `--features`: Path to the features.json file

**Parse Chapter JSON:**
The `--json` parameter contains chapter details:

```bash
CHAPTER_JSON="<value from --json parameter>"
CHAPTER_ID=$(echo "$CHAPTER_JSON" | jq -r '.id')
CHAPTER_TITLE=$(echo "$CHAPTER_JSON" | jq -r '.title')
CHAPTER_FILE=$(echo "$CHAPTER_JSON" | jq -r '.file')
TOPICS=$(echo "$CHAPTER_JSON" | jq -r '.topics[]')
VALIDATION_FOCUS=$(echo "$CHAPTER_JSON" | jq -r '.validation')
```

### Phase 3: Perform Drift Analysis

#### Step 1: Read Current Chapter

Read the chapter file specified in `$CHAPTER_FILE`:
- Parse markdown structure
- Extract code examples
- Note explanations and descriptions
- Check for version compatibility notes

#### Step 2: Load Feature Inventory

Read the features.json file specified by `--features` parameter for ground truth from codebase.

#### Step 3: Chapter-Specific Drift Checks

Based on `$CHAPTER_ID`, perform targeted drift analysis:

#### For "workflow-basics":
- Compare basic workflow structure against WorkflowConfig
- Check command execution flow is accurate
- Verify YAML syntax examples are valid
- Ensure simple examples work with current code
- Check git integration explanation is current

#### For "mapreduce":
- Compare setup/map/reduce phases against MapReduceWorkflowConfig
- Verify agent_template syntax (both array and nested formats)
- Check max_parallel and other configuration options
- Ensure parallel execution model is explained correctly
- Verify checkpoint/resume capabilities documented

#### For "commands":
- Compare all command types against WorkflowStepCommand enum
- Check each command type has correct fields
- Verify examples use current syntax
- Ensure deprecated commands are marked
- Check goal_seek, foreach, validation commands are current

#### For "variables":
- Compare variable types against VariableStore
- Check all standard variables listed
- Verify MapReduce variables (item, map.*, etc.)
- Check capture formats documented
- Verify variable interpolation syntax

#### For "environment":
- Check environment configuration against implementation
- Verify secrets management syntax
- Check environment profiles documented
- Verify step-level environment overrides

#### For "advanced":
- Check conditional execution (when:)
- Verify timeout configuration
- Check nested conditionals
- Verify output capture formats
- Check working_dir and other advanced features

#### For "error-handling":
- Compare error policies against WorkflowErrorPolicy
- Check on_failure configuration
- Verify DLQ explanation
- Check circuit breaker documented
- Verify retry mechanisms

#### For "examples":
- Parse each YAML example
- Verify examples are syntactically valid
- Check examples use current field names
- Ensure examples reflect best practices
- Verify examples actually work

#### For "troubleshooting":
- Check common issues are still relevant
- Verify solutions work with current code
- Check error messages match current implementation
- Ensure debug tips are accurate

#### Step 4: Categorize Drift Issues

**Missing Content** (High severity):
- Feature exists in code but not in chapter
- Important capability not explained
- Critical field not documented

**Outdated Information** (High severity):
- Information no longer accurate
- Syntax changed but chapter shows old syntax
- Capabilities changed but not reflected

**Incorrect Examples** (Medium severity):
- YAML example won't work
- Example uses deprecated syntax
- Example missing required fields

**Incomplete Explanation** (Medium severity):
- Feature mentioned but not fully explained
- No example provided for complex feature
- Use cases not clear

**Missing Best Practices** (Low severity):
- Common pattern not documented
- Gotcha not mentioned
- Optimization tip missing

**Unclear Content** (Low severity):
- Confusing explanation
- Poor organization
- Needs better examples

#### Step 5: Assess Chapter Quality

Overall chapter assessment:
- **Critical**: Multiple high severity issues, will cause user errors
- **High**: Several missing features or outdated information
- **Medium**: Incorrect examples or incomplete explanations
- **Low**: Minor issues, could be clearer
- **Good**: No significant drift, minor improvements only

### Phase 4: Create Drift Report

**Determine Output Path:**

- Pattern: `.prodigy/book-analysis/drift-$CHAPTER_ID.json`

Create drift report at the determined path:

```json
{
  "chapter_id": "$CHAPTER_ID",
  "chapter_title": "$CHAPTER_TITLE",
  "chapter_file": "$CHAPTER_FILE",
  "drift_detected": true,
  "severity": "high",
  "quality_assessment": "Chapter needs updates to reflect current MapReduce syntax",
  "issues": [
    {
      "type": "outdated_information",
      "severity": "high",
      "section": "Agent Template Syntax",
      "description": "Chapter shows only nested 'commands' format, not modern direct array format",
      "current_content": "agent_template:\n  commands:\n    - claude: /process",
      "should_be": "agent_template:\n  - claude: /process\n  - shell: test",
      "fix_suggestion": "Add section showing modern direct array syntax as primary, note nested format is deprecated",
      "source_reference": "src/config/mapreduce.rs:MapPhaseYaml::agent_template"
    },
    {
      "type": "missing_content",
      "severity": "medium",
      "section": "Capture Formats",
      "description": "Chapter doesn't document all capture formats available",
      "should_add": "Document capture_format: string, number, json, lines, boolean",
      "fix_suggestion": "Add subsection on output capture with examples for each format",
      "source_reference": "src/cook/workflow/variables.rs:CaptureFormat"
    },
    {
      "type": "incorrect_example",
      "severity": "medium",
      "section": "Example 3",
      "description": "YAML example uses deprecated 'test:' command",
      "current_content": "test: cargo test\n  on_failure: ...",
      "should_be": "shell: cargo test\n  on_failure: ...",
      "fix_suggestion": "Update example to use shell: instead of test:",
      "source_reference": "Deprecated in src/config/command.rs"
    }
  ],
  "positive_aspects": [
    "Clear introduction and motivation",
    "Good progression from simple to complex",
    "Helpful diagrams and explanations"
  ],
  "improvement_suggestions": [
    "Add more real-world examples",
    "Include common pitfalls section",
    "Add links to related chapters"
  ],
  "metadata": {
    "analyzed_at": "2025-01-XX",
    "feature_inventory": ".prodigy/book-analysis/features.json",
    "topics_covered": ["Setup phase", "Map phase", "Reduce phase"],
    "validation_focus": "$VALIDATION_FOCUS"
  }
}
```

### Phase 5: Quality Guidelines

#### Be User-Focused
- Think from reader's perspective
- Is the explanation clear?
- Are examples practical?
- Would a beginner understand this?

#### Check Accuracy
- Does the code actually work?
- Are field names correct?
- Are types accurate?
- Do examples parse?

#### Check Completeness
- Are all major features covered?
- Are important use cases shown?
- Are gotchas mentioned?
- Are alternatives explained?

#### Check Clarity
- Is the flow logical?
- Are concepts introduced in order?
- Are complex ideas broken down?
- Are examples well-explained?

#### Source References
- Link to specific struct/enum definitions
- Reference implementation files
- Help the fix command find details
- Include line numbers if helpful

### Phase 6: Commit the Drift Report

**CRITICAL**: Since each map agent runs in a separate git worktree, the drift JSON file MUST be committed to be accessible in the reduce phase.

**Determine Drift File Path:**
Use the same path logic as Phase 4 based on `--project` parameter.

```bash
PROJECT_DIR=".{project_lowercase}"
git add $PROJECT_DIR/book-analysis/drift-$CHAPTER_ID.json
git commit -m "analysis: drift report for $PROJECT_NAME book chapter '$CHAPTER_TITLE'

Drift severity: $SEVERITY
Issues found: $ISSUE_COUNT
Quality: $QUALITY_ASSESSMENT"
```

### Phase 7: Validation

The drift report should:
1. Accurately identify drift between book and code
2. Categorize issues by type and severity
3. Assess overall chapter quality
4. Provide actionable fix suggestions
5. Include source references
6. Note positive aspects to preserve
7. **Be committed to git** for reduce phase access
8. Use project name from `--project` parameter in messages
