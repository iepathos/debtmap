// This test should fail to compile because we're attempting to execute
// an unvalidated configuration. The type-state pattern prevents this at
// compile time, ensuring validation always occurs before execution.

use debtmap::commands::{AnalyzeConfig, Unvalidated};
use debtmap::cli;
use debtmap::formatting::FormattingConfig;
use std::path::PathBuf;

fn main() {
    // Create unvalidated config
    let config: AnalyzeConfig<Unvalidated> = AnalyzeConfig::new(
        PathBuf::from("."),
        cli::OutputFormat::Terminal,
        None,
        10,
        5,
        None,
        None,
        false,
        None,
        None,
        None,
        None,
        false,
        false,
        0,
        false,
        false,
        false,
        None,
        None,
        None,
        false,
        None,
        FormattingConfig::default(),
        true,
        4,
        false,
        false,
        None,
        false,
        false,
        None,
        None,
        false,
        None,
        false,
        false,
        0.5,
        false,
        None,
        0.5,
        false,
        false,
        None,
        false,
        cli::DebugFormatArg::Text,
        false,
        false,
        false,
        100,
        100,
        false,
        false,
        false,
        None,
        3,
        20,
        false,
        false,
    );

    // This should fail to compile - execute() is not available on Unvalidated config
    config.execute().unwrap();
}
