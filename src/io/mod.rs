pub mod destinations;
pub mod effects;
pub mod output;
pub mod pattern_output;
pub mod real;
pub mod traits;
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
