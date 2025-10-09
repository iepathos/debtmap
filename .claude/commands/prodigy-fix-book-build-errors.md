# /prodigy-fix-book-build-errors

Fix mdBook build errors in the Prodigy documentation book.

## Variables

None - analyzes build output and fixes errors.

## Execute

### Phase 1: Understand Context

The mdBook build failed. You need to identify the errors, understand their causes, and fix them.

### Phase 2: Analyze Build Output

The build output will show errors like:

**Common mdBook Errors:**

#### 1. Missing Files
```
Error: Unable to open file 'book/src/missing-chapter.md'
```
**Cause**: SUMMARY.md references a file that doesn't exist
**Fix**: Either create the file or remove the reference from SUMMARY.md

#### 2. Broken Links
```
Warning: [book/src/intro.md] Potential incomplete link
```
**Cause**: Markdown link syntax error or broken reference
**Fix**: Check link syntax and ensure target exists

#### 3. Invalid SUMMARY.md
```
Error: Failed to parse SUMMARY.md
```
**Cause**: Incorrect markdown list syntax in SUMMARY.md
**Fix**: Ensure proper indentation and list formatting

#### 4. Duplicate Chapter Names
```
Error: Duplicate chapter name 'Introduction'
```
**Cause**: Multiple chapters with same title
**Fix**: Make chapter titles unique

#### 5. Invalid Code Blocks
```
Warning: [book/src/commands.md] Code block not properly closed
```
**Cause**: Missing closing ``` for code block
**Fix**: Add closing ``` or fix syntax

#### 6. Invalid YAML in Examples
While mdBook may not validate YAML, readers will:
**Cause**: Syntax errors in YAML examples
**Fix**: Validate and fix YAML examples

### Phase 3: Systematic Error Resolution

#### Step 1: Read Build Error Output

Capture the full error output:
```bash
cd book && mdbook build 2>&1 | tee .prodigy/book-build-errors.log
```

#### Step 2: Categorize Errors

Parse errors into categories:
- **Critical**: Build fails completely
- **Warnings**: Build succeeds but issues found
- **Validation**: Content issues that may affect readers

#### Step 3: Fix Critical Errors First

##### Missing File Errors
1. Check SUMMARY.md for referenced files
2. For each missing file, decide:
   - Create placeholder file with basic structure
   - Remove reference if not needed
   - Fix typo in filename

##### SUMMARY.md Syntax Errors
1. Read `book/src/SUMMARY.md`
2. Check for:
   - Proper markdown list syntax (`-` or `*`)
   - Correct indentation (multiples of 2 or 4 spaces)
   - Matching brackets in links `[Title](file.md)`
   - File paths relative to src/ directory

##### Broken Internal Links
1. Find all internal links in chapter files
2. Verify target files exist
3. Fix paths (relative to src/ directory)
4. Update link text if needed

#### Step 4: Fix Warnings

##### Incomplete Links
Check for:
- Missing closing `]` or `)`
- Typos in filenames
- Wrong relative paths
- Anchors to non-existent headings

##### Code Block Issues
1. Find unclosed code blocks (missing ```)
2. Check language tags are valid
3. Ensure proper indentation

#### Step 5: Content Validation

##### Validate YAML Examples
For each YAML example in the book:
1. Extract the YAML
2. Parse it to check syntax
3. Fix any errors:
   - Indentation issues
   - Missing quotes
   - Invalid field names
   - Type mismatches

##### Check Cross-References
1. Verify chapter references are accurate
2. Check section anchors exist
3. Update outdated references

### Phase 4: Common Fixes

#### Fix SUMMARY.md Structure
```markdown
# Summary

[Introduction](intro.md)

# User Guide

- [Workflow Basics](workflow-basics.md)
- [MapReduce Workflows](mapreduce.md)
  - [Setup Phase](mapreduce/setup.md)
  - [Map Phase](mapreduce/map.md)

# Advanced Topics

- [Advanced Features](advanced.md)
```

**Rules:**
- Headings: `# Section Name` (not list items)
- Chapters: `- [Title](file.md)`
- Sub-chapters: Indent 2 spaces: `  - [Title](file.md)`
- Paths relative to src/: `file.md` not `src/file.md`

#### Fix Broken Links
```markdown
<!-- ❌ Wrong -->
[See variables](../variables.md)
[See variables](src/variables.md)

<!-- ✅ Correct -->
[See variables](variables.md)
[See variables](./variables.md)
```

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
```←- This needs to be on its own line
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
cd book && mdbook clean

# Rebuild
mdbook build

# Check exit code
if [ $? -eq 0 ]; then
  echo "✓ Book builds successfully!"
else
  echo "✗ Build still failing, checking errors..."
  mdbook build 2>&1 | head -20
fi
```

### Phase 6: Test Book Output

If build succeeds:

1. **Check Chapter Navigation**:
   - All chapters appear in sidebar
   - Chapter order is correct
   - Sub-chapters are properly nested

2. **Check Links**:
   - Internal links work
   - Code examples are readable
   - No broken references

3. **Check Formatting**:
   - Code blocks render properly
   - Tables display correctly
   - Lists are formatted well

### Phase 7: Document Fixes

Create a summary of fixes applied:

```markdown
# Book Build Error Fixes

## Errors Fixed

### Critical Errors
- Fixed SUMMARY.md indentation for sub-chapters
- Created missing chapter files: {list}
- Fixed broken internal links: {count}

### Warnings Resolved
- Closed unclosed code blocks: {count}
- Fixed incomplete links: {count}
- Corrected YAML syntax in examples: {count}

## Files Modified
- book/src/SUMMARY.md
- {list other files}

## Validation
- ✓ Book builds successfully
- ✓ All chapters accessible
- ✓ No broken links
- ✓ All code blocks render correctly
```

### Phase 8: Commit Fixes

```bash
git add book/
git commit -m "fix: resolve mdbook build errors

Fixed {N} critical errors and {N} warnings:
- {Summary of main fixes}
- {Summary of main fixes}

Book now builds successfully ✓"
```

### Phase 9: Troubleshooting

#### If Build Still Fails

1. **Check mdBook Version**:
   ```bash
   mdbook --version
   ```
   Ensure compatible version installed

2. **Validate book.toml**:
   - Check TOML syntax
   - Verify paths are correct
   - Check configuration options

3. **Check File Encoding**:
   - Ensure UTF-8 encoding
   - No BOM (Byte Order Mark)
   - Unix line endings (LF not CRLF)

4. **Isolate Problem**:
   - Comment out chapters in SUMMARY.md one by one
   - Identify which chapter causes failure
   - Focus on that chapter's content

#### If Warnings Persist

Some warnings are acceptable:
- Minor link warnings that don't affect functionality
- Style suggestions from mdBook
- Informational messages

Focus on errors that break the build or affect readers.

### Phase 10: Quality Checks

After successful build:

1. **Visual Inspection**:
   ```bash
   cd book && mdbook serve
   ```
   Open browser and check:
   - Navigation works
   - Code examples render well
   - No obvious formatting issues

2. **Link Validation**:
   Check all internal links manually or with tool

3. **Content Check**:
   Ensure fixes didn't break content or remove important information

### Phase 11: Prevention

To prevent future build errors:

1. **Use Consistent Formatting**:
   - Always close code blocks
   - Use consistent heading levels
   - Follow mdBook conventions

2. **Validate Before Committing**:
   - Run `mdbook build` before commits
   - Check for warnings
   - Test navigation

3. **Use Relative Links**:
   - Keep links relative to src/
   - Avoid absolute paths
   - Test links after moving files

4. **Keep SUMMARY.md Updated**:
   - Add new chapters immediately
   - Remove deleted chapters
   - Maintain proper indentation
