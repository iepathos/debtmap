//! Unified filter configuration for debt items.
//!
//! This module provides a single source of truth for all filtering thresholds.
//! Configuration precedence: CLI args > env vars > config file > defaults.

use crate::config;

/// Unified filter configuration for debt items.
///
/// All filtering happens during item construction using these thresholds.
/// There is no post-filtering stage.
///
/// # Configuration Precedence
///
/// 1. CLI arguments (handled by caller, passed to constructor)
/// 2. Environment variables
/// 3. Config file (`Config.toml`)
/// 4. Hardcoded defaults
///
/// # Examples
///
/// ```rust
/// // Get configuration from environment
/// let config = ItemFilterConfig::from_environment();
///
/// // Override with CLI args
/// let config = ItemFilterConfig::from_environment()
///     .with_min_score(Some(10.0));
///
/// // Use in filtering
/// if meets_score_threshold(&item, config.min_score) { ... }
/// ```
#[derive(Debug, Clone)]
pub struct ItemFilterConfig {
    /// Minimum unified score threshold (0-100 scale)
    pub min_score: f64,

    /// Minimum cyclomatic complexity threshold
    pub min_cyclomatic: u32,

    /// Minimum cognitive complexity threshold
    pub min_cognitive: u32,

    /// Minimum risk score threshold (0-1 scale)
    pub min_risk: f64,

    /// Whether to show T4 (low priority) items
    pub show_t4_items: bool,
}

impl ItemFilterConfig {
    /// Create configuration from environment (env vars, config file, defaults).
    ///
    /// Precedence: env vars > config file > defaults
    ///
    /// # Environment Variables
    ///
    /// - `DEBTMAP_MIN_SCORE_THRESHOLD`: Minimum score (0-100)
    /// - `DEBTMAP_MIN_CYCLOMATIC`: Minimum cyclomatic complexity
    /// - `DEBTMAP_MIN_COGNITIVE`: Minimum cognitive complexity
    /// - `DEBTMAP_MIN_RISK`: Minimum risk score (0-1)
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Get from environment
    /// let config = ItemFilterConfig::from_environment();
    ///
    /// // Override min_score from CLI
    /// let config = config.with_min_score(Some(10.0));
    /// ```
    pub fn from_environment() -> Self {
        Self {
            min_score: get_min_score_threshold(),
            min_cyclomatic: config::get_minimum_cyclomatic_complexity(),
            min_cognitive: config::get_minimum_cognitive_complexity(),
            min_risk: config::get_minimum_risk_score(),
            show_t4_items: get_show_t4_items(),
        }
    }

    /// Override minimum score (for CLI args).
    pub fn with_min_score(mut self, min_score: Option<f64>) -> Self {
        if let Some(score) = min_score {
            self.min_score = score;
        }
        self
    }

    /// Override minimum cyclomatic complexity (for CLI args).
    pub fn with_min_cyclomatic(mut self, min_cyclomatic: Option<u32>) -> Self {
        if let Some(cyc) = min_cyclomatic {
            self.min_cyclomatic = cyc;
        }
        self
    }

    /// Override minimum cognitive complexity (for CLI args).
    pub fn with_min_cognitive(mut self, min_cognitive: Option<u32>) -> Self {
        if let Some(cog) = min_cognitive {
            self.min_cognitive = cog;
        }
        self
    }

    /// Create permissive configuration (for testing).
    pub fn permissive() -> Self {
        Self {
            min_score: 0.0,
            min_cyclomatic: 0,
            min_cognitive: 0,
            min_risk: 0.0,
            show_t4_items: true,
        }
    }
}

/// Get minimum score threshold with precedence.
fn get_min_score_threshold() -> f64 {
    // Environment variable takes precedence
    if let Ok(env_value) = std::env::var("DEBTMAP_MIN_SCORE_THRESHOLD") {
        if let Ok(threshold) = env_value.parse::<f64>() {
            return threshold;
        }
    }

    // Fallback to config file
    config::get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.min_score_threshold)
        .unwrap_or(3.0) // Default
}

/// Get show T4 items setting.
fn get_show_t4_items() -> bool {
    config::get_config()
        .tiers
        .as_ref()
        .map(|t| t.show_t4_in_main_report)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configuration_has_reasonable_thresholds() {
        let config = ItemFilterConfig::from_environment();

        assert!(config.min_score >= 0.0);
        // min_cyclomatic and min_cognitive are u32, so always >= 0
        assert!(config.min_risk >= 0.0);
    }

    #[test]
    fn with_min_score_overrides() {
        let config = ItemFilterConfig::from_environment().with_min_score(Some(10.0));

        assert_eq!(config.min_score, 10.0);
    }

    #[test]
    fn permissive_config_allows_everything() {
        let config = ItemFilterConfig::permissive();

        assert_eq!(config.min_score, 0.0);
        assert_eq!(config.min_cyclomatic, 0);
        assert_eq!(config.min_cognitive, 0);
        assert!(config.show_t4_items);
    }
}
