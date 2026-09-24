struct Concrete;
trait First { fn perform(&self); }
trait Second { fn perform(&self); }
impl First for Concrete { fn perform(&self) {} }
impl Second for Concrete { fn perform(&self) {} }
fn qualified(c: &Concrete) { <Concrete as First>::perform(c); }
fn competing(c: Concrete) { c.perform(); }
fn dynamic(c: &dyn First) { c.perform(); }
fn generic_trait<T: First>(c: T) { c.perform(); }
trait Blanket { fn blanket(&self); }
impl<T> Blanket for T { fn blanket(&self) {} }
fn unsupported_blanket(c: Concrete) { c.blanket(); }
type Recursive = Recursive;
fn recursive_alias(c: Recursive) { c.perform(); }
