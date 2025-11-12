# /prodigy-analyze-features-for-mkdocs

Perform comprehensive analysis of a codebase to identify features and capabilities that should be documented in MkDocs Material documentation.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy")
- `--config <path>` - Path to MkDocs configuration JSON (e.g., ".prodigy/mkdocs-config.json")

## Execute

### Phase 1: Understand Context

You are analyzing a codebase to create a comprehensive feature inventory for MkDocs Material documentation. This will be used to detect drift between the docs and actual implementation.

**Parse Parameters:**
Extract the project name and configuration path from the command arguments:
- `--project`: The project name (used in output messages and file paths)
- `--config`: Path to the MkDocs configuration JSON file

**Load Configuration:**
Read the configuration file specified by `--config` to get:
- `project_name`: Display name of the project
- `analysis_targets`: Areas to analyze with source files and feature categories
- `docs_dir`: Documentation directory path (typically "docs")
- `mkdocs_config`: Path to mkdocs.yml
- `chapters_file`: Path to MkDocs chapter/page definitions
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

Instead of hardcoding project-specific terms:
- Use "codebase features" or "project capabilities"
- Reference the project name from `--project` parameter in output
- Extract features based on code structure, not assumptions
- Adapt analysis depth based on `custom_analysis` settings

### Phase 3: Create Feature Inventory

**IMPORTANT: You MUST create a JSON file using the Write tool. This is NOT optional.**

**Determine Output Path:**
Based on the project configuration:
- Extract `docs_dir` from config (defaults to "docs")
- Create analysis directory: `.prodigy/mkdocs-analysis/`
- Write to: `.prodigy/mkdocs-analysis/features.json`

**Action Required:**
Analyze the codebase to discover its features, then create a JSON file at the determined path with a hierarchical structure.

**Structure Requirements:**

1. **Top-level features** should have `type: "major_feature"`
2. **Meta-content** (best practices, troubleshooting, examples) should have `type: "meta"`
3. **Capabilities** should be nested under their parent features
4. **Use 2-3 levels maximum** to avoid over-fragmentation

**Discovery Process:**

1. Read the `analysis_targets` from mkdocs-config.json
2. For each analysis target:
   - Read the source files specified
   - Identify the main user-facing capabilities
   - Group related capabilities together
   - Determine if this is a major feature or meta-content

3. Create feature hierarchy:
   - **Major features**: Core capabilities users interact with directly
   - **Nested capabilities**: Sub-features that belong under a major feature
   - **Meta-content**: Best practices, troubleshooting, examples (reference sections)

**Example Structure** (adapt based on what you discover in the codebase):

```json
{
  "feature_name_1": {
    "type": "major_feature",
    "description": "Brief description of what this feature does",
    "capabilities": {
      "capability_1": "Description",
      "capability_2": "Description"
    }
  },
  "feature_name_2": {
    "type": "major_feature",
    "description": "Complex feature with multiple aspects",
    "phases": {
      "phase_1": {
        "description": "First phase description",
        "capabilities": ["cap1", "cap2"]
      },
      "phase_2": {
        "description": "Second phase description",
        "capabilities": ["cap3", "cap4"]
      }
    },
    "core_capabilities": {
      "capability_name": {
        "description": "What this capability enables",
        "features": ["feature1", "feature2"]
      }
    }
  },
  "meta_content": {
    "type": "meta",
    "description": "Cross-cutting content for reference section",
    "best_practices": {
      "category_1": ["practice1", "practice2"],
      "category_2": ["practice1", "practice2"]
    },
    "common_patterns": [
      {
        "name": "Pattern Name",
        "description": "Pattern description",
        "example": "path/to/example"
      }
    ],
    "troubleshooting": {
      "common_issues": [
        {
          "issue": "Problem description",
          "solution": "How to fix"
        }
      ]
    }
  },
  "version_info": {
    "analyzed_version": "extracted from codebase",
    "analysis_date": "current date"
  }
}
```

**Important Guidelines:**

- Feature names should be generic (e.g., "parallel_processing" not "prodigy_mapreduce")
- Descriptions should focus on WHAT users can do, not HOW it's implemented
- Group features logically based on user workflows
- Limit nesting to 2-3 levels maximum
- Mark meta-content explicitly with `type: "meta"`
- Include only user-facing features (no internal implementation details)

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
- Keep language accessible for documentation audience
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

1. Stage the features.json file for commit
2. Create a commit with this descriptive message format:
   ```
   chore: analyze {project_name} features for MkDocs documentation

   Generated comprehensive feature inventory covering:
   - [List the major feature areas you analyzed, e.g., "workflow_basics, mapreduce, command_types"]

   This analysis will be used to detect documentation drift.
   ```
3. Verify the commit was created successfully by checking the git log
