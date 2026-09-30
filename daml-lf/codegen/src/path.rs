//! Utilities to work with Rust paths (e.g. `foo::bar::baz`)
//!
//! See [reference].
//!
//! [reference]: https://doc.rust-lang.org/reference/paths.html

use heck::ToSnakeCase;
use syn::{Path, PathSegment};

use crate::ident;

/// Generate a module path
///
/// All segments of the path will be converted to snake_case
///
/// # Example
///
/// `["MyMod", "MySubmod"]` -> `my_mod::my_submod`
pub fn generate_module_path(path: impl IntoIterator<Item = impl AsRef<str>>) -> Path {
    generate_path(path.into_iter().map(|s| s.as_ref().to_snake_case()))
}

/// Generate a path (without leading colon)
///
/// All segments are left unmodified (not converted to snake_case or CamelCase)
pub fn generate_path(path: impl IntoIterator<Item = impl AsRef<str>>) -> Path {
    Path {
        leading_colon: None,
        segments: path
            .into_iter()
            .map(|segment| PathSegment::from(ident::generate_ident(segment)))
            .collect(),
    }
}

/// Length of the max common prefix of two paths
pub fn common_prefix_len(path1: &Path, path2: &Path) -> usize {
    path1
        .segments
        .iter()
        .zip(&path2.segments)
        .take_while(|(segment1, segment2)| segment1 == segment2)
        .count()
}

/// Find the shortest relative path from `main` module to `target` module
///
/// Both paths are expected to contain only regular identifier segments and no leading colon. The
/// result goes up to the common ancestor using `super` and then follows the remaining segments of
/// `target`. If both paths are equal, the result is an empty path.
///
/// Note that target path may be empty - then the function will return a number of `super`-s equal
/// to the length of main path.
///
/// If main path is empty, then function returns target path.
///
/// ## Example
///
/// If `main = a::b::c::d`, `target = a::b::e::f`, then the output is `super::super::e::f`
pub fn find_shortest_path(main: &Path, target: &Path) -> Path {
    let common_prefix_len = common_prefix_len(main, target);
    let mut path = super_repeat(main.segments.len() - common_prefix_len);
    path.segments
        .extend(target.segments.iter().skip(common_prefix_len).cloned());
    path
}

/// Returns path like `super::super::...`, where `super` is repeated `num` times
///
/// If `num` is `0`, returns empty path.
pub fn super_repeat(num: usize) -> Path {
    Path {
        leading_colon: None,
        segments: std::iter::repeat_with(|| PathSegment {
            ident: syn::token::Super::default().into(),
            arguments: syn::PathArguments::None,
        })
        .take(num)
        .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Path, common_prefix_len, find_shortest_path, super_repeat};

    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use syn::punctuated::Punctuated;

    fn parse_path(path: &str) -> Path {
        if path.is_empty() {
            Path {
                leading_colon: None,
                segments: Punctuated::new(),
            }
        } else {
            syn::parse_str(path).unwrap()
        }
    }

    #[rstest]
    #[case::both_empty("", "", 0)]
    #[case::left_empty("", "a::b", 0)]
    #[case::right_empty("a::b", "", 0)]
    #[case::identical("a::b::c", "a::b::c", 3)]
    #[case::disjoint("a::b", "c::d", 0)]
    #[case::partially_shared("a::b::c::d", "a::b::e::f", 2)]
    fn test_common_prefix_len(#[case] path1: &str, #[case] path2: &str, #[case] expected: usize) {
        assert_eq!(
            common_prefix_len(&parse_path(path1), &parse_path(path2)),
            expected
        );
    }

    #[rstest]
    #[case::both_empty("", "", "")]
    #[case::empty_main("", "a::b", "a::b")]
    #[case::empty_target("a::b::c", "", "super::super::super")]
    #[case::identical("a::b::c", "a::b::c", "")]
    #[case::target_descendant("a::b", "a::b::c::d", "c::d")]
    #[case::target_ancestor("a::b::c::d", "a::b", "super::super")]
    #[case::sibling_branches("a::b::c::d", "a::b::e::f", "super::super::e::f")]
    #[case::no_common_prefix("a::b", "c::d", "super::super::c::d")]
    fn test_find_shortest_path(#[case] main: &str, #[case] target: &str, #[case] expected: &str) {
        assert_eq!(
            find_shortest_path(&parse_path(main), &parse_path(target)),
            parse_path(expected)
        );
    }

    #[rstest]
    #[case::empty(0, "")]
    #[case::one(1, "super")]
    #[case::two(2, "super::super")]
    #[case::many(5, "super::super::super::super::super")]
    fn test_super_repeat(#[case] num: usize, #[case] expected: &str) {
        assert_eq!(super_repeat(num), parse_path(expected));
    }
}
