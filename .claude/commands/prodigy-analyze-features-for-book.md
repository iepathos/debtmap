# /prodigy-analyze-features-for-book

Perform comprehensive analysis of a codebase to identify features and capabilities that should be documented in the book.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy", "Debtmap")
- `--config <path>` - Path to book configuration JSON (e.g., ".prodigy/book-config.json")

## Execute

### Phase 1: Understand Context

You are analyzing a codebase to create a comprehensive feature inventory for the documentation book. This will be used to detect drift between the book documentation and actual implementation.

**Parse Parameters:**
Extract the project name and configuration path from the command arguments:
- `--project`: The project name (used in output messages and file paths)
- `--config`: Path to the book configuration JSON file

**Load Configuration:**
Read the configuration file specified by `--config` to get:
- `project_name`: Display name of the project
- `analysis_targets`: Areas to analyze with source files and feature categories
- `book_dir`, `book_src`: Book directory paths
- `chapter_file`: Path to chapter definitions
- `custom_analysis`: Options for examples, best practices, troubleshooting

### Phase 2: Analyze Core Features

**Use Analysis Targets from Configuration:**

For each `analysis_target` in the configuration:
- Read the `source_files` specified for that area
- Extract features based on the `feature_categories`
- Focus on user-facing capabilities, not implementation details

**Analysis Strategy by Area:**

The configuration defines which areas to analyze (e.g., workflow_basics, mapreduce, command_types, etc.). For each area:

1. **Read Source Files**: Examine the files specified in `source_files`
2. **Parse Structures**: Extract struct definitions, enums, fields, and serde attributes
3. **Identify Capabilities**: What can users actually do with this feature?
4. **Find Examples**: Look in workflows/ and tests/ directories
5. **Document Patterns**: Common use cases and best practices

**Generic Feature Extraction:**

Instead of hardcoding "Prodigy workflow" or "Prodigy features":
- Use "codebase features" or "project capabilities"
- Reference the project name from `--project` parameter in output
- Extract features based on code structure, not assumptions
- Adapt analysis depth based on `custom_analysis` settings

### Phase 3: Create Feature Inventory

**IMPORTANT: You MUST create a JSON file using the Write tool. This is NOT optional.**

**Determine Output Path:**
Based on the project configuration:
- Extract `book_dir` from config (defaults to "book")
- Create analysis directory adjacent to book: `.{project_lowercase}/book-analysis/`
- For Prodigy: `.prodigy/book-analysis/features.json`
- For Debtmap: `.prodigy/book-analysis/features.json`

**Action Required:**
Use the Write tool to create a JSON file at the determined path with this structure:

```json
{
  "workflow_basics": {
    "structure": {
      "simple_array": "Direct command array",
      "full_config": "With env, secrets, profiles"
    },
    "execution_model": "Sequential command execution",
    "commit_tracking": "Git integration for audit trail"
  },
  "mapreduce": {
    "phases": ["setup", "map", "reduce"],
    "capabilities": {
      "parallel_execution": true,
      "work_distribution": "Automatic across agents",
      "result_aggregation": "In reduce phase",
      "checkpoint_resume": true
    },
    "configuration": {
      "setup": ["commands", "timeout", "capture_outputs"],
      "map": ["input", "json_path", "agent_template", "max_parallel", "filter", "sort_by"],
      "reduce": ["commands", "aggregation"]
    }
  },
  "command_types": {
    "shell": {
      "description": "Execute shell commands",
      "common_fields": ["shell", "timeout", "capture", "on_failure"],
      "use_cases": ["Build", "Test", "Deploy", "Data processing"]
    },
    "claude": {
      "description": "Execute Claude AI commands",
      "common_fields": ["claude", "commit_required", "validate"],
      "use_cases": ["Code generation", "Analysis", "Refactoring"]
    },
    "goal_seek": {
      "description": "Iterative refinement to reach quality threshold",
      "fields": ["goal", "validate", "threshold", "max_attempts"],
      "use_cases": ["Coverage improvement", "Performance optimization"]
    },
    "foreach": {
      "description": "Iterate over lists with optional parallelism",
      "fields": ["input", "do", "parallel", "continue_on_error"],
      "use_cases": ["File processing", "Batch operations"]
    }
  },
  "variables": {
    "standard": {
      "shell.output": "Last shell command output",
      "claude.output": "Last Claude command output",
      "last.output": "Last command output (any type)",
      "last.exit_code": "Exit code from last command"
    },
    "mapreduce": {
      "item": "Current work item in map phase",
      "item.*": "Access item fields with wildcard",
      "map.total": "Total items processed",
      "map.successful": "Successfully processed items",
      "map.failed": "Failed items",
      "map.results": "Aggregated results"
    },
    "validation": {
      "validation.completion": "Completion percentage",
      "validation.gaps": "Missing requirements",
      "validation.status": "Status (complete/incomplete/failed)"
    }
  },
  "environment": {
    "global_env": "Static and dynamic variables",
    "secrets": "Masked in logs, supports providers",
    "profiles": "Environment-specific configurations",
    "step_env": "Command-level overrides"
  },
  "advanced_features": {
    "conditional_execution": "when: expression",
    "output_capture": ["string", "number", "json", "lines", "boolean"],
    "nested_handlers": "on_success, on_failure, on_exit_code",
    "timeout_control": "Command and workflow level",
    "working_directory": "Per-command cwd control"
  },
  "error_handling": {
    "workflow_level": {
      "on_item_failure": ["dlq", "retry", "skip", "stop"],
      "error_collection": ["aggregate", "immediate", "batched"],
      "circuit_breaker": true,
      "max_failures": "Stop after N failures"
    },
    "command_level": {
      "on_failure": "Nested command execution",
      "on_success": "Success handlers",
      "on_exit_code": "Map exit codes to actions",
      "retry_config": "With exponential backoff"
    }
  },
  "best_practices": {
    "workflow_design": [
      "Keep workflows simple and focused",
      "Use validation for quality gates",
      "Handle errors gracefully",
      "Capture important outputs"
    ],
    "mapreduce": [
      "Set appropriate parallelism",
      "Use DLQ for failed items",
      "Monitor with events",
      "Design idempotent work items"
    ],
    "testing": [
      "Include test steps in workflows",
      "Use on_failure for debugging",
      "Validate before deploying"
    ]
  },
  "common_patterns": [
    {
      "name": "Build and Test",
      "description": "Standard CI workflow",
      "example": "workflows/examples/build-test.yml"
    },
    {
      "name": "Parallel Processing",
      "description": "MapReduce for independent items",
      "example": "workflows/examples/parallel-review.yml"
    },
    {
      "name": "Goal Seeking",
      "description": "Iterative improvement to threshold",
      "example": "workflows/examples/coverage-improvement.yml"
    }
  ],
  "troubleshooting": {
    "common_issues": [
      {
        "issue": "Variables not interpolating",
        "solution": "Check ${} syntax and variable availability"
      },
      {
        "issue": "MapReduce items not found",
        "solution": "Verify JSONPath expression"
      },
      {
        "issue": "Timeout errors",
        "solution": "Increase timeout or optimize commands"
      }
    ]
  },
  "version_info": {
    "analyzed_version": "0.2.0+",
    "analysis_date": "2025-01-XX"
  }
}
```

### Phase 4: Analysis Method

1. **Read Source Files**: Examine all key implementation files from `analysis_targets`
2. **Parse Struct Definitions**: Extract fields, types, serde attributes
3. **Identify Capabilities**: What can users actually do?
4. **Find Examples**: Look in workflows/ and tests/ directories
5. **Document Patterns**: Common use cases and best practices
6. **Use Generic Language**: Avoid project-specific terminology in feature descriptions

### Phase 5: Quality Guidelines

- Focus on user-facing features, not implementation details
- Document capabilities, not just configuration options
- Include practical use cases for each feature
- Note common pitfalls and solutions
- Provide realistic examples
- Keep language accessible for book audience
- Use project name from `--project` parameter in output messages
- Adapt analysis based on `custom_analysis` configuration

### Phase 6: Validation

The features.json file should:
1. Cover all major feature areas defined in `analysis_targets`
2. Include practical use cases
3. Provide examples for each capability
4. Document common patterns
5. Include troubleshooting guidance (if `custom_analysis.include_troubleshooting` is true)
6. Be user-focused, not developer-focused
7. Be project-agnostic (work for any codebase with proper configuration)

### Phase 7: Commit the Changes

**CRITICAL: This step requires a commit to be created.**

After creating the features.json file:
1. Add the file to git: `git add .{project_lowercase}/book-analysis/features.json`
2. Create a commit with a descriptive message:
   ```
   chore: analyze {project_name} features for book documentation

   Generated comprehensive feature inventory covering:
   - [List key areas analyzed]

   This analysis will be used to detect documentation drift.
   ```
3. Verify the commit was created successfully with `git log -1`
