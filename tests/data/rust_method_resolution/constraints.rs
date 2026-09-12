struct Timeline;
struct PyTimeline;
impl Timeline { fn new() -> Self { Timeline } }
impl PyTimeline { fn bar(&self) {} }
fn suffix_collision() { Timeline::new().bar(); }
struct Associated;
impl Associated { fn bar() {} }
fn dotted_associated(a: Associated) { a.bar(); }
fn bar() {}
fn unknown_singleton(a: Missing) { a.unique(); }
struct Singleton;
impl Singleton { fn unique(&self) {} }
fn zero_candidates(a: Missing) { a.not_in_project(); }
mod left {
    pub struct Same;
    impl Same { pub fn run(&self) {} }
    pub fn caller(a: Same) { a.run(); }
}
mod right {
    pub struct Same;
    impl Same { pub fn run(&self) {} }
    pub fn caller(a: Same) { a.run(); }
}
use left::Same as Left;
fn alias_import(a: Left) { a.run(); }
