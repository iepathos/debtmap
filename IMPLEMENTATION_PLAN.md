## Stage 1: Audit Release-Facing Docs
**Goal**: Identify README and book pages that still describe pre-0.16 language support.
**Success Criteria**: Landing pages and support matrix pages with stale Rust-only or JS/TS-first messaging are identified.
**Tests**: Inspect `README.md` and the book's introduction, getting-started, FAQ, and language-analysis pages.
**Status**: Complete

## Stage 2: Update Top-Level Release Messaging
**Goal**: Refresh `README.md` for the 0.16.0 release and document Python as supported.
**Success Criteria**: README quick-start, supported languages, and roadmap content no longer list Python as planned work.
**Tests**: Review the README support matrix and example commands for consistency with current language support.
**Status**: Complete

## Stage 3: Update Book Entry Points
**Goal**: Align the book's introduction and getting-started pages with Python support.
**Success Criteria**: New users reading the book see Rust and Python called out correctly in installation, first-run, and status sections.
**Tests**: Review `book/src/introduction.md` and `book/src/getting-started.md` after edits.
**Status**: Complete

## Stage 4: Update Book Reference Pages
**Goal**: Correct the FAQ and analysis/configuration support pages to match the current implementation.
**Success Criteria**: Support matrices and analyzer/configuration docs no longer present Python as planned or JS/TS as default supported languages.
**Tests**: Review `book/src/faq.md`, `book/src/analysis-guide/analyzer-types.md`, `book/src/analysis-guide/overview.md`, `book/src/configuration/index.md`, and `book/src/configuration/languages.md`.
**Status**: Complete

## Stage 5: Verify Documentation Diff
**Goal**: Confirm the documentation updates are internally consistent and limited to release-prep docs.
**Success Criteria**: Diff shows coherent documentation-only changes for the 0.16.0 release.
**Tests**: Review `git diff --stat` and targeted diffs for edited docs.
**Status**: Complete
