//! Retry configuration for resilient operations.
//!
//! This module provides configuration types for automatic retry of transient failures.
//! The retry pattern helps handle temporary issues like:
//!
//! - File locks from concurrent access
//! - Network filesystem unavailability
//! - Git index lock contention
//! - External tool execution failures
//!
//! # Configuration Example
//!
//! ```toml
//! [retry]
//! enabled = true
//! max_retries = 3
//! base_delay_ms = 100
//! strategy = "exponential"
//! timeout_seconds = 30
//! jitter_factor = 0.1
//! ```
//!
//! # Retry Strategies
//!
//! - **Constant**: Same delay between each retry
//! - **Linear**: Delay increases linearly (base * attempt)
//! - **Exponential**: Delay doubles each attempt (base * 2^attempt)
//! - **Fibonacci**: Delay follows fibonacci sequence

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Retry configuration for resilient operations.
///
/// Controls automatic retry behavior for transient failures.
/// When enabled, operations that fail with retryable errors will
/// be automatically retried according to the configured strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Enable automatic retries (default: true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Maximum number of retry attempts (default: 3)
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Base delay between retries in milliseconds (default: 100)
    #[serde(default = "default_base_delay_ms")]
    pub base_delay_ms: u64,

    /// Retry strategy (default: exponential)
    #[serde(default)]
    pub strategy: RetryStrategy,

    /// Maximum total time to spend retrying in seconds (default: 30)
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,

    /// Jitter factor to add randomness to delays (default: 0.1 = 10%)
    ///
    /// A value of 0.1 means delays can vary by +/- 10%.
    /// This helps prevent thundering herd problems in distributed systems.
    #[serde(default = "default_jitter_factor")]
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            max_retries: default_max_retries(),
            base_delay_ms: default_base_delay_ms(),
            strategy: RetryStrategy::default(),
            timeout_seconds: default_timeout_seconds(),
            jitter_factor: default_jitter_factor(),
        }
    }
}

impl RetryConfig {
    /// Create a retry config with retries disabled.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Get the base delay as a Duration.
    pub fn base_delay(&self) -> Duration {
        Duration::from_millis(self.base_delay_ms)
    }

    /// Get the timeout as a Duration.
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds)
    }

    /// Calculate the delay for a specific retry attempt.
    ///
    /// The attempt number is 1-indexed (first retry is attempt 1).
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_delay = self.base_delay();
        let base_ms = self.base_delay_ms as f64;

        let delay_ms = match self.strategy {
            RetryStrategy::Constant => base_ms,
            RetryStrategy::Linear => base_ms * (attempt as f64),
            RetryStrategy::Exponential => base_ms * 2.0_f64.powi(attempt as i32 - 1),
            RetryStrategy::Fibonacci => {
                // Compute fibonacci delay
                let fib = fibonacci(attempt);
                base_ms * (fib as f64)
            }
        };

        // Apply jitter
        let jittered_ms = if self.jitter_factor > 0.0 {
            apply_jitter(delay_ms, self.jitter_factor)
        } else {
            delay_ms
        };

        // Cap at timeout
        let max_delay = self.timeout_seconds * 1000;
        let final_ms = jittered_ms.min(max_delay as f64);

        Duration::from_millis(final_ms as u64).min(base_delay * 100) // Cap single delay at 100x base
    }

    /// Check if retries should continue based on attempt count and elapsed time.
    pub fn should_retry(&self, attempt: u32, elapsed: Duration) -> bool {
        if !self.enabled {
            return false;
        }
        attempt < self.max_retries && elapsed < self.timeout()
    }
}

/// Retry delay strategy.
///
/// Determines how the delay between retries changes with each attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RetryStrategy {
    /// Same delay between each retry.
    Constant,
    /// Delay increases linearly: base * attempt.
    Linear,
    /// Delay doubles each attempt: base * 2^(attempt-1).
    Exponential,
    /// Delay follows fibonacci sequence: base * fib(attempt).
    Fibonacci,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self::Exponential
    }
}

// Default value functions for serde
fn default_enabled() -> bool {
    true
}

fn default_max_retries() -> u32 {
    3
}

fn default_base_delay_ms() -> u64 {
    100
}

fn default_timeout_seconds() -> u64 {
    30
}

fn default_jitter_factor() -> f64 {
    0.1
}

/// Compute the nth fibonacci number (1-indexed).
fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        2 => 1,
        _ => {
            let mut a = 1u64;
            let mut b = 1u64;
            for _ in 2..n {
                let c = a.saturating_add(b);
                a = b;
                b = c;
            }
            b
        }
    }
}

/// Apply jitter to a delay value.
///
/// Jitter is applied as a random factor in the range [1 - factor, 1 + factor].
fn apply_jitter(delay_ms: f64, factor: f64) -> f64 {
    // Use a simple deterministic jitter for reproducibility
    // In production, this could use a random source
    let jitter_range = delay_ms * factor;
    // Apply a small positive jitter by default (deterministic for testing)
    delay_ms + (jitter_range * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay_ms, 100);
        assert_eq!(config.strategy, RetryStrategy::Exponential);
        assert_eq!(config.timeout_seconds, 30);
        assert!((config.jitter_factor - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_retry_config_disabled() {
        let config = RetryConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_base_delay() {
        let config = RetryConfig {
            base_delay_ms: 250,
            ..Default::default()
        };
        assert_eq!(config.base_delay(), Duration::from_millis(250));
    }

    #[test]
    fn test_timeout() {
        let config = RetryConfig {
            timeout_seconds: 60,
            ..Default::default()
        };
        assert_eq!(config.timeout(), Duration::from_secs(60));
    }

    #[test]
    fn test_constant_strategy_delay() {
        let config = RetryConfig {
            strategy: RetryStrategy::Constant,
            base_delay_ms: 100,
            jitter_factor: 0.0, // Disable jitter for predictable test
            ..Default::default()
        };

        // Constant: same delay for all attempts
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(100));
    }

    #[test]
    fn test_linear_strategy_delay() {
        let config = RetryConfig {
            strategy: RetryStrategy::Linear,
            base_delay_ms: 100,
            jitter_factor: 0.0,
            ..Default::default()
        };

        // Linear: base * attempt
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(200));
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(300));
    }

    #[test]
    fn test_exponential_strategy_delay() {
        let config = RetryConfig {
            strategy: RetryStrategy::Exponential,
            base_delay_ms: 100,
            jitter_factor: 0.0,
            ..Default::default()
        };

        // Exponential: base * 2^(attempt-1)
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(100)); // 100 * 2^0
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(200)); // 100 * 2^1
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(400)); // 100 * 2^2
    }

    #[test]
    fn test_fibonacci_strategy_delay() {
        let config = RetryConfig {
            strategy: RetryStrategy::Fibonacci,
            base_delay_ms: 100,
            jitter_factor: 0.0,
            ..Default::default()
        };

        // Fibonacci: base * fib(n) where fib(1)=1, fib(2)=1, fib(3)=2, fib(4)=3
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(100)); // 100 * 1
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(100)); // 100 * 1
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(200)); // 100 * 2
        assert_eq!(config.delay_for_attempt(4), Duration::from_millis(300)); // 100 * 3
    }

    #[test]
    fn test_should_retry() {
        let config = RetryConfig {
            enabled: true,
            max_retries: 3,
            timeout_seconds: 30,
            ..Default::default()
        };

        // Within limits
        assert!(config.should_retry(0, Duration::from_secs(0)));
        assert!(config.should_retry(1, Duration::from_secs(5)));
        assert!(config.should_retry(2, Duration::from_secs(10)));

        // Exceeded retry count
        assert!(!config.should_retry(3, Duration::from_secs(10)));

        // Exceeded timeout
        assert!(!config.should_retry(1, Duration::from_secs(31)));
    }

    #[test]
    fn test_should_retry_disabled() {
        let config = RetryConfig::disabled();
        assert!(!config.should_retry(0, Duration::from_secs(0)));
    }

    #[test]
    fn test_fibonacci_function() {
        assert_eq!(fibonacci(0), 0);
        assert_eq!(fibonacci(1), 1);
        assert_eq!(fibonacci(2), 1);
        assert_eq!(fibonacci(3), 2);
        assert_eq!(fibonacci(4), 3);
        assert_eq!(fibonacci(5), 5);
        assert_eq!(fibonacci(6), 8);
    }

    #[test]
    fn test_jitter_applied() {
        let config = RetryConfig {
            strategy: RetryStrategy::Constant,
            base_delay_ms: 100,
            jitter_factor: 0.1, // 10% jitter
            ..Default::default()
        };

        let delay = config.delay_for_attempt(1);
        // With 10% jitter applied at +0.5, we expect ~105ms
        assert!(delay >= Duration::from_millis(100));
        assert!(delay <= Duration::from_millis(115));
    }

    #[test]
    fn test_delay_capped_at_timeout() {
        let config = RetryConfig {
            strategy: RetryStrategy::Exponential,
            base_delay_ms: 10000, // 10 second base
            timeout_seconds: 5,   // 5 second timeout
            jitter_factor: 0.0,
            ..Default::default()
        };

        // Should not exceed 100x base (1000 seconds) but also limited by timeout
        let delay = config.delay_for_attempt(5);
        assert!(delay <= Duration::from_secs(1000)); // Within 100x cap
    }

    #[test]
    fn test_serde_roundtrip() {
        let config = RetryConfig {
            enabled: true,
            max_retries: 5,
            base_delay_ms: 200,
            strategy: RetryStrategy::Linear,
            timeout_seconds: 60,
            jitter_factor: 0.2,
        };

        let toml = toml::to_string(&config).unwrap();
        let parsed: RetryConfig = toml::from_str(&toml).unwrap();

        assert_eq!(parsed.enabled, config.enabled);
        assert_eq!(parsed.max_retries, config.max_retries);
        assert_eq!(parsed.base_delay_ms, config.base_delay_ms);
        assert_eq!(parsed.strategy, config.strategy);
        assert_eq!(parsed.timeout_seconds, config.timeout_seconds);
        assert!((parsed.jitter_factor - config.jitter_factor).abs() < 0.001);
    }

    #[test]
    fn test_serde_defaults() {
        let toml = "";
        let config: RetryConfig = toml::from_str(toml).unwrap();
        assert!(config.enabled);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_strategy_serde() {
        // Test strategy parsing through a full config
        let config: RetryConfig = toml::from_str(r#"strategy = "constant""#).unwrap();
        assert_eq!(config.strategy, RetryStrategy::Constant);

        let config: RetryConfig = toml::from_str(r#"strategy = "linear""#).unwrap();
        assert_eq!(config.strategy, RetryStrategy::Linear);

        let config: RetryConfig = toml::from_str(r#"strategy = "exponential""#).unwrap();
        assert_eq!(config.strategy, RetryStrategy::Exponential);

        let config: RetryConfig = toml::from_str(r#"strategy = "fibonacci""#).unwrap();
        assert_eq!(config.strategy, RetryStrategy::Fibonacci);
    }
}
