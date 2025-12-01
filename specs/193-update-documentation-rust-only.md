---
number: 193
title: Update Documentation for Rust-Only Focus
category: foundation
priority: high
status: draft
dependencies: [191, 192]
created: 2025-11-30
---

# Specification 193: Update Documentation for Rust-Only Focus

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: [191, 192]

## Context

After removing Python, JavaScript, and TypeScript support (specs 191-192), all documentation must be updated to reflect that debtmap is currently a Rust-only code analyzer. This includes README, architecture documentation, user guides, and any references to multi-language support.

## Objective

Update all documentation to clearly communicate that debtmap is currently Rust-focused, remove outdated multi-language references, and set appropriate expectations for future language support.

## Requirements

### Functional Requirements

- Update README.md to emphasize Rust-only support
- Update ARCHITECTURE.md to reflect Rust-only implementation
- Update book documentation pages
- Update CONTRIBUTING.md to focus on Rust analysis improvements
- Update CLI help text and error messages
- Add clear messaging about future multi-language plans
- Update examples and getting started guides to be Rust-focused

### Non-Functional Requirements

- Documentation must be accurate and up-to-date
- Tone should be positive about Rust focus, not apologetic
- Future plans should be realistic and achievable
- Links must not be broken
- Examples must work as documented

## Acceptance Criteria

- [ ] README.md updated with clear Rust-only messaging
- [ ] README.md roadmap section updated to show future language plans
- [ ] README.md "Multi-language Support" feature changed to "Rust-First Analysis"
- [ ] README.md removes Python/JavaScript/TypeScript examples
- [ ] README.md "Language Support" section accurately reflects current state
- [ ] README.md dependency licensing note updated (remove rustpython-parser reference)
- [ ] ARCHITECTURE.md updated to describe Rust-only architecture
- [ ] book/src/introduction.md updated for Rust focus
- [ ] book/src/getting-started.md has Rust-only installation and examples
- [ ] book/src/analysis-guide.md focuses on Rust metrics
- [ ] book/src/faq.md addresses "Why Rust-only?" question
- [ ] CONTRIBUTING.md updated to focus on Rust analysis contributions
- [ ] CLI help text (`--help`) mentions Rust-only support
- [ ] Error messages for unsupported languages are clear and helpful
- [ ] All code examples in documentation use Rust
- [ ] No broken links or references in updated documentation
- [ ] Changelog entry documenting the strategic shift

## Technical Details

### Implementation Approach

**Phase 1: Update README.md**

1. Update tagline/description:
   ```markdown
   # debtmap

   > **Beta Software** - Debtmap is a Rust code analyzer actively developed and
   tested in production. Core features are stable, though APIs may evolve as we
   add new capabilities. Contributions and feedback welcome!

   Debtmap is the premier Rust code analyzer that combines coverage-risk correlation
   with multi-factor analysis (complexity, dependencies, call graphs) and
   entropy-adjusted scoring to reduce false positives and prioritize testing efforts
   effectively.
   ```

2. Update "Why Debtmap?" section:
   - Emphasize Rust-specific features
   - Remove mentions of multi-language as a feature
   - Add "Rust-First Design" as a unique capability

3. Update "Key Features" section:
   - Change "Multi-language Support" to "Rust-First Analysis"
   - Update description: "Deep Rust analysis with macro expansion, trait resolution, and lifetime awareness"

4. Update "Language Support" section in Roadmap:
   ```markdown
   ### Language Support
   - [x] Rust - Full support with AST parsing, macro expansion, and trait resolution
   - [ ] Python - Planned after Rust analysis is mature
   - [ ] JavaScript/TypeScript - Planned after Rust analysis is mature
   - [ ] Go - Planned
   - [ ] C/C++ - Planned
   - [ ] C# - Planned
   - [ ] Java - Planned

   **Current Focus**: Perfecting Rust analysis before expanding to other languages.
   We're building the best Rust code analyzer first, then will apply those learnings
   to other languages.
   ```

5. Remove Python/JavaScript/TypeScript examples throughout

6. Update dependency licensing note:
   ```markdown
   ## License

   MIT License - see [LICENSE](LICENSE) file for details

   Debtmap has no restrictive dependencies - all dependencies are MIT, Apache-2.0,
   or similarly permissive licenses.
   ```

**Phase 2: Update Book Documentation**

1. Update `book/src/introduction.md`:
   ```markdown
   # Introduction

   Debtmap is a Rust code analyzer that helps you identify complex code,
   technical debt, and testing priorities through comprehensive static analysis
   and coverage correlation.

   ## Current Status

   Debtmap currently focuses exclusively on Rust codebases. Our goal is to
   perfect Rust analysis before expanding to other languages. This focused
   approach allows us to:

   - Build deep Rust-specific analysis (macro expansion, trait resolution, etc.)
   - Perfect our core algorithms and metrics
   - Establish a stable API and user experience
   - Build a strong user community

   Multi-language support is planned for future releases once Rust analysis
   reaches maturity.
   ```

2. Update `book/src/getting-started.md`:
   - Remove references to Python/JavaScript/TypeScript
   - Update installation instructions
   - Update all examples to use Rust code
   - Add note about Rust-only support

3. Update `book/src/analysis-guide.md`:
   - Focus entirely on Rust metrics
   - Remove multi-language pattern sections
   - Update examples to be Rust-specific

4. Update `book/src/faq.md`:
   Add FAQ entry:
   ```markdown
   ### Why is debtmap Rust-only right now?

   We're taking a focused approach to deliver the best possible Rust code
   analyzer before expanding to other languages. This strategy allows us to:

   1. **Perfect the core**: Get our algorithms, metrics, and UX right with one
      language before the complexity of multi-language support
   2. **Deep integration**: Build Rust-specific features like macro expansion,
      trait resolution, and lifetime analysis
   3. **Build trust**: Establish debtmap as the go-to Rust analyzer with a
      strong user base
   4. **Learn once, apply broadly**: Use our learnings from Rust to build
      better multi-language support later

   We will expand to other languages (Python, JavaScript/TypeScript, Go, etc.)
   once Rust analysis is mature and we have a stable foundation.

   ### Will other languages be supported in the future?

   Absolutely! Multi-language support is on the roadmap. We plan to add:

   - Python (high priority)
   - JavaScript/TypeScript (high priority)
   - Go, Java, C/C++, C# (medium priority)

   The timeline depends on Rust analysis maturity and community demand.
   Follow our [GitHub milestones](https://github.com/iepathos/debtmap/milestones)
   for updates.
   ```

**Phase 3: Update ARCHITECTURE.md**

1. Remove Python/JavaScript/TypeScript architecture sections
2. Update analyzer architecture to reflect Rust-only design
3. Update diagrams to show Rust-focused flow
4. Add section on future multi-language architecture plans

**Phase 4: Update CONTRIBUTING.md**

1. Focus contribution areas on Rust analysis:
   ```markdown
   ### Areas for Contribution
   - **Rust analysis depth** - Improve macro expansion, trait resolution, lifetime analysis
   - **New Rust metrics** - Implement additional Rust-specific complexity or quality metrics
   - **Rust patterns** - Detect more Rust idioms and anti-patterns
   - **Speed** - Optimize Rust analysis algorithms
   - **Documentation** - Improve docs and add Rust examples
   - **Testing** - Expand Rust analysis test coverage

   **Future work**: Multi-language support will be considered once Rust analysis
   is mature. If you're interested in Python, JavaScript, or other languages,
   please open an issue to discuss the roadmap.
   ```

**Phase 5: Update CLI and Error Messages**

1. Update CLI help text to mention Rust-only:
   ```rust
   /// Analyze Rust code for complexity and technical debt
   #[derive(Parser)]
   #[command(author, version, about, long_about = None)]
   pub struct Cli {
       // ... existing fields
   }
   ```

2. Update error messages for unsupported file types:
   ```rust
   if language != Language::Rust {
       return Err(anyhow::anyhow!(
           "Only Rust files are currently supported. \
            Debtmap is focusing on perfecting Rust analysis before expanding to other languages. \
            File: {}",
           path.display()
       ));
   }
   ```

**Phase 6: Update Examples and Tests**

1. Remove non-Rust example code from `examples/` directory if it exists
2. Update integration test documentation
3. Ensure all example commands use Rust projects

### Documentation Structure Changes

- Remove Python/JavaScript/TypeScript sections entirely
- Add "Rust-First Design" section highlighting Rust-specific capabilities
- Add clear roadmap for future language support
- Update all code examples to Rust

### Tone and Messaging

**Key Messages**:
1. Rust-first is a strategic advantage, not a limitation
2. Deep Rust analysis beats shallow multi-language support
3. Future multi-language support is planned and realistic
4. Community feedback shapes the roadmap

**Avoid**:
- Apologetic tone about lack of multi-language support
- Vague promises about "coming soon"
- Underselling Rust-specific capabilities

## Dependencies

- **Prerequisites**: Specs 191 and 192 must be completed first
- **Affected Components**:
  - README.md
  - ARCHITECTURE.md
  - book/ documentation
  - CONTRIBUTING.md
  - CLI help text
  - Error messages
- **External Dependencies**: None

## Testing Strategy

- **Manual Review**: Read through all updated documentation for accuracy
- **Link Checking**: Verify all links work
- **Example Validation**: Test all documented examples work as described
- **Build Checks**: Ensure `mdbook build` succeeds if using mdbook

## Documentation Requirements

- **Style Guide**: Follow existing documentation style and tone
- **Code Examples**: All examples must be tested and working
- **Links**: All links must be valid and not broken
- **Accuracy**: All technical claims must be accurate

## Implementation Notes

- Be positive about Rust focus - it's a feature, not a bug
- Set realistic expectations for future language support
- Keep removed content in git history for future reference
- Update CHANGELOG.md to document this strategic shift
- Consider adding a blog post or announcement explaining the decision

## Migration and Compatibility

**Breaking Changes**:
- Documentation now accurately reflects Rust-only support
- Users expecting multi-language support will need clarification

**Communication**:
- Update homepage/marketing materials to match documentation
- Prepare announcement for community channels
- Update GitHub repo description
- Update crates.io description

**Suggested Changelog Entry**:
```markdown
## [0.7.0] - YYYY-MM-DD

### Changed - Strategic Focus on Rust
- **BREAKING**: Removed Python, JavaScript, and TypeScript analysis support
- Debtmap now focuses exclusively on Rust code analysis
- All documentation updated to reflect Rust-first approach

### Rationale
To build the best Rust code analyzer, we're focusing development efforts on
perfecting Rust analysis before expanding to other languages. This includes:
- Deep Rust-specific features (macro expansion, trait resolution, lifetime analysis)
- Optimized algorithms for Rust codebases
- Comprehensive Rust pattern detection

Multi-language support will be reconsidered in future releases once Rust
analysis reaches maturity.

### Migration Guide
If you were using debtmap for Python, JavaScript, or TypeScript analysis,
please use version 0.6.x or consider language-specific alternatives until
debtmap re-introduces multi-language support.
```
