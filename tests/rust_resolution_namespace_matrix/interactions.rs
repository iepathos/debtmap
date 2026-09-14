use super::*;

pub(super) fn rows() -> Vec<Case> {
    function_precedence()
        .into_iter()
        .chain(constant_precedence())
        .chain(mixed_globs())
        .collect()
}

fn function_precedence() -> Vec<Case> {
    [("unit", ""), ("tuple", "(pub u32)")].into_iter().flat_map(|(kind, shape)| {
    let left = constructor("left", shape, "left_hit");
    let imported = format!("{left}\nmod right {{ /*@function*/ pub fn Item(_: u32) -> crate::Other {{ crate::Other }} }}");
    [
        ("local_function_shadows_glob_constructor", format!("{left}\n/*@function*/ fn Item(_: u32) -> Other {{ Other }}"), "use left::*;"),
        ("explicit_function_shadows_glob_constructor", imported, "use left::*; use right::Item;"),
    ].into_iter().map(move |(name, declarations, imports)| Case {
        name: format!("{name}_{kind}"), kind: SourceKind::Valid,
        source: source(&declarations, imports, "Item(1)"),
        expected: vec![E::resolved("value", "Item(", &["function"]), E::resolved("method", "hit(", &["other_hit"])],
    })
    }).collect()
}

fn constant_precedence() -> Vec<Case> {
    [("unit", ""), ("tuple", "(pub u32)")]
        .into_iter()
        .flat_map(|(kind, shape)| {
            let left = constructor("left", shape, "left_hit");
            let imported =
                format!("{left}\nmod right {{ pub const Item: crate::Other = crate::Other; }}");
            [
                (
                    "local_constant_shadows_glob_constructor",
                    format!("{left}\nconst Item: Other = Other;"),
                    "use left::*;",
                ),
                (
                    "explicit_constant_shadows_glob_constructor",
                    imported,
                    "use left::*; use right::Item;",
                ),
            ]
            .into_iter()
            .map(move |(name, declarations, imports)| {
                resolved_constructor(
                    format!("{name}_{kind}"),
                    &declarations,
                    imports,
                    "Item",
                    "other_hit",
                )
            })
        })
        .collect()
}

fn mixed_globs() -> Vec<Case> {
    let tuple = constructor("left", "(pub u32)", "left_hit");
    let unit = constructor("left", "", "left_hit");
    vec![
        Case {
            name: "two_free_function_globs_keep_both_candidates".into(),
            kind: SourceKind::Ambiguous,
            source: "mod left { /*@left_function*/ pub fn Item(_: u32) {} }\nmod right { /*@right_function*/ pub fn Item(_: u32) {} }\nuse left::*; use right::*;\n/*@caller*/ fn caller() { /*#call*/ Item(1); }\n".into(),
            expected: vec![E::uncertain("call", "Item(", &["left_function", "right_function"])],
        },
        Case {
            name: "two_constant_globs_keep_both_owner_constraints".into(),
            kind: SourceKind::Ambiguous,
            source: source("mod left { pub struct Left; impl Left { /*@left_hit*/ pub fn hit(&self) {} } pub const Item: Left = Left; }\nmod right { pub struct Right; impl Right { /*@right_hit*/ pub fn hit(&self) {} } pub const Item: Right = Right; }", "use left::*; use right::*;", "Item"),
            expected: vec![E::absent("value", "Item"), E::uncertain("method", "hit(", &["left_hit", "right_hit"])],
        },
        Case {
            name: "function_and_tuple_constructor_globs_remain_ambiguous".into(),
            kind: SourceKind::Ambiguous,
            source: format!("{tuple}\nmod right {{ /*@function*/ pub fn Item(_: u32) {{}} }}\nuse left::*; use right::*;\n/*@caller*/ fn caller() {{ /*#call*/ Item(1); }}\n"),
            expected: vec![E::uncertain("call", "Item(", &["function"])],
        },
        Case {
            name: "function_and_unit_constructor_globs_remain_ambiguous".into(),
            kind: SourceKind::Ambiguous,
            source: source(&format!("{unit}\nmod right {{ /*@function*/ pub fn Item() {{}} }}"), "use left::*; use right::*;", "Item"),
            expected: vec![E::uncertain("value", "Item", &["function"]), E::uncertain("method", "hit(", &["left_hit"])],
        },
        Case {
            name: "constant_and_unit_constructor_globs_preserve_both_owners".into(),
            kind: SourceKind::Ambiguous,
            source: source(&format!("{unit}\nmod right {{ pub const Item: crate::Other = crate::Other; }}"), "use left::*; use right::*;", "Item"),
            expected: vec![E::absent("value", "Item"), E::uncertain("method", "hit(", &["left_hit", "other_hit"])],
        },
        resolved_constructor("explicit_constructor_disambiguates_function_glob", &format!("{tuple}\nmod right {{ /*@function*/ pub fn Item(_: u32) {{}} }}"), "use left::*; use right::*; use left::Item;", "Item(1)", "left_hit"),
        resolved_constructor("explicit_constructor_disambiguates_constant_glob", &format!("{unit}\nmod right {{ pub const Item: crate::Other = crate::Other; }}"), "use left::*; use right::*; use left::Item;", "Item", "left_hit"),
    ]
}
