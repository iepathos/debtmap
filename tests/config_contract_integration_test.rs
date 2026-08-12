use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

fn debtmap_command(current_dir: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_debtmap"));
    command
        .current_dir(current_dir)
        .env("HOME", current_dir.join(".test-home"))
        .env("XDG_CONFIG_HOME", current_dir.join(".test-config"))
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

fn run_json_analysis(directory: &Path, config: Option<&Path>, output_name: &str) -> Output {
    let mut command = debtmap_command(directory);
    if let Some(path) = config {
        command.arg("--config").arg(path);
    }
    command.args([
        "analyze",
        ".",
        "--format",
        "json",
        "--quiet",
        "--no-tui",
        "--no-context-aware",
        "--no-parallel",
        "--min-score",
        "0",
        "--output",
        output_name,
    ]);
    command.output().unwrap()
}

fn report_total_loc(directory: &Path, output_name: &str) -> u64 {
    let report: Value =
        serde_json::from_str(&fs::read_to_string(directory.join(output_name)).unwrap()).unwrap();
    report["summary"]["total_loc"].as_u64().unwrap()
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

#[test]
fn explicit_config_controls_ordinary_analysis() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("sample.rs"), "fn sample() {}\n").unwrap();
    let config = directory.path().join("custom.toml");
    fs::write(&config, "[ignore]\npatterns = [\"*.rs\"]\n").unwrap();

    let baseline = run_json_analysis(directory.path(), None, "baseline.json");
    assert!(baseline.status.success(), "{}", output_text(&baseline));
    assert_eq!(report_total_loc(directory.path(), "baseline.json"), 1);

    let configured = run_json_analysis(directory.path(), Some(&config), "configured.json");
    assert!(configured.status.success(), "{}", output_text(&configured));
    assert_eq!(report_total_loc(directory.path(), "configured.json"), 0);
}

#[test]
fn environment_config_controls_ordinary_analysis() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("sample.rs"), "fn sample() {}\n").unwrap();
    let config = directory.path().join("custom.toml");
    fs::write(&config, "[ignore]\npatterns = [\"*.rs\"]\n").unwrap();

    let output = debtmap_command(directory.path())
        .env("DEBTMAP_CONFIG", &config)
        .args([
            "analyze",
            ".",
            "--format",
            "json",
            "--quiet",
            "--no-tui",
            "--no-context-aware",
            "--no-parallel",
            "--min-score",
            "0",
            "--output",
            "environment.json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", output_text(&output));
    assert_eq!(report_total_loc(directory.path(), "environment.json"), 0);
}

#[test]
fn explicit_config_inherits_reported_lower_precedence_sources() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("sample.rs"), "fn sample() {}\n").unwrap();
    fs::write(
        directory.path().join(".debtmap.toml"),
        "[ignore]\npatterns = [\"*.rs\"]\n",
    )
    .unwrap();
    let config = directory.path().join("custom.toml");
    fs::write(&config, "[display]\nitems_per_tier = 7\n").unwrap();

    let output = run_json_analysis(directory.path(), Some(&config), "layered.json");

    assert!(output.status.success(), "{}", output_text(&output));
    assert_eq!(report_total_loc(directory.path(), "layered.json"), 0);
}

#[test]
fn explicit_config_is_used_when_showing_sources() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join(".debtmap.toml"),
        "[thresholds]\ncomplexity = 11\n",
    )
    .unwrap();
    let config = directory.path().join("custom.toml");
    fs::write(&config, "[display]\nitems_per_tier = 7\n").unwrap();

    let output = debtmap_command(directory.path())
        .arg("--config")
        .arg(&config)
        .args(["--show-config-sources", "analyze", "."])
        .env("DEBTMAP_COMPLEXITY_THRESHOLD", "33")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", output_text(&output));
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains(&format!("custom config: {}", config.display())),
        "{}",
        output_text(&output)
    );

    let first = String::from_utf8_lossy(&output.stdout);
    let field_names: Vec<_> = first
        .lines()
        .filter(|line| line.starts_with("  ") && line.ends_with(" = <value>"))
        .map(|line| line.trim().trim_end_matches(" = <value>"))
        .collect();
    let mut sorted_names = field_names.clone();
    sorted_names.sort_unstable();
    assert_eq!(field_names, sorted_names);

    let default_index = first.find("  1. built-in defaults").unwrap();
    let project_index = first.find("  2. project config:").unwrap();
    let custom_index = first.find("  3. custom config:").unwrap();
    let environment_index = first.find("  4. environment variable:").unwrap();
    assert!(default_index < project_index);
    assert!(project_index < custom_index);
    assert!(custom_index < environment_index);
}

#[test]
fn invalid_explicit_config_stops_analysis() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("sample.rs"), "fn sample() {}\n").unwrap();
    let config = directory.path().join("invalid.toml");
    fs::write(&config, "[thresholds\ncomplexity = 17\n").unwrap();

    let output = run_json_analysis(directory.path(), Some(&config), "report.json");

    assert!(!output.status.success(), "{}", output_text(&output));
    assert!(!directory.path().join("report.json").exists());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Failed to parse .debtmap.toml"),
        "{}",
        output_text(&output)
    );
}

#[test]
fn invalid_explicit_scoring_stops_analysis() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("sample.rs"), "fn sample() {}\n").unwrap();
    let config = directory.path().join("invalid-scoring.toml");
    fs::write(
        &config,
        "[scoring]\ncoverage = 1.0\ncomplexity = 1.0\ndependency = 1.0\n",
    )
    .unwrap();

    let output = run_json_analysis(directory.path(), Some(&config), "scoring.json");

    assert!(!output.status.success(), "{}", output_text(&output));
    assert!(!directory.path().join("scoring.json").exists());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("must sum to 1.0"),
        "{}",
        output_text(&output)
    );
}

#[test]
fn malformed_discovered_config_stops_analysis() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("sample.rs"), "fn sample() {}\n").unwrap();
    fs::write(directory.path().join(".debtmap.toml"), "[thresholds\n").unwrap();

    let output = run_json_analysis(directory.path(), None, "discovered.json");

    assert!(!output.status.success(), "{}", output_text(&output));
    assert!(!directory.path().join("discovered.json").exists());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Failed to parse .debtmap.toml"),
        "{}",
        output_text(&output)
    );
}

#[test]
fn user_config_is_applied_to_ordinary_analysis() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("sample.rs"), "fn sample() {}\n").unwrap();
    let user_directories = [
        directory.path().join(".test-config/debtmap"),
        directory.path().join(".test-home/.config/debtmap"),
        directory
            .path()
            .join(".test-home/Library/Application Support/debtmap"),
    ];
    for user_directory in user_directories {
        fs::create_dir_all(&user_directory).unwrap();
        fs::write(
            user_directory.join("config.toml"),
            "[ignore]\npatterns = [\"*.rs\"]\n",
        )
        .unwrap();
    }

    let output = run_json_analysis(directory.path(), None, "user.json");

    assert!(output.status.success(), "{}", output_text(&output));
    assert_eq!(report_total_loc(directory.path(), "user.json"), 0);
}

#[cfg(target_os = "linux")]
#[test]
fn explicit_non_utf8_config_path_is_honored() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("sample.rs"), "fn sample() {}\n").unwrap();
    let config = directory
        .path()
        .join(OsString::from_vec(b"config-\xff.toml".to_vec()));
    fs::write(&config, "[ignore]\npatterns = [\"*.rs\"]\n").unwrap();

    let output = run_json_analysis(directory.path(), Some(&config), "non-utf8.json");

    assert!(output.status.success(), "{}", output_text(&output));
    assert_eq!(report_total_loc(directory.path(), "non-utf8.json"), 0);
}

#[cfg(all(unix, not(target_os = "linux")))]
#[test]
fn unsupported_non_utf8_config_path_fails_closed() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("sample.rs"), "fn sample() {}\n").unwrap();
    let config = directory
        .path()
        .join(OsString::from_vec(b"config-\xff.toml".to_vec()));

    let output = run_json_analysis(directory.path(), Some(&config), "non-utf8.json");

    assert!(!output.status.success(), "{}", output_text(&output));
    assert!(!directory.path().join("non-utf8.json").exists());
}

#[test]
fn validate_subcommand_config_is_not_ignored() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing.toml");

    let output = debtmap_command(directory.path())
        .args(["validate", ".", "--config"])
        .arg(&missing)
        .output()
        .unwrap();

    assert!(!output.status.success(), "{}", output_text(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Cannot read config file"),
        "{}",
        output_text(&output)
    );
}
