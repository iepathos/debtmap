//! Dependency injection module
//!
//! This module provides default implementations of the core DI traits
//! used throughout the application.

mod default_implementations;

pub use default_implementations::{
    DefaultConfigProvider, DefaultDebtScorer, DefaultPriorityCalculator, JsonFormatter,
    MarkdownFormatter, TerminalFormatter, create_app_container,
};
