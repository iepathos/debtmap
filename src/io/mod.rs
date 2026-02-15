//! Input/output operations and file system abstractions.
//!
//! This module provides I/O utilities including file operations, output destinations,
//! and progress reporting. It follows the functional programming principle of keeping
//! I/O at system boundaries with pure computation in core modules.
//!
//! # Key Components
//!
//! - **File operations**: Read, write, and check file/directory existence
//! - **Output destinations**: Abstracted output targets (stdout, files, memory)
//! - **Coverage loaders**: Load test coverage data from LCOV files
//! - **Progress reporting**: Track analysis progress for user feedback
//! - **Directory walking**: Iterate over files in a directory tree

pub mod destinations;
pub mod effects;
pub mod output;
pub mod pattern_output;
pub mod progress;
pub mod real;
pub mod traits;
pub mod view_formatters;
pub mod walker;
pub mod writers;

// Re-export I/O traits for convenient access
pub use destinations::{
    FileDestination, MemoryDestination, OutputDestination, StderrDestination, StdoutDestination,
};
pub use real::{MemoryCache, NoOpCache, RealCoverageLoader, RealFileSystem};
pub use traits::{Cache, CoverageData, CoverageLoader, FileCoverage, FileSystem};

use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn read_file(path: &Path) -> Result<String> {
    Ok(fs::read_to_string(path)?)
}

pub fn write_file(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content)?;
    Ok(())
}

pub fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn file_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

pub fn dir_exists(path: &Path) -> bool {
    path.exists() && path.is_dir()
}
