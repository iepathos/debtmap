use super::*;

pub(super) fn rows() -> Vec<Case> {
    let mut rows = Vec::new();
    for shape in ["plain", "tuple", "reference"] {
        for qualified in [false, true] {
            for qself in [false, true] {
                rows.push(owner(shape, qualified, qself));
            }
        }
    }
    for qualified in [false, true] {
        for trait_qualified in [false, true] {
            rows.push(trait_argument(qualified, trait_qualified));
        }
    }
    rows
}

fn argument(shape: &str, name: &str) -> String {
    match shape {
        "tuple" => format!("({name},)"),
        "reference" => format!("&'static {name}"),
        _ => name.into(),
    }
}

fn owner(shape: &str, qualified: bool, qself: bool) -> Case {
    let a = argument(shape, "A");
    let b = argument(shape, "B");
    let query = argument(shape, if qualified { "crate::A" } else { "A" });
    let expression = if qself {
        format!("<W<{query}>>::hit()")
    } else {
        format!("W::<{query}>::hit()")
    };
    let expected = if qualified {
        E::resolved("hit", if qself { "<W" } else { "W::" }, &["a_hit"])
    } else {
        E::uncertain("hit", if qself { "<W" } else { "W::" }, &[])
    };
    Case {
        name: format!("generic-owner/{shape}/qualified={qualified}/qself={qself}"),
        source: format!(
            "struct A; struct B; struct W<T>(T);\nimpl W<{a}> {{ /*@a_hit*/ fn hit() {{}} }}\nimpl W<{b}> {{ /*@b_hit*/ fn hit() {{}} }}\n/*@caller*/ fn caller() {{ type A = B; /*#hit*/ {expression}; }}"
        ),
        expected: vec![expected],
    }
}

fn trait_argument(qualified: bool, trait_qualified: bool) -> Case {
    let arg = if qualified { "crate::A" } else { "A" };
    let name = if trait_qualified { "crate::T" } else { "T" };
    // Generic trait solving remains uncertain, but its admissible owner is constrained.
    Case {
        name: format!("trait-argument/qualified={qualified}/trait-qualified={trait_qualified}"),
        source: format!(
            "struct A; struct B; struct Owner; trait T<X> {{ fn hit(); }}\nimpl T<A> for Owner {{ /*@a_hit*/ fn hit() {{}} }}\nimpl T<B> for Owner {{ /*@b_hit*/ fn hit() {{}} }}\n/*@caller*/ fn caller() {{ type A = B; /*#hit*/ <Owner as {name}<{arg}>>::hit(); }}"
        ),
        expected: vec![E::uncertain(
            "hit",
            "<Owner",
            if qualified { &["a_hit"] } else { &[] },
        )],
    }
}
