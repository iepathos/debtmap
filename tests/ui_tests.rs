/// UI tests for compile-time guarantees using trybuild
///
/// These tests verify that the type-state pattern prevents invalid
/// usage at compile time. Tests in the ui/ directory should fail to
/// compile, demonstrating that the type system enforces correct usage.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();

    // This test should fail to compile because unvalidated config
    // cannot be executed - the type system prevents it
    t.compile_fail("tests/ui/unvalidated_execute.rs");
}
