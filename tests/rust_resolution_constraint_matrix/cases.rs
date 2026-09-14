//! Explicit receiver contracts paired across equivalent, compiling expressions.
use crate::support::Expectation as E;
#[path = "cases/generics.rs"]
mod generics;
#[path = "cases/propagation.rs"]
mod propagation;
#[path = "cases/receivers.rs"]
mod receivers;

pub struct Case {
    pub name: String,
    pub source: String,
    pub expected: Vec<E>,
}

pub const FORMS: [(&str, &str); 6] = [
    ("direct", "x.hit()"),
    ("parenthesized", "(x).hit()"),
    ("dereferenced", "(*x).hit()"),
    ("reborrowed", "(&*x).hit()"),
    ("borrow_then_deref", "(*(&x)).hit()"),
    ("double_deref", "(**(&x)).hit()"),
];

pub fn all() -> Vec<Case> {
    receivers::rows()
        .into_iter()
        .chain(generics::rows())
        .chain(propagation::rows())
        .collect()
}

fn uncertain(name: String, source: String, targets: &[&'static str]) -> Case {
    Case {
        name,
        source,
        expected: vec![E::uncertain("hit", "hit(", targets)],
    }
}
