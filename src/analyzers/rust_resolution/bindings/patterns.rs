//! Propagate tuple and reference patterns; unsupported bindings still shadow outer names.
use super::super::body::{Body, unknown};
use super::super::types::TypeFact;
use super::annotations::{annotated_fact, tuple_facts};
use syn::{Pat, visit::Visit};

impl Body<'_> {
    pub(in crate::analyzers::rust_resolution) fn bind_pattern(
        &mut self,
        pat: &Pat,
        fact: TypeFact,
        declared: bool,
    ) {
        match pat {
            Pat::Ident(p) => {
                let fact = if p.by_ref.is_some() {
                    TypeFact::Reference {
                        mutable: p.mutability.is_some(),
                        inner: Box::new(fact),
                    }
                } else {
                    fact
                };
                self.bindings.insert(p.ident.to_string(), fact, declared);
                if let Some((_, pat)) = &p.subpat {
                    self.bind_pattern(pat, unknown(), false);
                }
            }
            Pat::Type(p) => {
                let annotation = self.declared_type(&p.ty);
                let fact = annotated_fact(annotation, &fact, 0);
                self.bind_pattern(&p.pat, fact, true);
            }
            Pat::Paren(p) => self.bind_pattern(&p.pat, fact, declared),
            Pat::Tuple(p) => self.bind_tuple(&p.elems, fact, declared),
            _ => {
                let mut names = PatternNames::default();
                names.visit_pat(pat);
                for name in names.0 {
                    self.bindings.insert(name, unknown(), false);
                }
            }
        }
    }
    fn bind_tuple(
        &mut self,
        pats: &syn::punctuated::Punctuated<Pat, syn::Token![,]>,
        fact: TypeFact,
        declared: bool,
    ) {
        let facts = tuple_facts(fact);
        let rest = pats.iter().position(|pat| matches!(pat, Pat::Rest(_)));
        for (position, pat) in pats.iter().enumerate() {
            let index = if rest.is_some_and(|rest| position > rest) {
                facts.len().checked_sub(pats.len() - position)
            } else {
                Some(position)
            };
            let fact = index
                .and_then(|index| facts.get(index))
                .cloned()
                .unwrap_or_else(unknown);
            self.bind_pattern(pat, fact, declared);
        }
    }
}

#[derive(Default)]
struct PatternNames(Vec<String>);
impl<'ast> Visit<'ast> for PatternNames {
    fn visit_pat_ident(&mut self, pat: &'ast syn::PatIdent) {
        self.0.push(pat.ident.to_string());
        syn::visit::visit_pat_ident(self, pat);
    }
}
