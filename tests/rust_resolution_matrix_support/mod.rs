//! Test-only specifications shared by independently named matrix rows.
// Each test target exercises a different subset of the common expectation API.
#![allow(dead_code)]
mod checks;
mod oracle;
mod paths;

use debtmap::priority::call_graph::UncertaintyReason;

#[derive(Clone, Debug)]
pub enum Outcome {
    Resolved,
    Uncertain,
    Absent,
}

#[derive(Clone, Debug)]
pub struct Expectation {
    pub site: &'static str,
    pub token: &'static str,
    pub caller: &'static str,
    pub outcome: Outcome,
    pub targets: Vec<&'static str>,
    pub reason: Option<UncertaintyReason>,
}

impl Expectation {
    pub fn resolved(site: &'static str, token: &'static str, targets: &[&'static str]) -> Self {
        Self::new(site, token, targets, Outcome::Resolved)
    }
    pub fn uncertain(site: &'static str, token: &'static str, targets: &[&'static str]) -> Self {
        Self::new(site, token, targets, Outcome::Uncertain)
    }
    pub fn absent(site: &'static str, token: &'static str) -> Self {
        Self::new(site, token, &[], Outcome::Absent)
    }
    fn new(
        site: &'static str,
        token: &'static str,
        targets: &[&'static str],
        outcome: Outcome,
    ) -> Self {
        Self {
            site,
            token,
            targets: targets.to_vec(),
            outcome,
            caller: "caller",
            reason: None,
        }
    }
    pub fn caller(mut self, caller: &'static str) -> Self {
        self.caller = caller;
        self
    }
    pub fn reason(mut self, reason: UncertaintyReason) -> Self {
        self.reason = Some(reason);
        self
    }
}

pub fn verify(name: &str, source: &str, expected: &[Expectation]) -> Result<(), String> {
    verify_files(name, &[("src/lib.rs", source)], expected)
}

/// Inspect a synthetic graph to test the oracle's rejection behavior itself.
pub fn inspect(
    source: &str,
    graph: &debtmap::priority::call_graph::CallGraph,
    expected: &[Expectation],
) -> Vec<String> {
    let files = vec![(std::path::PathBuf::from("src/lib.rs"), source.to_string())];
    checks::check(graph, &oracle::Oracle::new(&files, expected), expected)
}

pub fn verify_files(
    name: &str,
    files: &[(&str, &str)],
    expected: &[Expectation],
) -> Result<(), String> {
    let directory = tempfile::tempdir().unwrap();
    let files: Vec<_> = files
        .iter()
        .map(|(path, source)| {
            let path = directory.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, source).unwrap();
            (path, source.to_string())
        })
        .collect();
    let oracle = oracle::Oracle::new(&files, expected);
    let graphs = paths::graphs(directory.path(), &files);
    let mut failures = Vec::new();
    if paths::normalized(&graphs[4].1) != paths::normalized(&graphs[5].1) {
        failures.push("full sequential/parallel metadata differs".into());
    }
    for (mode, graph) in graphs {
        let mut merged = graph.clone();
        merged.merge(graph.clone());
        merged.merge(graph.clone());
        if paths::normalized(&graph) != paths::normalized(&merged) {
            failures.push(format!("{mode}: repeated merge changed full metadata"));
        }
        for (version, graph) in [
            (mode.to_string(), graph),
            (format!("{mode}/merged"), merged),
        ] {
            failures.extend(
                checks::check(&graph, &oracle, expected)
                    .into_iter()
                    .map(|error| format!("{version}: {error}")),
            );
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "CASE {name}\n{}\nSOURCE:\n{}",
            failures.join("\n"),
            files
                .iter()
                .map(|(_, source)| source.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
}
