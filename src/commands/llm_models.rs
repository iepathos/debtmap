//! `debtmap llm-models`: list model profiles for the LLM editability estimate.

use crate::llm::{ModelProfile, Registry};
use anyhow::{Result, anyhow};

pub fn list_llm_models() -> Result<()> {
    let llm = crate::config::get_config()
        .llm_editability
        .clone()
        .unwrap_or_default();
    let registry = Registry::load(&llm.profiles).map_err(|e| anyhow!(e))?;
    let profiles = registry
        .ids()
        .iter()
        .map(|id| registry.resolve(id))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow!(e))?;
    print!("{}", format_model_list(&profiles));
    Ok(())
}

fn format_model_list(profiles: &[ModelProfile]) -> String {
    let header = format!(
        "{:<26} {:<10} {:<24} {:>4} {:>9} {:>9}  ALIASES\n",
        "ID", "PROVIDER", "TIER", "COST", "CONTEXT", "EFFECTIVE"
    );
    profiles
        .iter()
        .map(|p| {
            format!(
                "{:<26} {:<10} {:<24} {:>4} {:>9} {:>9}  {}\n",
                p.id,
                p.provider,
                p.tier,
                p.cost_tier,
                p.context_window,
                p.effective_context_tokens,
                p.aliases.join(", ")
            )
        })
        .fold(header, |acc, line| acc + &line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn lists_every_builtin_profile() {
        let registry = Registry::load(&BTreeMap::new()).unwrap();
        let profiles: Vec<_> = registry
            .ids()
            .iter()
            .map(|id| registry.resolve(id).unwrap())
            .collect();
        let listing = format_model_list(&profiles);
        assert_eq!(listing.lines().count(), profiles.len() + 1);
        assert!(listing.contains("claude-sonnet"));
        assert!(listing.contains("tier:small_local"));
    }
}
