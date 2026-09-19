//! Immutable declaration index and bounded, identity based candidate lookup.

use super::types::{DeclarationId, TypeFact, UnknownReason};
use crate::analyzers::call_graph::module_tree::ModuleTree;
use crate::priority::call_graph::{CallEdgeProvenance, FunctionId, UncertaintyReason};
use quote::ToTokens;

#[path = "index/adjustments.rs"]
mod adjustments;
#[path = "index/associated.rs"]
mod associated;
pub(super) use associated::AssociatedQuery;
#[path = "index/bounds.rs"]
mod bounds;
#[path = "index/syntax.rs"]
mod syntax;
#[path = "index/workspace.rs"]
mod workspace;
mod workspace_membership;
pub use syntax::{SignatureSyntax, TypeSyntax};
pub(crate) use workspace::DeclarationCollector;
#[path = "index/collect.rs"]
mod collect;
#[path = "index/expansion.rs"]
mod expansion;
#[path = "index/helpers.rs"]
mod helpers;
#[path = "index/lookup.rs"]
mod lookup;
#[path = "index/matching.rs"]
mod matching;
#[path = "index/modules.rs"]
mod modules;
#[path = "index/paths.rs"]
mod paths;
#[path = "index/propagation.rs"]
mod propagation;
#[cfg(test)]
#[path = "index/test_adapters.rs"]
mod test_adapters;
#[cfg(test)]
#[path = "index/tests.rs"]
mod tests;
#[path = "index/values.rs"]
mod values;

use adjustments::*;
use helpers::*;
use matching::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const EXPANSION_LIMIT: usize = 32;
pub type Substitutions = HashMap<String, TypeFact>;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Context {
    pub file: PathBuf,
    pub module: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallableKind {
    FreeFunction,
    AssociatedFunction,
    InherentMethod,
    TraitMethod,
    TraitDeclaration,
}

#[derive(Clone, Debug)]
pub struct Callable {
    pub id: FunctionId,
    pub context: Context,
    pub signature: SignatureSyntax,
    pub owner: Option<TypeFact>,
    pub trait_path: Option<Vec<String>>,
    pub trait_type: Option<TypeFact>,
    /// Raw declaration positions, including ambiguous and non-trait bindings.
    pub trait_candidates: Vec<usize>,
    pub kind: CallableKind,
    pub requirements_known: bool,
    pub has_body: bool,
    pub owner_syntax: Option<TypeSyntax>,
    pub trait_syntax: Option<TypeSyntax>,
    pub is_test: bool,
    pub substitutions: Substitutions,
    pub const_parameters: Vec<String>,
}

impl Callable {
    pub fn has_receiver(&self) -> bool {
        self.signature.receiver().is_some()
    }
}

pub struct Lookup<'a> {
    pub candidates: Vec<&'a Callable>,
    pub justified: bool,
    pub provenance: CallEdgeProvenance,
    pub reason: Option<UncertaintyReason>,
}

impl Default for Lookup<'_> {
    fn default() -> Self {
        Self {
            candidates: Vec::new(),
            justified: false,
            provenance: CallEdgeProvenance::AstDirect,
            reason: None,
        }
    }
}

#[derive(Clone)]
struct TypeDeclaration {
    id: DeclarationId,
    context: Context,
    generics: Vec<String>,
    fields: HashMap<String, TypeSyntax>,
    alias: Option<TypeSyntax>,
    is_trait: bool,
    unit: bool,
    tuple_arity: Option<usize>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum DeclarationKind {
    Struct,
    Enum,
    Union,
    Alias,
    Trait,
}

struct ValueDeclaration {
    context: Context,
    name: String,
    ty: TypeSyntax,
    line: usize,
    column: usize,
}

#[derive(Clone)]
struct Import {
    context: Context,
    alias: String,
    path: Vec<String>,
    glob: bool,
}

#[derive(Default)]
pub struct WorkspaceIndex {
    declarations: Vec<TypeDeclaration>,
    module_paths: HashSet<Context>,
    module_files: HashMap<Vec<String>, HashSet<PathBuf>>,
    values: Vec<ValueDeclaration>,
    value_paths: HashMap<Vec<String>, Vec<usize>>,
    callables: Vec<Callable>,
    imports: Vec<Import>,
    contexts: HashMap<PathBuf, Vec<Context>>,
    roots: HashMap<PathBuf, HashSet<PathBuf>>,
    workspace_membership: HashMap<PathBuf, usize>,
    type_paths: HashMap<Vec<String>, Vec<usize>>,
    declaration_positions: HashMap<DeclarationId, usize>,
    callable_names: HashMap<String, Vec<usize>>,
    free_callable_paths: HashMap<Vec<String>, Vec<usize>>,
    callable_positions: HashMap<PathBuf, HashMap<(usize, Option<usize>), usize>>,
    imports_context: HashMap<Context, Vec<usize>>,
    imports_module: HashMap<Vec<String>, Vec<usize>>,
}

impl WorkspaceIndex {
    pub fn build(files: &[(PathBuf, syn::File)]) -> Self {
        let known = files.iter().map(|(path, _)| path.clone()).collect();
        let mut collector = workspace::DeclarationCollector::new(known);
        collector.collect(files);
        collector.finish()
    }

    pub fn callables(&self) -> &[Callable] {
        &self.callables
    }

    #[cfg(test)]
    pub fn context(&self, file: &Path, inline_modules: &[String]) -> Context {
        let mut context = self
            .contexts
            .get(file)
            .and_then(|contexts| contexts.first())
            .cloned()
            .unwrap_or_else(|| Context {
                file: file.to_path_buf(),
                module: Vec::new(),
            });
        context.module.extend_from_slice(inline_modules);
        context
    }

    fn declaration(&self, id: &DeclarationId) -> Option<&TypeDeclaration> {
        self.declaration_positions
            .get(id)
            .map(|position| &self.declarations[*position])
    }

    fn index_declarations(&mut self) {
        for (position, value) in self.values.iter().enumerate() {
            self.value_paths
                .entry(qualified(&value.context.module, &value.name))
                .or_default()
                .push(position);
        }
        for (position, declaration) in self.declarations.iter().enumerate() {
            self.type_paths
                .entry(qualified(&declaration.context.module, &declaration.id.name))
                .or_default()
                .push(position);
            self.declaration_positions
                .insert(declaration.id.clone(), position);
        }
        for (position, import) in self.imports.iter().enumerate() {
            self.imports_module
                .entry(import.context.module.clone())
                .or_default()
                .push(position);
            self.imports_context
                .entry(import.context.clone())
                .or_default()
                .push(position);
        }
    }

    fn index_callable_names(&mut self) {
        for (position, callable) in self.callables.iter().enumerate() {
            if callable.kind == CallableKind::FreeFunction {
                self.free_callable_paths
                    .entry(qualified(
                        &callable.context.module,
                        &callable.signature.ident,
                    ))
                    .or_default()
                    .push(position);
            }
            self.callable_positions
                .entry(callable.id.file.clone())
                .or_default()
                .insert((callable.id.line, callable.id.column), position);
            self.callable_names
                .entry(callable.signature.ident.clone())
                .or_default()
                .push(position);
        }
    }

    pub fn callable_at(&self, file: &Path, line: usize, column: usize) -> Option<&Callable> {
        self.callable_positions
            .get(file)?
            .get(&(line, Some(column)))
            .map(|position| &self.callables[*position])
    }

    fn free_callables_at(&self, path: &[String]) -> impl Iterator<Item = &Callable> {
        self.free_callable_paths
            .get(path)
            .into_iter()
            .flatten()
            .map(|position| &self.callables[*position])
    }

    fn free_candidates(
        &self,
        paths: &[Vec<String>],
        context: &Context,
        name: &str,
    ) -> Vec<&Callable> {
        let mut positions: Vec<_> = paths
            .iter()
            .filter_map(|path| self.free_callable_paths.get(path))
            .flatten()
            .copied()
            .collect();
        positions.sort_unstable();
        positions.dedup();
        positions
            .into_iter()
            .map(|position| &self.callables[position])
            .filter(|call| {
                call.signature.ident == name
                    && self.same_workspace(&call.context.file, &context.file)
            })
            .collect()
    }

    fn callable_traits<'a>(
        &'a self,
        call: &'a Callable,
    ) -> impl Iterator<Item = &'a TypeDeclaration> {
        call.trait_candidates
            .iter()
            .map(|position| &self.declarations[*position])
    }

    fn named_callables(&self, name: &str) -> impl Iterator<Item = &Callable> {
        self.callable_names
            .get(name)
            .into_iter()
            .flatten()
            .map(|position| &self.callables[*position])
    }

    fn context_imports(&self, context: &Context) -> impl Iterator<Item = &Import> {
        self.imports_context
            .get(context)
            .into_iter()
            .flatten()
            .map(|position| &self.imports[*position])
    }

    fn module_imports(&self, module: &[String]) -> impl Iterator<Item = &Import> {
        self.imports_module
            .get(module)
            .into_iter()
            .flatten()
            .map(|position| &self.imports[*position])
    }
}
