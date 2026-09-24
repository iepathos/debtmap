use super::*;

pub(super) fn rows() -> Vec<Case> {
    let mut rows = Vec::new();
    for (form, expression) in FORMS {
        rows.push(returned(form, expression, false));
        rows.push(returned(form, expression, true));
    }
    for (name, setup, expression) in [
        ("copy", "let y = x;", "y.hit()"),
        ("tuple", "let y = (x,);", "y.0.hit()"),
        ("tuple_deref", "let y = (x,);", "(*y.0).hit()"),
        ("block_tail", "", "({ x }).hit()"),
        (
            "branch_join",
            "let y = if flag { x } else { x };",
            "y.hit()",
        ),
        (
            "branch_join_deref",
            "let y = if flag { x } else { x };",
            "(*y).hit()",
        ),
    ] {
        rows.push(shadowed(name, setup, expression));
    }
    for deref in [false, true] {
        rows.push(constructor_field(deref));
        rows.push(returned_field(deref));
    }
    rows
}

fn returned(form: &str, expression: &str, alias: bool) -> Case {
    let output = if alias { "Dyn" } else { "&'static dyn T" };
    let source = format!(
        "mod provider {{ pub trait T {{ fn hit(&self); }} struct A; impl T for A {{ /*@a_hit*/ fn hit(&self) {{}} }} static VALUE: A = A; type Dyn = &'static dyn T; /*@make*/ pub fn make() -> {output} {{ &VALUE }} }}\ntrait T {{ fn hit(&self); }} struct B; impl T for B {{ /*@b_hit*/ fn hit(&self) {{}} }}\n/*@caller*/ fn caller() {{ let x = /*#make*/ provider::make(); /*#hit*/ {expression}; }}"
    );
    Case {
        name: format!("returned-bound/alias={alias}/{form}"),
        source,
        expected: vec![
            E::resolved("make", "provider::", &["make"]),
            E::uncertain("hit", "hit(", &["a_hit"]),
        ],
    }
}

fn shadowed(name: &str, setup: &str, expression: &str) -> Case {
    uncertain(
        format!("shadow-propagation/{name}"),
        format!(
            "trait T {{ fn hit(&self); }} struct A; impl T for A {{ /*@a_hit*/ fn hit(&self) {{}} }} struct B;\n/*@caller*/ fn caller(flag: bool) {{ trait T {{ fn hit(&self); }} impl T for B {{ fn hit(&self) {{}} }} let x: &dyn T = &B; {setup} /*#hit*/ {expression}; }}"
        ),
        &[],
    )
}

fn constructor_field(deref: bool) -> Case {
    let expression = if deref { "(*y.0).hit()" } else { "y.0.hit()" };
    Case {
        name: format!("shadow-propagation/constructor-field/deref={deref}"),
        source: format!(
            "trait T {{ fn hit(&self); }} struct A; impl T for A {{ /*@a_hit*/ fn hit(&self) {{}} }} struct B; struct Holder<U>(U);\n/*@caller*/ fn caller() {{ trait T {{ fn hit(&self); }} impl T for B {{ fn hit(&self) {{}} }} let x: &dyn T = &B; let y = /*#ctor*/ Holder::<&dyn T>(x); /*#hit*/ {expression}; }}"
        ),
        expected: vec![
            E::absent("ctor", "Holder::"),
            E::uncertain("hit", "hit(", &[]),
        ],
    }
}

fn returned_field(deref: bool) -> Case {
    let expression = if deref {
        "(*x.inner).hit()"
    } else {
        "x.inner.hit()"
    };
    Case {
        name: format!("returned-bound/field/deref={deref}"),
        source: format!(
            "mod provider {{ pub trait T {{ fn hit(&self); }} struct A; impl T for A {{ /*@a_hit*/ fn hit(&self) {{}} }} static VALUE: A = A; pub struct Holder {{ pub inner: &'static dyn T }} /*@make*/ pub fn make() -> Holder {{ Holder {{ inner: &VALUE }} }} }}\ntrait T {{ fn hit(&self); }} struct B; impl T for B {{ /*@b_hit*/ fn hit(&self) {{}} }}\n/*@caller*/ fn caller() {{ let x = /*#make*/ provider::make(); /*#hit*/ {expression}; }}"
        ),
        expected: vec![
            E::resolved("make", "provider::", &["make"]),
            E::uncertain("hit", "hit(", &["a_hit"]),
        ],
    }
}
