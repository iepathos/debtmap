//! Relocation and irrelevant declarations must not alter established outcomes.
#[path = "rust_resolution_matrix_support/mod.rs"]
mod support;
use support::Expectation as E;

#[test]
fn layout_module_and_distractor_variants_preserve_exact_targets() {
    let source = "struct A; struct B; struct W<T>(T);\nimpl W<A> { /*@left*/ fn hit() {} }\nimpl W<B> { /*@right*/ fn hit() {} }\n/*@caller*/ fn caller() { /*#hit*/ W::<A>::hit(); }\n";
    let variants = [
        ("original", source.to_string()),
        ("same_line", source.replace('\n', " ")),
        (
            "indented",
            source.lines().map(|line| format!("    {line}\n")).collect(),
        ),
        ("inline_module", format!("mod nested {{\n{source}\n}}")),
        (
            "unrelated_owner",
            format!("{source}\nstruct Other; impl Other {{ /*@noise*/ fn hit() {{}} }}"),
        ),
    ];
    let failures: Vec<_> = variants
        .iter()
        .filter_map(|(name, source)| {
            support::verify(name, source, &[E::resolved("hit", "W::", &["left"])]).err()
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn file_order_and_unrelated_files_preserve_exact_targets() {
    let files = [
        (
            "src/lib.rs",
            "mod remote; mod noise; /*@caller*/ fn caller(x: &remote::Owner) { /*#hit*/ x.hit(); }",
        ),
        (
            "src/remote.rs",
            "pub struct Owner; impl Owner { /*@remote*/ pub fn hit(&self) {} }",
        ),
        (
            "src/noise.rs",
            "pub struct Owner; impl Owner { /*@noise*/ pub fn hit(&self) {} }",
        ),
    ];
    let orders = [[0, 1, 2], [2, 1, 0], [1, 0, 2]];
    let failures: Vec<_> = orders
        .iter()
        .filter_map(|order| {
            let sources: Vec<_> = order.iter().map(|index| files[*index]).collect();
            support::verify_files(
                &format!("file_order_{order:?}"),
                &sources,
                &[E::resolved("hit", "hit(", &["remote"])],
            )
            .err()
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
