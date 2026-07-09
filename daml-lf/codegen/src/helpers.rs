//! Helpers to work with syn

use syn::{Ident, Item, ItemMod, Visibility};

/// Push module content by path
///
/// If any module alongside the path doesn't exist, it will be created
///
/// Error is returned if any of the segments of the path is not a valid identifier
///
/// Notes:
///
/// 1. A module with given identifier exists not more than once in the tree.
///     E.g things like:
///     ```rust,ignore
///     #[cfg(unix)]
///     mod A {};
///     #[cfg(windows)]
///     mod A {};
///     ```
///     are not handled properly.
/// 2. Name collisions with other items are not handled gracefully.
///     E.g. if there is a `mod Foo {}` or even `struct Foo`, this function will not report an error
///     if creating `mod Foo {}`.
/// 3. File-based modules are not considered (`mod my_mod;`)
pub fn push_module(mut root: &mut ItemMod, parent_path: &[&str], module: ItemMod) {
    for module_name in parent_path {
        let ident = crate::ident::generate_snake_ident(module_name);
        let ident_copy = ident.clone();

        root = find_or_insert_submod_with(
            root,
            |module| module.ident == ident_copy,
            || empty_mod(ident),
        );
    }

    // At this point we need to merge/insert into the root
    let items = &mut root.content.get_or_insert_default().1;

    if let Some(mod_) = items.iter_mut().find_map(|item| match item {
        Item::Mod(item_mod) if item_mod.ident == module.ident => Some(item_mod),
        _ => None,
    }) {
        // There is already a module with this name in the root, so merge content
        let (_, mod_items) = mod_.content.get_or_insert_default();
        if let Some((_, new_items)) = module.content {
            mod_items.extend(new_items);
        }
    } else {
        // Simply insert a new module
        items.push(Item::Mod(module));
    }
}

/// Get reference to a submodule of `mod_` which matches predicate or create and insert one with `f`
pub fn find_or_insert_submod_with<P, F>(mod_: &mut ItemMod, mut predicate: P, f: F) -> &mut ItemMod
where
    P: FnMut(&ItemMod) -> bool,
    F: FnOnce() -> ItemMod,
{
    let (_, items) = mod_.content.get_or_insert_default();

    let position = items.iter().position(|item| {
        if let Item::Mod(submod) = item {
            predicate(submod)
        } else {
            false
        }
    });

    let item = if let Some(pos) = position {
        &mut items[pos]
    } else {
        items.push_mut(Item::Mod(f()))
    };

    let Item::Mod(submod) = item else {
        // position was found or inserted as Mod variant
        unreachable!()
    };

    submod
}

/// `pub mod Name {}`
pub fn empty_mod(ident: Ident) -> ItemMod {
    mod_with_items(ident, Vec::new())
}

/// `pub mod Name { ... }`
pub fn mod_with_items(ident: Ident, items: Vec<Item>) -> ItemMod {
    ItemMod {
        attrs: Vec::new(),
        vis: Visibility::Public(Default::default()),
        unsafety: None,
        mod_token: Default::default(),
        ident,
        content: Some((Default::default(), items)),
        semi: None,
    }
}

pub fn is_empty_mod(module: &ItemMod) -> bool {
    module
        .content
        .as_ref()
        .is_none_or(|(_, items)| items.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;
    use quote::quote;
    use rstest::rstest;

    #[rstest]
    #[case(
        "pub mod root { }",
        "alpha.beta",
        "pub mod gamma { pub struct Added ; }",
        "pub mod root { pub mod alpha { pub mod beta { pub mod gamma { pub struct Added ; } } } }"
    )]
    #[case(
        "pub mod root { pub mod alpha { pub mod beta { } } }",
        "alpha",
        "pub mod alpha { pub struct Added ; pub enum Choice { Left , Right } }",
        "pub mod root { pub const EXISTING : u8 = 1 ; pub mod alpha { pub struct Added ; pub enum Choice { Left , Right } } }"
    )]
    fn test_push_module(
        #[case] src: &str,
        #[case] path: &str,
        #[case] content: &str,
        #[case] expected: &str,
    ) {
        let mut src = syn::parse_str::<ItemMod>(src).unwrap();

        let path = path
            .split('.')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();

        let content = syn::parse_str(content).unwrap();

        push_module(&mut src, &path, content);

        let result = quote! { #src }.to_string();
        assert_eq!(result, expected);
    }
}
