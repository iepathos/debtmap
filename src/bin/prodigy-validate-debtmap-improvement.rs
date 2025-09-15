use anyhow::{Context, Result};
use debtmap::commands::{compare_debtmaps, CompareConfig};
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args = env::var("ARGUMENTS").unwrap_or_default();

    let is_automation = env::var("PRODIGY_AUTOMATION")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true")
        || env::var("PRODIGY_VALIDATION")
            .unwrap_or_default()
            .eq_ignore_ascii_case("true");

    if !is_automation {
        println!("Validating debtmap improvement...");
    }

    let config = parse_arguments(&args)?;

    if !is_automation {
        println!("Comparing:");
        println!("  Before: {}", config.before_path.display());
        println!("  After: {}", config.after_path.display());
        println!("  Output: {}", config.output_path.display());
    }

    compare_debtmaps(config)?;

    if !is_automation {
        println!("\nValidation complete. Results written to output file.");
    }

    Ok(())
}

fn parse_arguments(args: &str) -> Result<CompareConfig> {
    let mut before_path = None;
    let mut after_path = None;
    let mut output_path = None;

    let parts: Vec<&str> = args.split_whitespace().collect();
    let mut i = 0;

    while i < parts.len() {
        match parts[i] {
            "--before" => {
                if i + 1 < parts.len() {
                    before_path = Some(PathBuf::from(parts[i + 1]));
                    i += 2;
                } else {
                    anyhow::bail!("--before requires a path argument");
                }
            }
            "--after" => {
                if i + 1 < parts.len() {
                    after_path = Some(PathBuf::from(parts[i + 1]));
                    i += 2;
                } else {
                    anyhow::bail!("--after requires a path argument");
                }
            }
            "--output" => {
                if i + 1 < parts.len() {
                    output_path = Some(PathBuf::from(parts[i + 1]));
                    i += 2;
                } else {
                    anyhow::bail!("--output requires a path argument");
                }
            }
            _ => i += 1,
        }
    }

    let before_path = before_path.context("Missing required --before argument")?;
    let after_path = after_path.context("Missing required --after argument")?;
    let output_path =
        output_path.unwrap_or_else(|| PathBuf::from(".prodigy/debtmap-validation.json"));

    if !before_path.exists() {
        anyhow::bail!("Before file does not exist: {}", before_path.display());
    }

    if !after_path.exists() {
        anyhow::bail!("After file does not exist: {}", after_path.display());
    }

    Ok(CompareConfig {
        before_path,
        after_path,
        output_path,
    })
}
