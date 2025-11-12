# /prodigy-fix-mkdocs-build-errors

Fix MkDocs build errors in the Prodigy documentation.

## Variables

- `--project <name>` - Project name (default: "Prodigy")

## Execute

### Phase 1: Understand Context

The MkDocs build failed. You need to identify the errors, understand their causes, and fix them.

### Phase 2: Analyze Build Output

The build output will show errors like:

**Common MkDocs Errors:**

#### 1. Missing Files
```
Error: Doc file 'missing-chapter.md' contains a link to 'other-file.md' which is not part of the site.
```
**Cause**: mkdocs.yml references a file that doesn't exist, or internal links point to non-existent files
**Fix**: Either create the file, add it to nav in mkdocs.yml, or remove the reference

#### 2. Broken Links (Strict Mode)
```
WARNING - Doc file 'intro.md' contains a link to 'nonexistent.md', which is not found in the documentation files.
```
**Cause**: Markdown link references file not in navigation or doesn't exist
**Fix**: Check link syntax and ensure target exists and is in mkdocs.yml nav

#### 3. Invalid mkdocs.yml
```
Error: Config file 'mkdocs.yml' is invalid.
YAML error: mapping values are not allowed here
```
**Cause**: YAML syntax error in mkdocs.yml
**Fix**: Validate YAML syntax, check indentation and formatting

#### 4. Duplicate Navigation Entries
```
Warning: A source file has been specified multiple times in the navigation
```
**Cause**: Same file appears multiple times in nav structure
**Fix**: Remove duplicate entries or use different sections

#### 5. Invalid Code Blocks
```
Warning: [mkdocs/commands.md] Code block not properly closed
```
**Cause**: Missing closing ``` for code block
**Fix**: Add closing ``` or fix syntax

#### 6. Invalid Admonitions
```
Error: Admonition not properly formatted
```
**Cause**: MkDocs admonition syntax errors (if using admonition extension)
**Fix**: Ensure proper `!!! note` or `??? warning` formatting

### Phase 3: Systematic Error Resolution

#### Step 1: Read Build Error Output

Capture the full error output:
```bash
mkdocs build --strict 2>&1 | tee .prodigy/mkdocs-build-errors.log
```

The `--strict` flag turns warnings into errors, which is what the workflow uses.

#### Step 2: Categorize Errors

Parse errors into categories:
- **Critical**: Build fails completely
- **Warnings**: Build succeeds but issues found (failures in strict mode)
- **Validation**: Content issues that may affect readers

#### Step 3: Fix Critical Errors First

##### Missing File Errors
1. Check mkdocs.yml nav structure for referenced files
2. For each missing file, decide:
   - Create placeholder file with basic structure
   - Remove reference if not needed
   - Fix typo in filename

##### mkdocs.yml Syntax Errors
1. Read `mkdocs.yml`
2. Check for:
   - Valid YAML syntax
   - Proper indentation (2 spaces per level)
   - Correct nav structure format
   - File paths relative to docs_dir (default: mkdocs/)

Example valid nav structure:
```yaml
nav:
  - Home: index.md
  - Getting Started: getting-started.md
  - User Guide:
      - Workflow Basics: workflow-basics.md
      - MapReduce: mapreduce/index.md
  - Advanced:
      - Overview: advanced/index.md
      - Features: advanced/features.md
```

##### Broken Internal Links
1. Find all internal links in markdown files
2. Verify target files exist
3. Ensure target files are in mkdocs.yml nav (required in strict mode)
4. Fix paths (relative to current file's location)
5. Update link text if needed

#### Step 4: Fix Warnings (Strict Mode)

##### Incomplete Links
Check for:
- Missing closing `]` or `)`
- Typos in filenames
- Wrong relative paths
- Links to files not in navigation

##### Code Block Issues
1. Find unclosed code blocks (missing ```)
2. Check language tags are valid
3. Ensure proper indentation

##### Admonition Issues (if using admonitions)
```markdown
<!-- ❌ Wrong -->
!!! note
This is wrong indentation

<!-- ✅ Correct -->
!!! note
    This is properly indented with 4 spaces

    Multiple paragraphs need to maintain indentation.
```

#### Step 5: Content Validation

##### Validate YAML Examples
For each YAML example in the docs:
1. Extract the YAML
2. Parse it to check syntax
3. Fix any errors:
   - Indentation issues
   - Missing quotes
   - Invalid field names
   - Type mismatches

##### Check Cross-References
1. Verify page references are accurate
2. Check section anchors exist
3. Update outdated references

### Phase 4: Common Fixes

#### Fix mkdocs.yml Structure
```yaml
site_name: Prodigy Documentation
docs_dir: mkdocs

nav:
  - Home: index.md
  - Getting Started: getting-started.md
  - User Guide:
      - Workflow Basics: workflow-basics.md
      - Environment Variables: environment.md
      - MapReduce:
          - Overview: mapreduce/index.md
          - Setup Phase: mapreduce/setup.md
          - Map Phase: mapreduce/map.md
  - Advanced Topics:
      - Overview: advanced/index.md
      - Features: advanced/features.md
```

**Rules:**
- Use 2-space indentation
- File paths relative to docs_dir
- Nested navigation uses YAML dict/list structure
- Each nav item is either `Title: file.md` or `Title:` with nested items

#### Fix Broken Links
```markdown
<!-- ❌ Wrong - absolute paths don't work in MkDocs -->
[See variables](/variables.md)

<!-- ❌ Wrong - incorrect relative path -->
[See variables](../variables.md)  # When both files are in same directory

<!-- ✅ Correct - relative to current file -->
[See variables](variables.md)
[See variables](./variables.md)
[See subsection](mapreduce/setup.md)  # Link to subdirectory
[See parent](../index.md)  # Link to parent directory
```

**MkDocs Link Resolution Rules:**
- Links are relative to the current file's location
- Use `./` for same directory (optional)
- Use `../` for parent directory
- All linked files must be in mkdocs.yml nav (in strict mode)

#### Fix Code Blocks
```markdown
<!-- ❌ Wrong -->
```yaml
workflow:
  - shell: "test"

<!-- Missing closing ``` -->

<!-- ✅ Correct -->
```yaml
workflow:
  - shell: "test"
```
```

#### Create Placeholder Files
When creating missing files:
```markdown
# {Chapter Title}

> **Note**: This chapter is under development.

{Brief description of what will be covered}

## Coming Soon

This chapter will cover:
- {Topic 1}
- {Topic 2}
- {Topic 3}

For now, see [Related Chapter](related.md) for more information.
```

### Phase 5: Verify Fixes

After applying fixes:

```bash
# Clean build
rm -rf site/

# Rebuild with strict mode (same as workflow)
mkdocs build --strict

# Check exit code
if [ $? -eq 0 ]; then
  echo "✓ MkDocs builds successfully!"
else
  echo "✗ Build still failing, checking errors..."
  mkdocs build --strict 2>&1 | head -30
fi
```

### Phase 6: Test Documentation Output

If build succeeds:

1. **Check Navigation**:
   - All pages appear in nav
   - Page order is correct
   - Nested pages are properly organized

2. **Check Links**:
   - Internal links work
   - Code examples are readable
   - No broken references

3. **Check Formatting**:
   - Code blocks render properly
   - Admonitions display correctly (if used)
   - Tables display correctly
   - Lists are formatted well

4. **Check Search** (if enabled):
   - Search index builds
   - Pages are searchable

### Phase 7: Document Fixes

Create a summary of fixes applied:

```markdown
# MkDocs Build Error Fixes

## Errors Fixed

### Critical Errors
- Fixed mkdocs.yml indentation for nested navigation
- Created missing page files: {list}
- Fixed broken internal links: {count}
- Added missing pages to navigation: {list}

### Warnings Resolved (Strict Mode)
- Closed unclosed code blocks: {count}
- Fixed incomplete links: {count}
- Corrected YAML syntax in examples: {count}
- Fixed admonition formatting: {count}

## Files Modified
- mkdocs.yml
- {list other files}

## Validation
- ✓ MkDocs builds successfully (strict mode)
- ✓ All pages accessible
- ✓ No broken links
- ✓ All code blocks render correctly
```

### Phase 8: Commit Fixes

```bash
git add mkdocs/ mkdocs.yml
git commit -m "fix: resolve mkdocs build errors

Fixed {N} critical errors and {N} warnings:
- {Summary of main fixes}
- {Summary of main fixes}

MkDocs now builds successfully in strict mode ✓"
```

### Phase 9: Troubleshooting

#### If Build Still Fails

1. **Check MkDocs Version**:
   ```bash
   mkdocs --version
   ```
   Ensure compatible version installed

2. **Validate mkdocs.yml**:
   ```bash
   # Test YAML syntax
   python -c "import yaml; yaml.safe_load(open('mkdocs.yml'))"
   ```
   - Check YAML syntax
   - Verify paths are correct
   - Check configuration options

3. **Check File Encoding**:
   - Ensure UTF-8 encoding
   - No BOM (Byte Order Mark)
   - Unix line endings (LF not CRLF)

4. **Isolate Problem**:
   - Comment out nav sections in mkdocs.yml one by one
   - Identify which section causes failure
   - Focus on those files' content

5. **Check Extensions**:
   If using MkDocs extensions, verify they're installed:
   ```bash
   pip list | grep mkdocs
   ```

#### If Warnings Persist in Strict Mode

In `--strict` mode, ALL warnings become errors. Common strict mode issues:

1. **Links to files not in nav**: Add files to mkdocs.yml or remove links
2. **Duplicate files in nav**: Remove duplicates
3. **Invalid admonitions**: Fix syntax or disable extension
4. **Missing meta tags**: Add required frontmatter

### Phase 10: Quality Checks

After successful build:

1. **Visual Inspection**:
   ```bash
   mkdocs serve
   ```
   Open browser to http://127.0.0.1:8000 and check:
   - Navigation works
   - Code examples render well
   - No obvious formatting issues
   - Search works (if enabled)

2. **Link Validation**:
   All internal links should work in strict mode build

3. **Content Check**:
   Ensure fixes didn't break content or remove important information

### Phase 11: Prevention

To prevent future build errors:

1. **Use Consistent Formatting**:
   - Always close code blocks
   - Use consistent heading levels
   - Follow MkDocs conventions

2. **Validate Before Committing**:
   - Run `mkdocs build --strict` before commits
   - Check for warnings
   - Test navigation

3. **Use Relative Links**:
   - Keep links relative to current file
   - Avoid absolute paths
   - Test links after moving files

4. **Keep mkdocs.yml Updated**:
   - Add new pages to nav immediately
   - Remove deleted pages from nav
   - Maintain proper YAML indentation (2 spaces)

5. **Test Locally**:
   - Use `mkdocs serve` to preview changes
   - Check navigation and links before committing
   - Verify strict mode build passes

### Common MkDocs vs mdBook Differences

If migrating from mdBook:

1. **Navigation Structure**:
   - mdBook: `SUMMARY.md` (Markdown)
   - MkDocs: `mkdocs.yml` nav (YAML)

2. **File Paths**:
   - mdBook: Relative to `src/`
   - MkDocs: Relative to `docs_dir` (configurable)

3. **Link Resolution**:
   - mdBook: Relative to `src/`
   - MkDocs: Relative to current file

4. **Build Command**:
   - mdBook: `mdbook build`
   - MkDocs: `mkdocs build` (use `--strict` for validation)

5. **Strict Mode**:
   - mdBook: Warnings don't fail build
   - MkDocs: `--strict` turns warnings into errors
