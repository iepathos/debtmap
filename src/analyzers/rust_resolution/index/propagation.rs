//! Return and field propagation through declared owned type syntax.
use super::*;

impl WorkspaceIndex {
    pub fn return_type(
        &self,
        call: &Callable,
        receiver: Option<&TypeFact>,
        arguments: &[TypeFact],
    ) -> TypeFact {
        let mut substitutions = call.substitutions.clone();
        if let Some(receiver) = receiver {
            let receiver = strip_references(receiver);
            substitutions.insert("Self".to_string(), receiver.clone());
            if let Some(owner) = &call.owner {
                infer_substitutions(owner, receiver, &mut substitutions);
            }
        }
        for (name, argument) in call.signature.generics.iter().zip(arguments) {
            substitutions.insert(name.clone(), argument.clone());
        }
        let output = match &call.signature.output {
            None => TypeFact::Tuple(Vec::new()),
            Some(ty) => self.type_from_owned(ty, &call.context, &substitutions),
        };
        let output = match receiver.and_then(TypeFact::uncertainty_reason) {
            Some(reason) => with_uncertainty(output, reason),
            None => output,
        };
        if call.signature.asyncness {
            TypeFact::Future(Box::new(output))
        } else {
            output
        }
    }

    pub fn field_type(&self, receiver: &TypeFact, member: &syn::Member) -> TypeFact {
        match receiver {
            TypeFact::Reference { inner, .. } => self.field_type(inner, member),
            TypeFact::Uncertain { constraint, reason } => {
                with_uncertainty(self.field_type(constraint, member), reason.clone())
            }
            TypeFact::Ambiguous(owners) => with_uncertainty(
                TypeFact::Ambiguous(
                    owners
                        .iter()
                        .map(|owner| self.field_type(owner, member))
                        .collect(),
                ),
                UnknownReason::AmbiguousDeclaration,
            ),
            _ => self.known_field_type(receiver, member),
        }
    }

    fn known_field_type(&self, receiver: &TypeFact, member: &syn::Member) -> TypeFact {
        let receiver = strip_references(receiver);
        if let (TypeFact::Tuple(fields), syn::Member::Unnamed(member)) = (receiver, member) {
            return fields
                .get(member.index as usize)
                .cloned()
                .unwrap_or_else(TypeFact::unknown);
        }
        let Some((id, arguments)) = receiver.nominal() else {
            return TypeFact::unknown();
        };
        let Some(declaration) = self.declaration(id) else {
            return TypeFact::unknown();
        };
        let key = match member {
            syn::Member::Named(name) => name.to_string(),
            syn::Member::Unnamed(index) => index.index.to_string(),
        };
        let Some(ty) = declaration.fields.get(&key) else {
            return TypeFact::Unknown(UnknownReason::UnavailableDefinition);
        };
        self.type_from_owned(
            ty,
            &declaration.context,
            &bind_generics(&declaration.generics, arguments),
        )
    }
}
