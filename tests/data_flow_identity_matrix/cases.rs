use debtmap::data_flow::DataFlowGraph;
use debtmap::priority::call_graph::{CallGraph, FunctionId};

pub struct Case {
    pub name: &'static str,
    pub incoming: FunctionId,
    pub matches: bool,
    pub tied: bool,
}

pub fn target() -> FunctionId {
    FunctionId::new("src/owner.rs".into(), "Owner::work".into(), 10).with_column(Some(7))
}

pub fn unrelated() -> FunctionId {
    FunctionId::new("src/unrelated.rs".into(), "guard".into(), 70).with_column(Some(5))
}

pub fn tied() -> FunctionId {
    target().with_column(Some(37))
}

pub fn cases() -> Vec<Case> {
    let legacy = target().with_column(None);
    let mut foreign = legacy.clone();
    foreign.file = "src/another.rs".into();
    let mut line = legacy.clone();
    line.line = 90;
    let renamed = |name: &str| FunctionId {
        name: name.into(),
        ..legacy.clone()
    };
    [
        ("exact", target(), true, false),
        ("unique_legacy", legacy.clone(), true, false),
        ("foreign_file", foreign, false, false),
        ("different_line", line, false, false),
        ("different_name", renamed("Owner::other"), false, false),
        ("different_owner", renamed("Other::work"), false, false),
        (
            "different_module_qualification",
            renamed("nested::Owner::work"),
            false,
            false,
        ),
        (
            "different_column",
            target().with_column(Some(99)),
            false,
            false,
        ),
        ("same_line_tie", legacy, false, true),
    ]
    .into_iter()
    .map(|(name, incoming, matches, tied)| Case {
        name,
        incoming,
        matches,
        tied,
    })
    .collect()
}

pub fn graph(case: &Case) -> DataFlowGraph {
    let mut graph = CallGraph::new();
    for id in [target(), unrelated()]
        .into_iter()
        .chain(case.tied.then(tied))
    {
        graph.add_function(id, false, false, 1, 1);
    }
    DataFlowGraph::from_call_graph(graph)
}

pub fn check<T: std::fmt::Debug + PartialEq>(
    failures: &mut Vec<String>,
    label: &str,
    actual: T,
    expected: T,
) {
    if actual != expected {
        failures.push(format!("{label}: expected {expected:?}, got {actual:?}"));
    }
}

#[derive(Default)]
pub struct Cells {
    pub total: usize,
    failed: Vec<String>,
    previous_failure_count: usize,
}

impl Cells {
    pub fn record(&mut self, name: String, failures: &[String]) {
        self.total += 1;
        if failures.len() > self.previous_failure_count {
            self.failed.push(name);
        }
        self.previous_failure_count = failures.len();
    }
}

pub fn finish(group: &str, cells: Cells, failures: Vec<String>) {
    println!(
        "{group}: {} cells; {} passed; {} failed",
        cells.total,
        cells.total - cells.failed.len(),
        cells.failed.len()
    );
    for name in &cells.failed {
        println!("{group}: FAILED CELL {name}");
    }
    assert!(
        failures.is_empty(),
        "{group}: {} matrix cells evaluated; {} failed checks:\n{}",
        cells.total,
        failures.len(),
        failures.join("\n")
    );
}
