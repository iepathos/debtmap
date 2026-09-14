use super::support::Expectation as E;

#[path = "constructors.rs"]
mod constructors;
#[path = "interactions.rs"]
mod interactions;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    Valid,
    // These inputs deliberately have conflicting bindings; no Rust build is implied.
    Ambiguous,
}

pub struct Case {
    pub name: String,
    pub kind: SourceKind,
    pub source: String,
    pub expected: Vec<E>,
}

pub fn all() -> Vec<Case> {
    constructors::rows()
        .into_iter()
        .chain(interactions::rows())
        .collect()
}

const OTHER: &str = "struct Other; impl Other { /*@other_hit*/ fn hit(&self) {} }";

fn constructor(module: &str, shape: &str, label: &str) -> String {
    format!(
        "mod {module} {{ pub struct Item{shape}; impl Item {{ /*@{label}*/ pub fn hit(&self) {{}} }} }}"
    )
}

fn source(declarations: &str, imports: &str, expression: &str) -> String {
    format!(
        "{declarations}\n{OTHER}\n{imports}\n/*@caller*/ fn caller() {{\n    let value = /*#value*/ {expression};\n    /*#method*/ value.hit();\n}}\n"
    )
}

fn resolved_constructor(
    name: impl Into<String>,
    declarations: &str,
    imports: &str,
    expression: &'static str,
    target: &'static str,
) -> Case {
    Case {
        name: name.into(),
        kind: SourceKind::Valid,
        source: source(declarations, imports, expression),
        expected: vec![
            E::absent("value", expression),
            E::resolved("method", "hit(", &[target]),
        ],
    }
}

fn ambiguous_constructor(
    name: &str,
    declarations: &str,
    imports: &str,
    expression: &'static str,
) -> Case {
    Case {
        name: name.into(),
        kind: SourceKind::Ambiguous,
        source: source(declarations, imports, expression),
        expected: vec![
            E::absent("value", expression),
            E::uncertain("method", "hit(", &["left_hit", "right_hit"]),
        ],
    }
}
