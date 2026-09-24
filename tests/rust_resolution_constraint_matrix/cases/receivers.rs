use super::*;

pub(super) fn rows() -> Vec<Case> {
    let mut rows = Vec::new();
    for (form, expression) in FORMS {
        rows.push(dynamic(form, expression, false));
        rows.push(dynamic(form, expression, true));
        rows.push(nominal(form, expression, false));
        rows.push(nominal(form, expression, true));
        rows.push(primitive(form, expression, false));
        rows.push(primitive(form, expression, true));
    }
    rows
}

fn dynamic(form: &str, expression: &str, qualified: bool) -> Case {
    let (ty, value, targets) = if qualified {
        ("crate::T", "A", vec!["a_hit"])
    } else {
        ("T", "B", vec![])
    };
    uncertain(
        format!("dynamic/qualified={qualified}/{form}"),
        format!(
            "trait T {{ fn hit(&self); }}\nstruct A; impl T for A {{ /*@a_hit*/ fn hit(&self) {{}} }}\nstruct B;\n/*@caller*/ fn caller() {{ trait T {{ fn hit(&self); }} impl T for B {{ fn hit(&self) {{}} }} let x: &dyn {ty} = &{value}; /*#hit*/ {expression}; }}"
        ),
        &targets,
    )
}

fn nominal(form: &str, expression: &str, qualified: bool) -> Case {
    let (ty, value) = if qualified {
        ("crate::A", "crate::A")
    } else {
        ("A", "B")
    };
    let mut row = uncertain(
        format!("nominal/qualified={qualified}/{form}"),
        format!(
            "struct A; impl A {{ /*@a_hit*/ fn hit(&self) {{}} }}\nstruct B; impl B {{ /*@b_hit*/ fn hit(&self) {{}} }}\n/*@caller*/ fn caller() {{ type A = B; let x: &{ty} = &{value}; /*#hit*/ {expression}; }}"
        ),
        &[],
    );
    if qualified {
        row.expected = vec![E::resolved("hit", "hit(", &["a_hit"])];
    }
    row
}

fn primitive(form: &str, expression: &str, alias: bool) -> Case {
    let ty = if alias { "Number" } else { "u32" };
    uncertain(
        format!("primitive/alias={alias}/{form}"),
        format!(
            "type Number = u32;\ntrait T {{ fn hit(&self); }}\nimpl T for u32 {{ /*@primitive_hit*/ fn hit(&self) {{}} }}\nstruct Other; impl Other {{ /*@other_hit*/ fn hit(&self) {{}} }}\n/*@caller*/ fn caller(x: &{ty}) {{ /*#hit*/ {expression}; }}"
        ),
        &["primitive_hit"],
    )
}
