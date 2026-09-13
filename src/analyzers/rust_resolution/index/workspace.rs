//! Two-pass collection. Only owned records survive an active parsing batch.
use super::*;

pub(crate) struct DeclarationCollector {
    index: WorkspaceIndex,
    known: HashMap<PathBuf, Vec<PathBuf>>,
    edges: Vec<(PathBuf, PathBuf, Vec<String>)>,
}

impl DeclarationCollector {
    pub fn new(paths: HashSet<PathBuf>) -> Self {
        let mut known = HashMap::<_, Vec<_>>::new();
        for path in paths {
            known
                .entry(modules::normalize_module_path(&path))
                .or_default()
                .push(path);
        }
        Self {
            index: WorkspaceIndex::default(),
            known,
            edges: Vec::new(),
        }
    }

    pub fn collect(&mut self, files: &[(PathBuf, syn::File)]) {
        for (file, ast) in files {
            let context = Context {
                file: file.clone(),
                module: Vec::new(),
            };
            self.edges
                .extend(modules::module_edges(file, &ast.items, &self.known));
            self.index.collect_types(&ast.items, &context);
            self.index.collect_callables(&ast.items, &context, &[]);
        }
    }

    pub fn finish(mut self) -> WorkspaceIndex {
        let known = self.known.into_values().flatten().collect();
        self.index.establish_modules(known, self.edges);
        self.index.rebase_declarations();
        self.index.index_declarations();
        self.index.finish_callables();
        self.index
    }
}

impl WorkspaceIndex {
    fn rebase_declarations(&mut self) {
        let bases: HashMap<_, _> = self
            .contexts
            .iter()
            .filter_map(|(file, contexts)| {
                contexts
                    .first()
                    .map(|context| (file.clone(), context.module.clone()))
            })
            .collect();
        for declaration in &mut self.declarations {
            rebase_context(&mut declaration.context, &bases);
            declaration.id.module = declaration.context.module.clone();
        }
        for value in &mut self.values {
            rebase_context(&mut value.context, &bases);
        }
        for import in &mut self.imports {
            rebase_context(&mut import.context, &bases);
        }
        for callable in &mut self.callables {
            rebase_context(&mut callable.context, &bases);
        }
    }

    fn finish_callables(&mut self) {
        // Free names must be available during namespace-aware owner expansion.
        self.callables.sort_by(|left, right| left.id.cmp(&right.id));
        self.index_callable_names();
        let resolved: Vec<_> = self
            .callables
            .iter()
            .map(|call| self.resolve_callable(call))
            .collect();
        self.callables = resolved;
    }

    fn resolve_callable(&self, original: &Callable) -> Callable {
        let mut call = original.clone();
        if let Some(ty) = &call.owner_syntax {
            let owner = self.type_from_owned(ty, &call.context, &call.substitutions);
            call.requirements_known &= matches!(owner, TypeFact::Nominal { .. });
            call.substitutions.insert("Self".into(), owner.clone());
            call.owner = Some(owner);
        }
        call.trait_type = call
            .trait_syntax
            .as_ref()
            .map(|ty| self.type_from_owned(ty, &call.context, &call.substitutions));
        call
    }
}

fn rebase_context(context: &mut Context, bases: &HashMap<PathBuf, Vec<String>>) {
    if let Some(prefix) = bases.get(&context.file) {
        context.module = qualified_path(prefix, &context.module);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_metadata_contains_no_thread_local_ast_or_span() {
        fn owned<T: Send + Sync>() {}
        owned::<WorkspaceIndex>();
        owned::<DeclarationCollector>();
    }
}
