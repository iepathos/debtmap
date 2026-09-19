//! Executable line observations, shared by exact AST coverage queries.
use crate::risk::path_normalization::normalize_path_components;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub(crate) struct LineCoverageIndex {
    files: HashMap<PathBuf, SourceLines>,
    aliases: HashMap<PathBuf, Option<PathBuf>>,
    summaries: HashMap<PathBuf, (usize, usize)>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SourceLines {
    pub lines: Vec<(usize, u64)>,
    covered_prefix: Vec<usize>,
}

impl SourceLines {
    fn new(lines: Vec<(usize, u64)>) -> Self {
        let covered_prefix = std::iter::once(0)
            .chain(lines.iter().scan(0, |count, (_, hits)| {
                *count += usize::from(*hits > 0);
                Some(*count)
            }))
            .collect();
        Self {
            lines,
            covered_prefix,
        }
    }

    fn range(&self, start: usize, end: usize) -> std::ops::Range<usize> {
        self.lines.partition_point(|(line, _)| *line < start)
            ..self.lines.partition_point(|(line, _)| *line <= end)
    }

    pub fn fraction(&self, start: usize, end: usize) -> f64 {
        let range = self.range(start, end);
        if range.is_empty() {
            return 0.0;
        }
        (self.covered_prefix[range.end] - self.covered_prefix[range.start]) as f64
            / range.len() as f64
    }

    pub fn uncovered(&self, start: usize, end: usize) -> Vec<usize> {
        self.lines[self.range(start, end)]
            .iter()
            .filter_map(|(line, hits)| (*hits == 0).then_some(*line))
            .collect()
    }
}

impl LineCoverageIndex {
    pub fn record_summary(&mut self, path: &Path, found: Option<usize>, hit: Option<usize>) {
        let summary = self
            .summaries
            .entry(normalize_path_components(path).iter().collect())
            .or_default();
        summary.0 = summary.0.max(found.unwrap_or(0));
        summary.1 = summary.1.max(hit.unwrap_or(0));
    }

    pub fn totals(&self) -> (usize, usize) {
        self.files
            .iter()
            .map(|(path, source)| {
                if source.lines.is_empty() {
                    self.summaries.get(path).copied().unwrap_or_default()
                } else {
                    (
                        source.lines.len(),
                        source.covered_prefix.last().copied().unwrap_or(0),
                    )
                }
            })
            .fold((0, 0), |(total, hit), (file_total, file_hit)| {
                (total + file_total, hit + file_hit)
            })
    }

    pub fn merge(&mut self, path: &Path, observations: &HashMap<usize, u64>) {
        let source = self
            .files
            .entry(normalize_path_components(path).iter().collect())
            .or_default();
        let mut lines: BTreeMap<_, _> = source.lines.iter().copied().collect();
        for (&line, &hits) in observations {
            lines
                .entry(line)
                .and_modify(|old| *old = (*old).max(hits))
                .or_insert(hits);
        }
        *source = SourceLines::new(lines.into_iter().collect());
    }

    /// Index only unambiguous suffixes; equal basenames must not select a file.
    pub fn build_aliases(&mut self) {
        self.aliases.clear();
        for path in self.files.keys() {
            let components: Vec<_> = path.components().collect();
            for start in 0..components.len() {
                let suffix: PathBuf = components[start..].iter().collect();
                self.aliases
                    .entry(suffix)
                    .and_modify(|existing| {
                        if existing.as_ref() != Some(path) {
                            *existing = None;
                        }
                    })
                    .or_insert_with(|| Some(path.clone()));
            }
        }
    }

    pub fn get(&self, path: &Path) -> Option<&SourceLines> {
        let components = normalize_path_components(path);
        let normalized: PathBuf = components.iter().collect();
        self.files
            .get(&normalized)
            .or_else(|| {
                self.aliases
                    .get(&normalized)?
                    .as_ref()
                    .and_then(|key| self.files.get(key))
            })
            .or_else(|| {
                (1..components.len()).find_map(|start| {
                    let suffix: PathBuf = components[start..].iter().collect();
                    self.files.get(&suffix)
                })
            })
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}
