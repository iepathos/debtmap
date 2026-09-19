//! Coverage policy contracts. Unix execution tests require the `just` developer tool.

const JUSTFILE: &str = include_str!("../Justfile");

#[test]
fn ci_uses_the_shared_coverage_policy() {
    let workflow = include_str!("../.github/workflows/coverage.yml");
    assert!(workflow.contains("just coverage-check"));
    assert!(workflow.contains("just coverage-report-html"));
    assert!(!workflow.contains("cargo llvm-cov"));
    assert!(!workflow.contains("--test "));
    assert!(!workflow.contains("python3"));
    let coverage = JUSTFILE.split("coverage:").nth(1).unwrap();
    let coverage = coverage.split("coverage-open:").next().unwrap();
    assert!(
        !coverage.contains("--test "),
        "Do not restore a test allowlist"
    );
    assert!(
        !coverage.contains("--ignored"),
        "Honor normal ignored tests"
    );
}

#[cfg(unix)]
mod execution {
    use super::JUSTFILE;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Output};
    use tempfile::TempDir;

    const CARGO_STUB: &str = r#"#!/bin/sh
set -eu
printf '%s|%s\n' "${CARGO_LLVM_COV_TARGET_DIR:-default}" "$*" >> "$DEBTMAP_CARGO_LOG"
case "$*" in
  *--no-report*) [ "${DEBTMAP_FAKE_FAILURE:-}" != collection ] || exit 17 ;;
  *--fail-under-lines*) [ "${DEBTMAP_FAKE_FAILURE:-}" != threshold ] || exit 19 ;;
esac
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output-path) shift; mkdir -p "$(dirname "$1")"; printf '%s' "${CARGO_LLVM_COV_TARGET_DIR:-default}" > "$1" ;;
    --output-dir) shift; mkdir -p "$1/html"; touch "$1/html/index.html" ;;
  esac
  shift
done
"#;

    struct Fixture(TempDir);

    impl Fixture {
        fn new() -> Self {
            let fixture = Self(tempfile::tempdir().unwrap());
            fs::write(fixture.0.path().join("Justfile"), JUSTFILE).unwrap();
            let cargo = fixture.0.path().join("cargo");
            fs::write(&cargo, CARGO_STUB).unwrap();
            fs::set_permissions(cargo, fs::Permissions::from_mode(0o755)).unwrap();
            fixture
        }

        fn run(&self, recipe: &str, failure: &str) -> Output {
            let paths = std::iter::once(self.0.path().to_path_buf())
                .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap()))
                .collect::<Vec<_>>();
            Command::new("just")
                .arg(recipe)
                .current_dir(self.0.path())
                .env("PATH", std::env::join_paths(paths).unwrap())
                .env("DEBTMAP_CARGO_LOG", self.0.path().join("cargo.log"))
                .env("DEBTMAP_FAKE_FAILURE", failure)
                .env_remove("CARGO_LLVM_COV_TARGET_DIR")
                .env_remove("CARGO_TARGET_DIR")
                .output()
                .expect("Coverage recipe contracts require `just` installed on PATH")
        }

        fn log(&self) -> String {
            fs::read_to_string(self.0.path().join("cargo.log")).unwrap_or_default()
        }

        fn exists(&self, path: &str) -> bool {
            self.0.path().join(path).exists()
        }
    }

    fn assert_success(output: Output) {
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn all_representative_formats_and_legacy_aliases_discover_all_tests() {
        for recipe in [
            "coverage",
            "coverage-lcov",
            "coverage-check",
            "coverage-full",
            "coverage-full-lcov",
            "coverage-full-check",
        ] {
            let fixture = Fixture::new();
            assert_success(fixture.run(recipe, ""));
            let log = fixture.log();
            let collections = log
                .lines()
                .filter(|line| line.contains("--no-report"))
                .collect::<Vec<_>>();
            assert_eq!(collections.len(), 1, "{recipe}: {log}");
            assert!(
                collections[0].contains("--all-features")
                    && collections[0].contains("--tests")
                    && collections[0].ends_with("-- --quiet"),
                "{recipe}: {log}"
            );
            assert!(log.contains("clean --profraw-only"), "{recipe}: {log}");
            assert!(fixture.exists("target/coverage/.complete"));
        }
    }

    #[test]
    fn failed_collection_invalidates_old_reports_and_prevents_export() {
        let fixture = Fixture::new();
        assert_success(fixture.run("coverage-lcov", ""));
        assert_success(fixture.run("coverage-report-check", ""));
        assert_success(fixture.run("coverage-report-html", ""));
        assert!(fixture.exists("target/coverage/html/index.html"));
        fs::write(fixture.0.path().join("cargo.log"), "").unwrap();
        assert!(!fixture.run("coverage-lcov", "collection").status.success());
        assert!(!fixture.exists("target/coverage/.complete"));
        assert!(!fixture.exists("target/coverage/lcov.info"));
        assert!(!fixture.exists("target/coverage/coverage-summary.json"));
        assert!(!fixture.exists("target/coverage/html/index.html"));
        assert!(!fixture.log().contains("llvm-cov report"));
        for recipe in [
            "coverage-report-html",
            "coverage-report-lcov",
            "coverage-report-check",
        ] {
            assert!(!fixture.run(recipe, "").status.success());
        }
        assert!(!fixture.log().contains("llvm-cov report"));
    }

    #[test]
    fn native_coverage_threshold_failure_propagates() {
        let fixture = Fixture::new();
        assert!(!fixture.run("coverage-check", "threshold").status.success());
        assert!(fixture.exists("target/coverage/.complete"));
        assert!(fixture.log().contains("--fail-under-lines 80"));
    }

    #[test]
    fn partial_coverage_has_separate_profiles_and_reports() {
        let fixture = Fixture::new();
        assert_success(fixture.run("coverage-lcov", ""));
        fs::write(fixture.0.path().join("cargo.log"), "").unwrap();
        for recipe in ["coverage-fast", "coverage-fast-lcov"] {
            assert_success(fixture.run(recipe, ""));
        }
        let log = fixture.log();
        assert!(
            log.lines()
                .all(|line| line.starts_with("target/llvm-cov-fast-target|")),
            "{log}"
        );
        assert!(!log.contains("--tests"));
        assert!(log.contains("--lib"));
        assert!(fixture.exists("target/coverage-fast/lcov.info"));
        assert!(fixture.exists("target/coverage/.complete"));
        assert_eq!(
            fs::read_to_string(fixture.0.path().join("target/coverage/lcov.info")).unwrap(),
            "default"
        );
        assert_success(fixture.run("coverage-fast", ""));
        assert!(fixture.exists("target/coverage-fast/html/index.html"));
        assert!(
            !fixture
                .run("coverage-fast-lcov", "collection")
                .status
                .success()
        );
        assert!(!fixture.exists("target/coverage-fast/html/index.html"));
        assert!(!fixture.exists("target/coverage-fast/lcov.info"));
        assert!(fixture.exists("target/coverage/.complete"));
        assert!(fixture.exists("target/coverage/lcov.info"));
    }
}
