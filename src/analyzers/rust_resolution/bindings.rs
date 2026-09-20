//! Lexical bindings with explicit declarations kept apart from inferred values.
use super::body::unknown;
use super::types::{FactOrigin, TypeFact, UnknownReason};
use std::collections::{HashMap, HashSet};
mod annotations;
mod patterns;
use annotations::compatible;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Binding {
    fact: TypeFact,
    declared: Option<TypeFact>,
    origin: FactOrigin,
    callable: Option<crate::priority::call_graph::FunctionId>,
    closure: Option<usize>,
}

#[derive(Clone, Debug)]
pub(super) struct Bindings {
    scopes: Vec<HashMap<String, Binding>>,
    type_scopes: Vec<HashSet<String>>,
}

impl Default for Bindings {
    fn default() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            type_scopes: vec![HashSet::new()],
        }
    }
}

impl Bindings {
    pub fn push(&mut self) {
        self.scopes.push(HashMap::new());
        self.type_scopes.push(HashSet::new());
    }
    pub fn pop(&mut self) {
        self.scopes.pop();
        self.type_scopes.pop();
    }
    pub fn shadow_item(&mut self, name: String, types: bool, values: bool) {
        if types && let Some(scope) = self.type_scopes.last_mut() {
            scope.insert(name.clone());
        }
        if values {
            self.insert(name, unknown(), false);
        }
    }
    pub fn type_shadowed(&self, name: &str) -> bool {
        self.type_scopes
            .iter()
            .any(|scope| scope.contains(name) || scope.contains("*"))
    }
    pub fn value_shadowed(&self, name: &str) -> bool {
        self.scopes
            .iter()
            .any(|scope| scope.contains_key(name) || scope.contains_key("*"))
    }
    pub fn get(&self, name: &str) -> Option<TypeFact> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .map(|b| b.fact.clone())
    }
    pub fn insert(&mut self, name: String, fact: TypeFact, declared: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            let annotation = declared.then(|| match &fact {
                TypeFact::Uncertain { constraint, .. } => *constraint.clone(),
                _ => fact.clone(),
            });
            scope.insert(
                name,
                Binding {
                    declared: annotation,
                    fact,
                    origin: if declared {
                        FactOrigin::Declaration
                    } else {
                        FactOrigin::Propagation
                    },
                    callable: None,
                    closure: None,
                },
            );
        }
    }
    pub fn assign(&mut self, name: &str, fact: TypeFact) {
        if let Some(binding) = self.scopes.iter_mut().rev().find_map(|s| s.get_mut(name)) {
            binding.callable = None;
            binding.closure = None;
            binding.fact = match &binding.declared {
                Some(declared) if !compatible(declared, &fact) => TypeFact::Uncertain {
                    constraint: Box::new(declared.clone()),
                    reason: UnknownReason::UnsupportedTypeOperation,
                },
                Some(declared) => declared.clone(),
                None => fact,
            };
        }
    }
    pub fn join(&self, branches: &[Self]) -> Self {
        let mut joined = self.clone();
        for (depth, scope) in joined.scopes.iter_mut().enumerate() {
            for branch in branches
                .iter()
                .filter_map(|branch| branch.scopes.get(depth))
            {
                for (name, binding) in branch {
                    scope.entry(name.clone()).or_insert_with(|| binding.clone());
                }
            }
            for (name, binding) in scope {
                let values: Vec<_> = branches
                    .iter()
                    .filter_map(|b| b.scopes.get(depth)?.get(name))
                    .collect();
                if values.len() != branches.len()
                    || values
                        .iter()
                        .any(|value| value.callable != binding.callable)
                {
                    binding.callable = None;
                }
                if values.len() != branches.len()
                    || values.iter().any(|value| value.closure != binding.closure)
                {
                    binding.closure = None;
                }
                binding.fact = match values.first() {
                    Some(first)
                        if values.len() == branches.len()
                            && values.iter().all(|v| v.fact == first.fact) =>
                    {
                        first.fact.clone()
                    }
                    _ => binding
                        .declared
                        .as_ref()
                        .map(|constraint| TypeFact::Uncertain {
                            constraint: Box::new(constraint.clone()),
                            reason: UnknownReason::UnsupportedTypeOperation,
                        })
                        .unwrap_or_else(unknown),
                };
            }
        }
        joined
    }
    pub fn invalidate_writes(&mut self, names: &HashSet<String>) {
        for name in names {
            if let Some(binding) = self.scopes.iter_mut().rev().find_map(|s| s.get_mut(name))
                && binding.declared.is_none()
            {
                binding.fact = unknown();
                binding.callable = None;
                binding.closure = None;
            }
        }
    }

    pub fn bind_callable(&mut self, name: &str, target: crate::priority::call_graph::FunctionId) {
        if let Some(binding) = self.scopes.last_mut().and_then(|scope| scope.get_mut(name)) {
            binding.callable = Some(target);
        }
    }

    pub fn callable(&self, name: &str) -> Option<&crate::priority::call_graph::FunctionId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))?
            .callable
            .as_ref()
    }

    pub fn bind_closure(&mut self, name: &str, closure: usize) {
        if let Some(binding) = self.scopes.last_mut().and_then(|scope| scope.get_mut(name)) {
            binding.closure = Some(closure);
        }
    }

    pub fn closure(&self, name: &str) -> Option<usize> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))?
            .closure
    }

    pub fn captures_unchanged(&self, current: &Self) -> bool {
        self.scopes
            .iter()
            .flat_map(|scope| scope.keys())
            .all(|name| {
                let visible = |bindings: &Self| {
                    bindings
                        .scopes
                        .iter()
                        .rev()
                        .find_map(|scope| scope.get(name))
                        .cloned()
                };
                visible(self) == visible(current)
            })
    }
}
