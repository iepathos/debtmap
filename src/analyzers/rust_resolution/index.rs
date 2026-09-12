//! Immutable declaration index and bounded, identity based candidate lookup.

use super::types::{DeclarationId, TypeFact, UnknownReason};
use crate::analyzers::call_graph::module_tree::ModuleTree;
use crate::priority::call_graph::{CallEdgeProvenance, FunctionId};
use quote::ToTokens;

#[path = "index/adjustments.rs"]
mod adjustments;
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
    pub signature: syn::Signature,
    pub owner: Option<TypeFact>,
    pub trait_path: Option<Vec<String>>,
    pub trait_type: Option<TypeFact>,
    pub kind: CallableKind,
    pub requirements_known: bool,
    pub body: Option<syn::Block>,
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
}

impl Default for Lookup<'_> {
    fn default() -> Self {
        Self {
            candidates: Vec::new(),
            justified: false,
            provenance: CallEdgeProvenance::AstDirect,
        }
    }
}

#[derive(Clone)]
struct TypeDeclaration {
    id: DeclarationId,
    context: Context,
    generics: Vec<String>,
    fields: HashMap<String, syn::Type>,
    alias: Option<syn::Type>,
    is_trait: bool,
    unit: bool,
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
    ty: syn::Type,
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
    values: Vec<ValueDeclaration>,
    value_paths: HashMap<Vec<String>, Vec<usize>>,
    callables: Vec<Callable>,
    imports: Vec<Import>,
    contexts: HashMap<PathBuf, Vec<Context>>,
    roots: HashMap<PathBuf, HashSet<PathBuf>>,
    type_paths: HashMap<Vec<String>, Vec<usize>>,
    declaration_positions: HashMap<DeclarationId, usize>,
    callable_names: HashMap<String, Vec<usize>>,
    imports_context: HashMap<Context, Vec<usize>>,
    imports_module: HashMap<Vec<String>, Vec<usize>>,
}

impl WorkspaceIndex {
    pub fn build(files: &[(PathBuf, syn::File)]) -> Self {
        let mut index = Self::default();
        index.establish_modules(files);
        for (file, ast) in files {
            let context = index.context(file, &[]);
            index.collect_types(&ast.items, &context);
        }
        index.index_declarations();
        for (file, ast) in files {
            let context = index.context(file, &[]);
            index.collect_callables(&ast.items, &context, &[]);
        }
        index
            .callables
            .sort_by(|left, right| left.id.cmp(&right.id));
        index.index_callable_names();
        index
    }

    pub fn callables(&self) -> &[Callable] {
        &self.callables
    }

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
            self.callable_names
                .entry(callable.signature.ident.to_string())
                .or_default()
                .push(position);
        }
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
