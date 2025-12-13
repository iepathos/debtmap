//! Dependency injection module
//!
//! This module provides default implementations of the core DI traits
//! used throughout the application.

mod default_implementations;

pub use default_implementations::{
    create_app_container, DefaultConfigProvider, DefaultDebtScorer, DefaultPriorityCalculator,
    JsonFormatter, MarkdownFormatter, TerminalFormatter,
};
