//! Effect-based output writers for debtmap analysis.
//!
//! This module provides Effect-wrapped output writers that enable testable,
//! composable output operations. All I/O is deferred until the effect is run.
//!
//! # Design Philosophy
//!
//! Following the Stillwater philosophy of "Pure Core, Imperative Shell":
//!
//! - **Pure Rendering** (`render`): Pure functions that transform data to strings
//! - **Effect Wrapping** (`writers`, `compose`, `report`): I/O operations wrapped in Effects
//! - **Composability**: Multiple outputs can be combined in a single pipeline
//! - **Testability**: All writers can be tested without file system access
//!
//! # Module Structure
//!
//! - [`config`]: Configuration types for output generation
//! - [`render`]: Pure rendering functions (no side effects)
//! - [`writers`]: Single-format effect writers
//! - [`compose`]: Multi-format composition utilities
//! - [`report`]: Complete report generation
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::io::writers::effects::{write_markdown_effect, write_multi_format_effect};
//! use debtmap::effects::run_effect;
//! use debtmap::config::DebtmapConfig;
//!
//! // Write to markdown file
//! let effect = write_markdown_effect(results.clone(), "report.md".into());
//! run_effect(effect, DebtmapConfig::default())?;
//!
//! // Write to multiple formats at once
//! let config = OutputConfig::builder()
//!     .markdown("report.md")
//!     .json("report.json")
//!     .build();
//! let effect = write_multi_format_effect(results, &config);
//! run_effect(effect, DebtmapConfig::default())?;
//! ```

mod compose;
mod config;
mod render;
mod report;
mod writers;

// ============================================================================
// Public Re-exports
// ============================================================================

// Configuration types
pub use config::{OutputConfig, OutputConfigBuilder, OutputFormat, OutputResult};

// Pure rendering functions
pub use render::{
    render_html, render_json, render_markdown, render_risk_json, render_risk_markdown,
    render_terminal,
};

// Single-format effect writers
pub use writers::{
    write_html_effect, write_json_effect, write_markdown_effect, write_risk_json_effect,
    write_risk_markdown_effect, write_risk_terminal_effect, write_terminal_effect,
};

// Composition utilities
pub use compose::{render_to_string_effect, write_multi_format_effect};

// Report generation
pub use report::{write_analysis_report_effect, ReportConfig, ReportConfigBuilder, ReportResult};
