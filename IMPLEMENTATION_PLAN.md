## Stage 1: Audit Release Inputs
**Goal**: Confirm the current crate version, lockfile state, and changelog structure for the upcoming release.
**Success Criteria**: Root release files are identified and commits since `v0.15.3` are reviewed for changelog content.
**Tests**: Inspect `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, and recent git history.
**Status**: Complete

## Stage 2: Update Release Metadata
**Goal**: Bump the crate version to `0.16.0` and prepare the changelog entry.
**Success Criteria**: `Cargo.toml` and `CHANGELOG.md` reflect the `0.16.0` release with a dated release section and refreshed `Unreleased` heading.
**Tests**: Review edited metadata files for consistency and semantic version formatting.
**Status**: Complete

## Stage 3: Regenerate Lockfile
**Goal**: Refresh the root Cargo lockfile so it records the new package version.
**Success Criteria**: `Cargo.lock` is regenerated successfully and the `debtmap` package entry matches `0.16.0`.
**Tests**: Run `cargo generate-lockfile` and inspect the root package entry in `Cargo.lock`.
**Status**: Complete

## Stage 4: Verify Release Artifacts
**Goal**: Confirm the release-prep diff is limited to the expected files and content.
**Success Criteria**: Version, changelog, plan, and lockfile updates are internally consistent and ready for review.
**Tests**: Review `git diff --stat` and targeted file diffs.
**Status**: Complete
