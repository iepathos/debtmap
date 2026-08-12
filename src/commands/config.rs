//! Configuration inspection commands.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const MAX_TRAVERSAL_DEPTH: usize = 10;

pub fn check_config(explicit_path: Option<&Path>) -> Result<()> {
    let path = resolve_config_path(explicit_path)?;
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("Cannot read configuration file {}", path.display()))?;

    match crate::config::check::check_config_contents(&contents) {
        Ok(_) => {
            println!("Configuration is valid: {}", path.display());
            Ok(())
        }
        Err(diagnostics) => {
            eprintln!("Configuration errors in {}:", path.display());
            for diagnostic in diagnostics {
                eprintln!("  - {}", diagnostic.render());
            }
            anyhow::bail!("configuration is invalid")
        }
    }
}

fn resolve_config_path(explicit_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit_path {
        return path
            .is_file()
            .then(|| path.to_path_buf())
            .with_context(|| format!("Configuration file not found: {}", path.display()));
    }

    let current = std::env::current_dir().context("Cannot determine the current directory")?;
    crate::config::directory_ancestors(current, MAX_TRAVERSAL_DEPTH)
        .map(|directory| directory.join(".debtmap.toml"))
        .find(|path| path.is_file())
        .context("No .debtmap.toml found; run `debtmap init` to create one")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_missing_path_is_an_error() {
        let error = resolve_config_path(Some(Path::new("missing-config.toml"))).unwrap_err();
        assert!(error.to_string().contains("Configuration file not found"));
    }
}
