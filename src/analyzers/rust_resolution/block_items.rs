//! Namespace-specific block-wide item shadows. Block declarations are not indexed bodies.

pub(super) struct ItemShadow {
    pub name: String,
    pub types: bool,
    pub values: bool,
    pub import: Option<Vec<String>>,
}

impl ItemShadow {
    fn local(name: &syn::Ident, types: bool, values: bool) -> Self {
        Self {
            name: name.to_string(),
            types,
            values,
            import: None,
        }
    }
}

pub(super) fn block_items(block: &syn::Block) -> Vec<ItemShadow> {
    block
        .stmts
        .iter()
        .flat_map(|stmt| match stmt {
            syn::Stmt::Item(item) => item_names(item),
            _ => Vec::new(),
        })
        .collect()
}

fn item_names(item: &syn::Item) -> Vec<ItemShadow> {
    let shadow = match item {
        syn::Item::Fn(item) => ItemShadow::local(&item.sig.ident, false, true),
        syn::Item::Const(item) => ItemShadow::local(&item.ident, false, true),
        syn::Item::Static(item) => ItemShadow::local(&item.ident, false, true),
        syn::Item::Struct(item) => ItemShadow::local(
            &item.ident,
            true,
            !matches!(item.fields, syn::Fields::Named(_)),
        ),
        syn::Item::Enum(item) => ItemShadow::local(&item.ident, true, false),
        syn::Item::Union(item) => ItemShadow::local(&item.ident, true, false),
        syn::Item::Type(item) => ItemShadow::local(&item.ident, true, false),
        syn::Item::Mod(item) => ItemShadow::local(&item.ident, true, false),
        syn::Item::Trait(item) => ItemShadow::local(&item.ident, true, false),
        syn::Item::TraitAlias(item) => ItemShadow::local(&item.ident, true, false),
        syn::Item::ExternCrate(item) => ItemShadow::local(
            item.rename
                .as_ref()
                .map(|(_, name)| name)
                .unwrap_or(&item.ident),
            true,
            false,
        ),
        syn::Item::Use(item) => {
            return use_names(
                &item.tree,
                if item.leading_colon.is_some() {
                    vec!["::".into()]
                } else {
                    Vec::new()
                },
            );
        }
        _ => return Vec::new(),
    };
    vec![shadow]
}

fn use_names(tree: &syn::UseTree, prefix: Vec<String>) -> Vec<ItemShadow> {
    match tree {
        syn::UseTree::Path(path) => use_names(
            &path.tree,
            prefix.into_iter().chain([path.ident.to_string()]).collect(),
        ),
        syn::UseTree::Name(name) => {
            let path = if name.ident == "self" {
                prefix
            } else {
                prefix.into_iter().chain([name.ident.to_string()]).collect()
            };
            vec![ItemShadow {
                name: path.last().cloned().unwrap_or_default(),
                types: true,
                values: true,
                import: Some(path),
            }]
        }
        syn::UseTree::Rename(rename) => {
            let path = if rename.ident == "self" {
                prefix
            } else {
                prefix
                    .into_iter()
                    .chain([rename.ident.to_string()])
                    .collect()
            };
            vec![ItemShadow {
                name: rename.rename.to_string(),
                types: true,
                values: true,
                import: Some(path),
            }]
        }
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .flat_map(|item| use_names(item, prefix.clone()))
            .collect(),
        syn::UseTree::Glob(_) => vec![ItemShadow {
            name: "*".into(),
            types: true,
            values: true,
            import: None,
        }],
    }
}
