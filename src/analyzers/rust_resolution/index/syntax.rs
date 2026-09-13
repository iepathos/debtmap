//! Owned type syntax: declaration metadata never retains proc-macro spans.
use super::*;

#[derive(Clone, Debug)]
pub enum TypeSyntax {
    Path {
        segments: Vec<String>,
        arguments: Vec<TypeSyntax>,
    },
    Reference {
        mutable: bool,
        inner: Box<TypeSyntax>,
    },
    Tuple(Vec<TypeSyntax>),
    Dynamic(Vec<Vec<String>>),
    Const(String),
    Unsupported,
}

impl TypeSyntax {
    pub fn from_syn(ty: &syn::Type) -> Self {
        match ty {
            syn::Type::Path(path) if path.qself.is_none() => Self::from_path(&path.path),
            syn::Type::Reference(reference) => Self::Reference {
                mutable: reference.mutability.is_some(),
                inner: Box::new(Self::from_syn(&reference.elem)),
            },
            syn::Type::Paren(paren) => Self::from_syn(&paren.elem),
            syn::Type::Group(group) => Self::from_syn(&group.elem),
            syn::Type::Tuple(tuple) => {
                Self::Tuple(tuple.elems.iter().map(Self::from_syn).collect())
            }
            syn::Type::TraitObject(object) => {
                Self::Dynamic(object.bounds.iter().filter_map(trait_bound_path).collect())
            }
            _ => Self::Unsupported,
        }
    }

    pub fn from_path(path: &syn::Path) -> Self {
        Self::Path {
            segments: resolution_segments(path),
            arguments: path
                .segments
                .last()
                .map(|segment| Self::arguments(&segment.arguments))
                .unwrap_or_default(),
        }
    }

    pub fn arguments(args: &syn::PathArguments) -> Vec<Self> {
        let syn::PathArguments::AngleBracketed(args) = args else {
            return Vec::new();
        };
        args.args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(ty) => Some(Self::from_syn(ty)),
                syn::GenericArgument::Const(expr) => {
                    Some(Self::Const(expr.to_token_stream().to_string()))
                }
                syn::GenericArgument::Lifetime(_) => None,
                _ => Some(Self::Unsupported),
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct ReceiverSyntax {
    pub explicit: bool,
    pub reference: bool,
    pub mutable: bool,
}

#[derive(Clone, Debug)]
pub struct SignatureSyntax {
    pub ident: String,
    pub generics: Vec<String>,
    pub output: Option<TypeSyntax>,
    pub asyncness: bool,
    pub receiver: Option<ReceiverSyntax>,
}

impl SignatureSyntax {
    pub fn from_syn(signature: &syn::Signature) -> Self {
        Self {
            ident: signature.ident.to_string(),
            generics: generic_names(&signature.generics),
            output: match &signature.output {
                syn::ReturnType::Default => None,
                syn::ReturnType::Type(_, ty) => Some(TypeSyntax::from_syn(ty)),
            },
            asyncness: signature.asyncness.is_some(),
            receiver: signature.receiver().map(|receiver| ReceiverSyntax {
                explicit: receiver.colon_token.is_some(),
                reference: receiver.reference.is_some(),
                mutable: receiver.mutability.is_some(),
            }),
        }
    }

    pub fn receiver(&self) -> Option<&ReceiverSyntax> {
        self.receiver.as_ref()
    }
}
