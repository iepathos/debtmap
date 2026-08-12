use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn debtmap_command(current_dir: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_debtmap"));
    command
        .current_dir(current_dir)
        .env_remove("DEBTMAP_CONFIG");
    command
}

fn output_text(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn init_template_passes_strict_config_check() {
    let directory = tempfile::tempdir().unwrap();
    let init = debtmap_command(directory.path())
        .arg("init")
        .output()
        .unwrap();
    assert!(init.status.success(), "{}", output_text(&init));

    let check = debtmap_command(directory.path())
        .args(["config", "check"])
        .output()
        .unwrap();
    assert!(check.status.success(), "{}", output_text(&check));
    assert!(
        String::from_utf8_lossy(&check.stdout).contains("Configuration is valid:"),
        "{}",
        output_text(&check)
    );

    let generated = fs::read_to_string(directory.path().join(".debtmap.toml")).unwrap();
    let parsed = debtmap::config::parse_and_validate_config(&generated).unwrap();
    let validation = parsed.thresholds.unwrap().validation.unwrap();
    assert_eq!(validation.max_debt_density, 50.0);
    assert_eq!(validation.max_codebase_risk_score, 7.0);
}

#[test]
fn config_check_reports_all_unknown_keys_with_suggestions() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("custom.toml");
    fs::write(
        &path,
        "threshold = 10\n[thresholds]\nmax_function_lenght = 40\n",
    )
    .unwrap();

    let output = debtmap_command(directory.path())
        .arg("--config")
        .arg(&path)
        .args(["config", "check"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "{}", output_text(&output));
    assert!(stderr.contains("unknown configuration key `threshold`"));
    assert!(stderr.contains("did you mean `thresholds`?"));
    assert!(stderr.contains("`thresholds.max_function_lenght`"));
    assert!(stderr.contains("`thresholds.max_function_length`"));
}

#[test]
fn config_check_requires_an_explicit_or_discovered_file() {
    let directory = tempfile::tempdir().unwrap();
    let output = debtmap_command(directory.path())
        .args(["config", "check"])
        .output()
        .unwrap();

    assert!(!output.status.success(), "{}", output_text(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("debtmap init"),
        "{}",
        output_text(&output)
    );
}

#[test]
fn legacy_parser_remains_permissive_for_unknown_keys() {
    assert!(debtmap::config::parse_and_validate_config("threshold = 10").is_ok());
}

#[test]
fn shipped_configs_pass_the_executable_contract() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let paths = [
        ".debtmap.toml",
        ".debtmap.example.toml",
        ".debtmap-improved.toml",
        "examples/config-permissive.toml",
        "examples/config-strict.toml",
        "examples/library_config.toml",
        "examples/orchestration-config.toml",
    ];
    let failures: Vec<_> = paths
        .iter()
        .filter_map(|relative| {
            let output = debtmap_command(root)
                .arg("--config")
                .arg(root.join(relative))
                .args(["config", "check"])
                .output()
                .unwrap();
            (!output.status.success()).then(|| format!("{relative}: {}", output_text(&output)))
        })
        .collect();

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
