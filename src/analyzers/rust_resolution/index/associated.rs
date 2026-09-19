//! Associated calls consume facts that have already been lowered in lexical scope.
use super::*;

pub(in crate::analyzers::rust_resolution) struct AssociatedQuery {
    pub owner: TypeFact,
    pub trait_type: Option<TypeFact>,
    pub member: String,
    pub arguments: Vec<TypeFact>,
}

impl WorkspaceIndex {
    pub(in crate::analyzers::rust_resolution) fn lookup_associated_query(
        &self,
        query: &AssociatedQuery,
        context: &Context,
    ) -> Lookup<'_> {
        match &query.trait_type {
            Some(trait_type) => {
                self.lookup_trait_associated(&query.owner, trait_type, &query.member)
            }
            None => self.lookup_associated(&query.owner, &query.member, context),
        }
    }
}
