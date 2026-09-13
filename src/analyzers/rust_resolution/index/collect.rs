//! Declaration collection without body analysis.

use super::*;

impl WorkspaceIndex {
    pub(super) fn collect_types(&mut self, items: &[syn::Item], context: &Context) {
        for item in items {
            match item {
                syn::Item::Struct(item) => self.add_type(
                    context,
                    &item.ident,
                    &item.generics,
                    &item.fields,
                    None,
                    DeclarationKind::Struct,
                ),
                syn::Item::Enum(item) => self.add_type(
                    context,
                    &item.ident,
                    &item.generics,
                    &syn::Fields::Unit,
                    None,
                    DeclarationKind::Enum,
                ),
                syn::Item::Union(item) => self.add_type(
                    context,
                    &item.ident,
                    &item.generics,
                    &syn::Fields::Named(item.fields.clone()),
                    None,
                    DeclarationKind::Union,
                ),
                syn::Item::Type(item) => self.add_type(
                    context,
                    &item.ident,
                    &item.generics,
                    &syn::Fields::Unit,
                    Some(*item.ty.clone()),
                    DeclarationKind::Alias,
                ),
                syn::Item::Trait(item) => self.add_type(
                    context,
                    &item.ident,
                    &item.generics,
                    &syn::Fields::Unit,
                    None,
                    DeclarationKind::Trait,
                ),
                syn::Item::Mod(item) => {
                    if let Some((_, items)) = &item.content {
                        self.collect_types(items, &child_context(context, &item.ident.to_string()));
                    }
                }
                syn::Item::Use(item) => {
                    let prefix = if item.leading_colon.is_some() {
                        vec!["::".to_string()]
                    } else {
                        Vec::new()
                    };
                    self.collect_imports(&item.tree, context, &prefix);
                }
                syn::Item::Const(item) => self.values.push(ValueDeclaration {
                    context: context.clone(),
                    name: item.ident.to_string(),
                    ty: TypeSyntax::from_syn(&item.ty),
                    line: item.ident.span().start().line,
                    column: item.ident.span().start().column,
                }),
                syn::Item::Static(item) => self.values.push(ValueDeclaration {
                    context: context.clone(),
                    name: item.ident.to_string(),
                    ty: TypeSyntax::from_syn(&item.ty),
                    line: item.ident.span().start().line,
                    column: item.ident.span().start().column,
                }),
                _ => {}
            }
        }
    }

    pub(super) fn add_type(
        &mut self,
        context: &Context,
        name: &syn::Ident,
        generics: &syn::Generics,
        fields: &syn::Fields,
        alias: Option<syn::Type>,
        kind: DeclarationKind,
    ) {
        let id = DeclarationId {
            file: context.file.clone(),
            module: context.module.clone(),
            name: name.to_string(),
            line: name.span().start().line,
            column: name.span().start().column,
        };
        let field_types = fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                (
                    field
                        .ident
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| index.to_string()),
                    TypeSyntax::from_syn(&field.ty),
                )
            })
            .collect();
        self.declarations.push(TypeDeclaration {
            id,
            context: context.clone(),
            generics: generic_names(generics),
            fields: field_types,
            alias: alias.as_ref().map(TypeSyntax::from_syn),
            is_trait: kind == DeclarationKind::Trait,
            unit: matches!(fields, syn::Fields::Unit) && kind == DeclarationKind::Struct,
        });
    }

    pub(super) fn collect_imports(
        &mut self,
        tree: &syn::UseTree,
        context: &Context,
        prefix: &[String],
    ) {
        match tree {
            syn::UseTree::Path(path) => self.collect_imports(
                &path.tree,
                context,
                &qualified(prefix, &path.ident.to_string()),
            ),
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.collect_imports(item, context, prefix);
                }
            }
            syn::UseTree::Name(name) => {
                let path = if name.ident == "self" {
                    prefix.to_vec()
                } else {
                    qualified(prefix, &name.ident.to_string())
                };
                let alias = path.last().cloned().unwrap_or_default();
                self.imports.push(Import {
                    context: context.clone(),
                    alias,
                    path,
                    glob: false,
                });
            }
            syn::UseTree::Rename(rename) => {
                let path = if rename.ident == "self" {
                    prefix.to_vec()
                } else {
                    qualified(prefix, &rename.ident.to_string())
                };
                self.imports.push(Import {
                    context: context.clone(),
                    alias: rename.rename.to_string(),
                    path,
                    glob: false,
                });
            }
            syn::UseTree::Glob(_) => self.imports.push(Import {
                context: context.clone(),
                alias: String::new(),
                path: prefix.to_vec(),
                glob: true,
            }),
        }
    }

    pub(super) fn collect_callables(
        &mut self,
        items: &[syn::Item],
        context: &Context,
        inline: &[String],
    ) {
        for item in items {
            match item {
                syn::Item::Fn(item) => {
                    let call = make_callable(context, inline, &item.sig, &item.attrs, true);
                    self.callables.push(call);
                }
                syn::Item::Impl(item) => self.collect_impl(item, context, inline),
                syn::Item::Trait(item) => self.collect_trait(item, context, inline),
                syn::Item::Mod(item) => {
                    if let Some((_, items)) = &item.content {
                        self.collect_callables(
                            items,
                            &child_context(context, &item.ident.to_string()),
                            &qualified(inline, &item.ident.to_string()),
                        );
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_impl(
        &mut self,
        item: &syn::ItemImpl,
        context: &Context,
        inline: &[String],
    ) {
        let substitutions = generic_substitutions(&item.generics, context);
        let owner_syntax = TypeSyntax::from_syn(&item.self_ty);
        let owner_name = match item.self_ty.as_ref() {
            syn::Type::Path(path) => path_segments(&path.path).join("::"),
            _ => item.self_ty.to_token_stream().to_string(),
        };
        for member in &item.items {
            let syn::ImplItem::Fn(method) = member else {
                continue;
            };
            let mut call = make_callable(
                context,
                &qualified(inline, &owner_name),
                &method.sig,
                &method.attrs,
                true,
            );
            call.owner_syntax = Some(owner_syntax.clone());
            call.trait_path = item
                .trait_
                .as_ref()
                .map(|(_, path, _)| resolution_segments(path));
            call.trait_syntax = item
                .trait_
                .as_ref()
                .map(|(_, path, _)| TypeSyntax::from_path(path));
            call.kind = if call.trait_path.is_some() {
                CallableKind::TraitMethod
            } else if call.has_receiver() {
                CallableKind::InherentMethod
            } else {
                CallableKind::AssociatedFunction
            };
            call.requirements_known = requirements_known(&item.generics)
                && requirements_known(&method.sig.generics)
                && item.trait_.as_ref().is_none_or(|(negative, path, _)| {
                    negative.is_none() && !has_trait_type_arguments(path)
                });
            call.substitutions.extend(substitutions.clone());
            call.const_parameters.extend(
                item.generics
                    .const_params()
                    .map(|parameter| parameter.ident.to_string()),
            );
            self.callables.push(call);
        }
    }

    pub(super) fn collect_trait(
        &mut self,
        item: &syn::ItemTrait,
        context: &Context,
        inline: &[String],
    ) {
        for member in &item.items {
            let syn::TraitItem::Fn(method) = member else {
                continue;
            };
            let mut call = make_callable(
                context,
                &qualified(inline, &item.ident.to_string()),
                &method.sig,
                &method.attrs,
                method.default.is_some(),
            );
            call.kind = CallableKind::TraitDeclaration;
            call.owner = Some(TypeFact::SelfType);
            call.trait_path = Some(vec![item.ident.to_string()]);
            call.requirements_known = false;
            call.substitutions
                .extend(generic_substitutions(&item.generics, context));
            call.const_parameters.extend(
                item.generics
                    .const_params()
                    .map(|parameter| parameter.ident.to_string()),
            );
            call.substitutions
                .insert("Self".to_string(), TypeFact::SelfType);
            self.callables.push(call);
        }
    }
}
