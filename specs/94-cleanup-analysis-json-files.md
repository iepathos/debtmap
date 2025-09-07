---
number: 94
title: Cleanup Analysis JSON Files
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-09-07
---

# Specification 94: Cleanup Analysis JSON Files

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The project has accumulated large JSON analysis files (some over 730K lines) that have been committed to the repository. These files are generated during debtmap analysis runs and workflow executions. They contribute to repository bloat, may be detected as debt items themselves, and slow down git operations. Additionally, these files are regenerated frequently and should not be tracked in version control. Removing them and properly configuring `.gitignore` would reduce debt score by an estimated 20-40 points.

## Objective

Remove all generated analysis JSON files from the repository, prevent future commits of such files through proper `.gitignore` configuration, and establish a clean separation between source code and generated analysis artifacts.

## Requirements

### Functional Requirements
- Identify and remove all generated JSON analysis files from git history
- Update `.gitignore` to prevent future commits of analysis files
- Create designated directories for temporary analysis output
- Preserve any legitimate configuration JSON files
- Clean up workflow-generated temporary files

### Non-Functional Requirements
- Reduce repository size significantly
- Improve git operation performance
- Maintain clean separation of concerns
- Follow best practices for artifact management
- Ensure CI/CD workflows continue to function

## Acceptance Criteria

- [ ] All generated analysis JSON files removed from repository
- [ ] Repository size reduced by at least 20%
- [ ] `.gitignore` updated with comprehensive patterns
- [ ] No analysis artifacts in source directories
- [ ] CI/CD pipelines continue to work correctly
- [ ] Technical debt score reduced by at least 20 points
- [ ] Documentation updated with artifact handling guidelines

## Technical Details

### Files to Remove

1. **Analysis Output Files**:
   ```
   analysis.json
   debt_output.json
   debtmap.json
   debtmap-after.json
   debtmap_analysis.json
   debtmap_output.json
   item.json
   selected_item.json
   test_debt.json
   current_debt.json
   ```

2. **Workflow Artifacts**:
   ```
   workflows/*.json
   target/tarpaulin/debtmap-coverage.json
   analyze_debt_changes.py  # Also remove Python script
   ```

3. **Temporary Files**:
   ```
   *.tmp.json
   *-backup.json
   *-old.json
   ```

### GitIgnore Configuration

```gitignore
# Analysis outputs
/analysis.json
/debt_output.json
/debtmap*.json
/item.json
/selected_item.json
/test_debt.json
/current_debt.json

# Workflow artifacts
/workflows/*.json
/workflows/*.tmp
/workflows/*.log

# Analysis directories
/analysis-output/
/debt-reports/
/tmp/

# Python scripts (shouldn't exist in Rust project)
*.py
!scripts/*.py  # Exception for legitimate scripts if any

# Large generated files
*.json
!package.json
!Cargo.lock
!.vscode/*.json
!.github/**/*.json
!configs/*.json  # Preserve configuration files

# Coverage reports
/coverage/
/target/coverage/
*.lcov
*.profraw
*.profdata
tarpaulin-report.json
```

### Directory Structure

```
debtmap/
├── src/               # Source code only
├── tests/             # Test code only
├── analysis-output/   # Generated analysis (git ignored)
│   ├── reports/       # Debt reports
│   ├── metrics/       # Metrics data
│   └── temp/          # Temporary files
├── configs/           # Configuration files (tracked)
└── scripts/           # Utility scripts (tracked)
```

### Cleanup Process

1. **Immediate Cleanup**:
   ```bash
   # Remove files from repository
   git rm --cached analysis.json debt_output.json debtmap*.json
   git rm --cached item.json selected_item.json test_debt.json
   git rm --cached analyze_debt_changes.py
   
   # Clean working directory
   rm -f *.json
   rm -f workflows/*.json
   
   # Create analysis output directory
   mkdir -p analysis-output/{reports,metrics,temp}
   ```

2. **History Cleanup** (optional, for thorough cleanup):
   ```bash
   # Use BFG Repo Cleaner or git filter-branch
   bfg --delete-files '*.json' --no-blob-protection
   git reflog expire --expire=now --all
   git gc --prune=now --aggressive
   ```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - CI/CD workflows that may expect JSON files
  - Scripts that read analysis output
  - Documentation referencing file locations
- **External Dependencies**: 
  - May need BFG Repo Cleaner for history cleanup

## Testing Strategy

- **Verification Tests**: 
  - Ensure no JSON files in git status
  - Verify .gitignore patterns work correctly
  - Check repository size reduction
- **CI/CD Tests**: 
  - Verify workflows still function
  - Ensure analysis can write to new directories
- **Integration Tests**: 
  - Run full debtmap analysis
  - Verify output goes to correct location

## Documentation Requirements

- **Artifact Guidelines**: Document where analysis output should go
- **Developer Guide**: Update with new directory structure
- **CI/CD Documentation**: Update workflow documentation
- **.gitignore Comments**: Add explanatory comments in .gitignore

## Implementation Notes

1. **Staged Approach**:
   - Stage 1: Update .gitignore and test
   - Stage 2: Remove files from current commit
   - Stage 3: Clean working directories
   - Stage 4: (Optional) Clean git history

2. **Workflow Updates**:
   ```yaml
   # Update workflows to use analysis-output directory
   - name: Run debtmap analysis
     run: |
       mkdir -p analysis-output/reports
       debtmap analyze src --output analysis-output/reports/debt.json
   ```

3. **Preservation List**:
   - Keep `Cargo.lock` (legitimate)
   - Keep `.vscode/settings.json` (IDE config)
   - Keep any `configs/*.json` (app configuration)

4. **Monitoring**:
   - Set up pre-commit hook to prevent large JSON commits
   - Add CI check for repository size
   - Monitor for accidental JSON commits

## Migration and Compatibility

- Update all scripts to use new output directories
- Modify CI/CD workflows to create directories as needed
- Update documentation with new file locations
- Provide migration script for existing setups