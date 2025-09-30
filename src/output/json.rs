use crate::priority;
use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn output_json(
    analysis: &priority::UnifiedAnalysis,
    output_file: Option<PathBuf>,
) -> Result<()> {
    let json = serde_json::to_string_pretty(analysis)?;
    if let Some(path) = output_file {
        if let Some(parent) = path.parent() {
            crate::io::ensure_dir(parent)?;
        }
        let mut file = fs::File::create(path)?;
        file.write_all(json.as_bytes())?;
    } else {
        println!("{json}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::call_graph::CallGraph;
    use tempfile::TempDir;

    #[test]
    fn test_output_json_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir
            .path()
            .join("nested")
            .join("subdirs")
            .join("output.json");

        let call_graph = CallGraph::new();
        let analysis = priority::UnifiedAnalysis::new(call_graph);

        let result = output_json(&analysis, Some(nested_path.clone()));
        assert!(
            result.is_ok(),
            "Failed to write JSON to nested path: {:?}",
            result.err()
        );
        assert!(
            nested_path.exists(),
            "Output file was not created at nested path"
        );

        let content = fs::read_to_string(&nested_path).unwrap();
        assert!(!content.is_empty(), "Output file is empty");
    }
}
