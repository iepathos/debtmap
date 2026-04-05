## Stage 1: Inspect Release State
**Goal**: Confirm the current crate version, changelog structure, release workflow, and git status.
**Success Criteria**: Release inputs are identified and the next release version is selected.
**Tests**: Verify `Cargo.toml`, `CHANGELOG.md`, release scripts, and `git status`.
**Status**: Complete

## Stage 2: Update Release Metadata
**Goal**: Bump the crate version and convert unreleased notes into a dated changelog entry for the next release.
**Success Criteria**: `Cargo.toml` and `CHANGELOG.md` reflect the new release version and date.
**Tests**: Inspect diffs for version consistency and changelog formatting.
**Status**: Complete

## Stage 3: Regenerate Lockfile
**Goal**: Recreate `Cargo.lock` from the updated manifest state.
**Success Criteria**: `Cargo.lock` is regenerated successfully with the new root package version.
**Tests**: Run `cargo generate-lockfile`.
**Status**: Complete

## Stage 4: Verify And Commit
**Goal**: Review the final diff, validate relevant release checks, and create a release-prep commit.
**Success Criteria**: Changes are verified and committed with a clean, conventional message.
**Tests**: Inspect `git diff`, run targeted Cargo checks if needed, and confirm `git status`.
**Status**: Complete
