// Source fixtures are parsed without invoking rustc. Unsupported expressions
// deliberately model partial workspaces and must preserve uncertainty.
struct A;
struct B;
struct Named { value: A }
struct Pair(A);
struct Wrapper<T> { value: T }
struct Interpreter<'a> { value: &'a A }
impl A {
    fn bar(&self) {}
    fn new() -> Self { A }
    fn again(&self) -> Self { A }
    fn clone(&self) -> Self { A }
    fn get(&self) -> Self { A }
    fn any(&self) -> bool { true }
    fn self_call(&self) { self.bar(); }
}
impl B { fn bar(&self) {} fn new() -> A { A } }
impl<T> Wrapper<T> { fn get(&self) -> T { unavailable() } }
impl<'a> Interpreter<'a> { fn execute(&self) {} }
fn bar() {}
fn make() -> A { A }
async fn make_async() -> A { A }
fn borrowed(a: &A) { a.bar(); }
fn annotated() { let a: A = A; a.bar(); }
fn constructed() { let a = A::new(); a.bar(); }
fn factory() { let a = make(); a.bar(); }
fn alias(a: &A) { let b = a; b.bar(); }
fn referenced(a: A) { let b = &a; (*b).bar(); }
fn literal() { let a = A {}; a.bar(); }
fn unit() { let a = A; a.bar(); }
fn named_field(n: Named) { n.value.bar(); }
fn tuple_field(p: Pair) { p.0.bar(); }
fn tuple_local(a: A) { let pair = (a, 1); pair.0.bar(); }
fn returned(a: A) { a.again().bar(); }
async fn awaited() { make_async().await.bar(); }
fn generic(w: Wrapper<A>) { w.get().bar(); }
fn lifetime(i: Interpreter<'_>) { i.execute(); }
fn new_returns_other() { B::new().bar(); }
fn project_names(a: A) { a.clone().get().bar(); a.any(); }
fn initializer(a: A) { let a = a.again(); a.bar(); }
fn unknown_shadow(a: A) { { let a = missing(); a.bar(); } a.bar(); }
fn assignment() { let mut a = A; a = B; a.bar(); }
fn branch_invalidation(flag: bool) { let mut a = A; if flag { a = B; } a.bar(); }
fn loop_invalidation() { let mut a = A; loop { a = B; break; } a.bar(); }
fn match_binding(a: A, input: Unknown) { match input { Some(a) => a.bar(), _ => {} } }
fn loop_binding(a: A, input: Unknown) { for a in input { a.bar(); } }
fn conditional_binding(a: A, input: Unknown) { if let Some(a) = input { a.bar(); } }
fn destructured_binding(a: A, input: Unknown) { let (a, _) = input; a.bar(); }
fn unresolved_generic<T>(w: Wrapper<T>) { w.get().bar(); }
