use super::*;

pub(super) fn rows() -> Vec<Case> {
    type_only_namesakes()
        .into_iter()
        .chain(precedence())
        .chain(qualified_and_imported())
        .chain(competing_globs())
        .collect()
}

fn type_only_namesakes() -> Vec<Case> {
    let declarations = [
        ("alias", "type Item = Other;"),
        (
            "enum",
            "enum Item {} impl Item { /*@local_hit*/ fn hit(&self) {} }",
        ),
        (
            "braced_struct",
            "struct Item {} impl Item { /*@local_hit*/ fn hit(&self) {} }",
        ),
        (
            "union",
            "union Item { value: u32 } impl Item { /*@local_hit*/ fn hit(&self) {} }",
        ),
        ("trait", "trait Item {}"),
    ];
    [("unit", "", "Item"), ("tuple", "(pub u32)", "Item(1)")]
        .into_iter()
        .flat_map(|(kind, shape, expression)| {
            declarations.into_iter().map(move |(name, local)| {
                resolved_constructor(
                    format!("{kind}_constructor_coexists_with_local_{name}"),
                    &format!("{}\n{local}", constructor("left", shape, "left_hit")),
                    "use left::*;",
                    expression,
                    "left_hit",
                )
            })
        })
        .collect()
}

fn precedence() -> Vec<Case> {
    [("unit", "", "Item"), ("tuple", "(pub u32)", "Item(1)")].into_iter().flat_map(|(kind, shape, expression)| {
        let left = constructor("left", shape, "left_hit");
        let pair = format!("{left}\n{}", constructor("right", shape, "right_hit"));
        [
            resolved_constructor(format!("local_{kind}_constructor_shadows_glob"), &format!("{left}\nstruct Item{shape}; impl Item {{ /*@local_hit*/ fn hit(&self) {{}} }}"), "use left::*;", expression, "local_hit"),
            resolved_constructor(format!("explicit_{kind}_constructor_shadows_globs"), &pair, "use left::*; use right::*; use right::Item;", expression, "right_hit"),
            resolved_constructor(format!("duplicate_{kind}_globs_keep_one_identity"), &left, "use left::*; use left::*;", expression, "left_hit"),
        ]
    }).collect()
}

fn qualified_and_imported() -> Vec<Case> {
    [
        ("unit", "", "left::Item", "Renamed", "PublicItem"),
        (
            "tuple",
            "(pub u32)",
            "left::Item(1)",
            "Renamed(1)",
            "PublicItem(1)",
        ),
    ]
    .into_iter()
    .flat_map(|(kind, shape, qualified, alias, reexport)| {
        let left = constructor("left", shape, "left_hit");
        let pair = format!("{left}\n{}", constructor("right", shape, "right_hit"));
        [
            resolved_constructor(
                format!("qualified_{kind}_constructor_bypasses_glob_conflict"),
                &pair,
                "use left::*; use right::*;",
                qualified,
                "left_hit",
            ),
            resolved_constructor(
                format!("renamed_{kind}_constructor_retains_identity"),
                &left,
                "use left::Item as Renamed;",
                alias,
                "left_hit",
            ),
            resolved_constructor(
                format!("reexported_{kind}_constructor_retains_identity"),
                &format!("{left}\nmod bridge {{ pub use crate::left::Item as PublicItem; }}"),
                "use bridge::PublicItem;",
                reexport,
                "left_hit",
            ),
        ]
    })
    .collect()
}

fn competing_globs() -> Vec<Case> {
    [("unit", "", "Item"), ("tuple", "(pub u32)", "Item(1)")]
        .into_iter()
        .map(|(kind, shape, expression)| {
            ambiguous_constructor(
                &format!("competing_{kind}_globs_preserve_both_owners"),
                &format!(
                    "{}\n{}",
                    constructor("left", shape, "left_hit"),
                    constructor("right", shape, "right_hit")
                ),
                "use left::*; use right::*;",
                expression,
            )
        })
        .collect()
}
