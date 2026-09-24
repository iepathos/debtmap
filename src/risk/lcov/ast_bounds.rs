//! Immutable source bounds supplied by the metrics workspace for propagation.
use crate::core::FunctionMetrics;
use crate::risk::path_normalization::normalize_path_components;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

type DefinitionBounds = Option<(usize, Option<usize>)>;
type NamedBounds = HashMap<String, DefinitionBounds>;

#[derive(Debug, Clone, Default)]
pub(crate) struct AstBounds {
    files: HashMap<PathBuf, HashMap<usize, NamedBounds>>,
}

fn normalized(path: &Path) -> PathBuf {
    normalize_path_components(path).iter().collect()
}

impl AstBounds {
    pub fn from_metrics(metrics: &[FunctionMetrics]) -> Self {
        let mut bounds = Self::default();
        for metric in metrics {
            if metric.line == 0 || metric.length == 0 {
                continue;
            }
            let value = (
                metric.line.saturating_add(metric.length.saturating_sub(1)),
                metric.column,
            );
            bounds
                .files
                .entry(normalized(&metric.file))
                .or_default()
                .entry(metric.line)
                .or_default()
                .entry(metric.name.clone())
                .and_modify(|existing| {
                    if *existing != Some(value) {
                        *existing = None;
                    }
                })
                .or_insert(Some(value));
        }
        bounds
    }

    /// Exact normalized file/name/start identity; `Some(None)` means ambiguity.
    /// Missing bindings retain the legacy LCOV symbol lookup behavior.
    pub fn get(&self, file: &Path, name: &str, line: usize) -> Option<Option<usize>> {
        self.files
            .get(&normalized(file))?
            .get(&line)?
            .get(name)
            .map(|entry| entry.map(|(end, _)| end))
    }
}
