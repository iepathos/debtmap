# /prodigy-analyze-workflow-features

Perform comprehensive analysis of Prodigy codebase to extract all supported workflow syntax features.

## Variables

None required - analyzes codebase and creates .prodigy/syntax-analysis/features.json

## Execute

### Phase 1: Understand Context

You are analyzing the Prodigy workflow orchestration tool to document all currently supported workflow syntax features. This analysis will be used to detect drift between documentation and actual implementation.

### Phase 2: Analyze Command Types
Analyze all command types and their configurations:

**Key Files:**
- `src/config/command.rs` - WorkflowStepCommand, WorkflowCommand enums
- `src/cook/workflow/executor.rs` - WorkflowStep struct, CommandType enum

**Extract:**
- All command type variants (shell, claude, goal_seek, foreach, etc.)
- All fields for each command type
- Required vs optional fields
- Default values
- Deprecated fields and their replacements

### 2. MapReduce Configuration
Analyze MapReduce workflow structure:

**Key Files:**
- `src/config/mapreduce.rs` - MapReduceWorkflowConfig, MapPhaseYaml, ReducePhaseYaml
- `src/cook/execution/mapreduce/` - Implementation details

**Extract:**
- Setup phase configuration
- Map phase configuration (input, json_path, agent_template, etc.)
- Reduce phase configuration
- Error policy options
- Merge workflow configuration

### 3. Variable Interpolation
Analyze all variable types and contexts:

**Key Files:**
- `src/cook/workflow/variables.rs` - VariableStore, CaptureFormat, CaptureStreams
- `src/cook/workflow/executor.rs` - WorkflowContext, variable interpolation

**Extract:**
- All standard variables (shell.output, claude.output, etc.)
- MapReduce variables (item, map.total, map.successful, etc.)
- Git context variables (step.files_changed, workflow.commits, etc.)
- Validation variables
- Merge variables
- Custom capture variable syntax

### 4. Capture Formats
Analyze output capture capabilities:

**Key Files:**
- `src/cook/workflow/variables.rs` - CaptureFormat enum, CaptureStreams struct

**Extract:**
- All capture formats (string, number, json, lines, boolean)
- Capture streams configuration (stdout, stderr, exit_code, success)
- Custom variable names
- Backward compatibility with capture_output

### 5. Validation Features
Analyze validation and completion checking:

**Key Files:**
- `src/cook/workflow/validation.rs` - ValidationConfig, OnIncompleteConfig

**Extract:**
- Validation configuration fields
- on_incomplete retry logic
- Validation result format
- Threshold and completion checking
- Array format support for multi-step validation

### 6. Goal-Seeking Features
Analyze iterative refinement capabilities:

**Key Files:**
- `src/cook/goal_seek/mod.rs` - GoalSeekConfig

**Extract:**
- Goal-seeking configuration fields
- Validation command requirements
- Score format expectations
- Threshold and max_attempts

### 7. Error Handling
Analyze error handling mechanisms:

**Key Files:**
- `src/cook/workflow/error_policy.rs` - WorkflowErrorPolicy, ItemFailureAction
- `src/config/command.rs` - TestDebugConfig, on_failure

**Extract:**
- Error policy configuration
- on_failure configuration
- Retry mechanisms
- DLQ (Dead Letter Queue) options
- Circuit breaker configurations

### 8. Environment Configuration
Analyze environment and secrets management:

**Key Files:**
- `src/cook/environment/` - Environment management
- `src/config/workflow.rs` - WorkflowConfig env fields

**Extract:**
- Environment variable configuration
- Secrets management
- Environment profiles
- Dynamic environment variables

### Phase 3: Create Output

Create a JSON file at `.prodigy/syntax-analysis/features.json` with this structure:

```json
{
  "command_types": {
    "shell": {
      "fields": [
        {"name": "shell", "type": "string", "required": true, "description": "..."},
        {"name": "timeout", "type": "number", "required": false, "default": null, "description": "..."},
        ...
      ],
      "examples": [
        "shell: \"cargo test\"",
        "shell: \"cargo build\"\n  timeout: 300"
      ],
      "deprecated_fields": []
    },
    "claude": { /* same structure */ },
    "goal_seek": { /* same structure */ },
    "foreach": { /* same structure */ },
    "validation": { /* same structure */ }
  },
  "common_fields": [
    {"name": "id", "type": "string", "applies_to": ["all"], "description": "..."},
    {"name": "when", "type": "string", "applies_to": ["all"], "description": "..."},
    {"name": "commit_required", "type": "boolean", "applies_to": ["claude", "goal_seek"], "description": "..."},
    ...
  ],
  "variable_types": {
    "standard": ["shell.output", "claude.output", "handler.output", ...],
    "mapreduce": ["item", "item.*", "map.total", "map.successful", "map.failed", ...],
    "git_context": ["step.files_changed", "step.commits", "workflow.commits", ...],
    "validation": ["validation.completion", "validation.gaps", ...],
    "merge": ["merge.worktree", "merge.source_branch", ...]
  },
  "capture_formats": [
    {"format": "string", "description": "...", "example": "..."},
    {"format": "number", "description": "...", "example": "..."},
    {"format": "json", "description": "...", "example": "..."},
    {"format": "lines", "description": "...", "example": "..."},
    {"format": "boolean", "description": "...", "example": "..."}
  ],
  "capture_streams": {
    "fields": ["stdout", "stderr", "exit_code", "success"],
    "example": "capture_streams:\n  stdout: true\n  stderr: true"
  },
  "mapreduce_config": {
    "setup": {
      "formats": ["simple_array", "full_config"],
      "fields": [...],
      "capture_outputs_support": true
    },
    "map": {
      "required_fields": ["input", "agent_template"],
      "optional_fields": ["json_path", "filter", "sort_by", "max_items", ...],
      "agent_template_formats": ["simple_array", "nested_commands"]
    },
    "reduce": {
      "formats": ["simple_array", "nested_commands"],
      "fields": [...]
    },
    "merge": {
      "formats": ["simple_array", "full_config"],
      "fields": ["commands", "timeout"]
    }
  },
  "validation_features": {
    "validation_config": {
      "command_fields": ["shell", "claude", "commands"],
      "other_fields": ["threshold", "timeout", "result_file", "on_incomplete"],
      "array_format_support": true
    },
    "on_incomplete_config": {
      "fields": ["claude", "shell", "commands", "max_attempts", "fail_workflow", "commit_required"],
      "array_format_support": true
    }
  },
  "error_handling": {
    "workflow_level": {
      "on_item_failure": ["dlq", "retry", "skip", "stop", "custom"],
      "error_collection": ["aggregate", "immediate", "batched:N"],
      "other_fields": ["continue_on_failure", "max_failures", "failure_threshold"]
    },
    "command_level": {
      "on_failure_fields": ["claude", "shell", "max_attempts", "fail_workflow", "commit_required"],
      "on_success_support": true,
      "on_exit_code_support": true
    }
  },
  "deprecated_features": [
    {
      "feature": "test: command syntax",
      "replacement": "shell: with on_failure:",
      "deprecated_in": "0.1.9",
      "removed_in": null
    },
    {
      "feature": "command: in validation",
      "replacement": "shell:",
      "deprecated_in": "0.1.8",
      "removed_in": null
    },
    {
      "feature": "capture_output: true/false",
      "replacement": "capture: variable_name",
      "deprecated_in": "0.2.0",
      "removed_in": null
    }
  ],
  "version_info": {
    "analyzed_version": "0.2.0+",
    "analysis_date": "2025-01-XX"
  }
}
```

### Phase 4: Analysis Method

1. **Read Source Files**: Use the Read tool to examine all key files
2. **Parse Struct Definitions**: Extract all struct fields, types, and serde attributes
3. **Identify Defaults**: Look for `#[serde(default)]` and default functions
4. **Find Deprecations**: Look for deprecation warnings and comments
5. **Extract Examples**: Find examples in test files and example workflows
6. **Cross-Reference**: Ensure all features are connected to their implementations

### Phase 5: Quality Guidelines

- Be exhaustive - include ALL fields, even if they seem minor
- Note which fields are `#[serde(skip_serializing_if = "Option::is_none")]`
- Identify `#[serde(default)]` attributes
- Look for custom deserializers that support multiple formats (like untagged enums)
- Document both old (backward compatible) and new syntax
- Include version information when features were added/deprecated

### Phase 6: Validation

The features.json file should:
1. List ALL command types with ALL their fields
2. Include ALL variable types and contexts
3. Document ALL capture formats and stream options
4. Cover ALL error handling mechanisms
5. List ALL deprecated features with replacements
6. Provide accurate examples for each feature
