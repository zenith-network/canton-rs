//! Utils to generate valid Rust identifiers
//!
//! TODO: it may be useful to implement a generator with cache here

use heck::{ToSnakeCase as _, ToUpperCamelCase as _};
use proc_macro2::Span;
use syn::{Ident, parse_str};
use unicode_ident::{is_xid_continue, is_xid_start};
use unicode_normalization::UnicodeNormalization as _;

/// Generate a valid identifier converting input to snake_case (field names, module names etc.)
pub fn generate_snake_ident(input: impl AsRef<str>) -> Ident {
    generate_ident(input.as_ref().to_snake_case())
}

/// Generate a valid identifier converting the input to UpperCamelCase (struct names, enum names,
/// etc.)
pub fn generate_camel_ident(input: impl AsRef<str>) -> Ident {
    generate_ident(input.as_ref().to_upper_camel_case())
}

/// Generate a valid identifier from input
///
/// Most of the time you want to use [`generate_snake_ident`] or [`generate_camel_ident`].
pub fn generate_ident(input: impl AsRef<str>) -> Ident {
    // Happy-path first
    if let Ok(ident) = parse_str(input.as_ref()) {
        return ident;
    }

    // This is heavier than happy path
    sanitize_ident(input)
}

/// Sanitizes the input before creating the identifier
///
/// Don't use this directly, use [`generate_ident`] instead.
pub fn sanitize_ident(input: impl AsRef<str>) -> Ident {
    // First check if we're hitting keywords, before doing heavy operations
    if kw::is_strict(&input) || kw::is_reserved(&input) {
        if kw::is_reserved_raw(&input) {
            return Ident::new(&format!("{}_", input.as_ref()), Span::call_site());
        } else {
            return Ident::new_raw(input.as_ref(), Span::call_site());
        }
    }

    // Here the input is assumed to violate some rules from Unicode Standard Annex
    let mut input: String = input.as_ref().nfc().collect();

    // We're not gonna add more than 2 ASCII symbols
    const MAX_ADDED: usize = 2;
    let mut output = String::with_capacity(input.len() + MAX_ADDED);

    if input.is_empty() {
        input = "__".to_string();
    }

    let mut chars = input.chars();

    // Safety: we checked that input is not empty above, so it's safe to unwrap here
    let first = chars.next().unwrap();
    if !is_xid_start(first) && first != '_' {
        output.push('_');
        if is_xid_continue(first) {
            output.push(first);
        } else {
            output.push(first);
        }
    } else {
        output.push(first);
    }

    for c in chars {
        if is_xid_continue(c) {
            output.push(c)
        } else {
            // We replace all "bad" symbols with '_'
            output.push('_');
        }
    }

    Ident::new(&output, Span::call_site())
}

/// Some helpers to work with Rust keywords
pub mod kw {
    /// Returns `true` if input is a strict keyword
    ///
    /// Reference: https://doc.rust-lang.org/reference/keywords.html#strict-keywords
    pub fn is_strict(input: impl AsRef<str>) -> bool {
        match input.as_ref() {
            "_" | "as" | "async" | "await" | "break" | "const" | "continue" | "crate" | "dyn"
            | "else" | "enum" | "extern" | "false" | "fn" | "for" | "if" | "impl" | "in"
            | "let" | "loop" | "match" | "mod" | "move" | "mut" | "pub" | "ref" | "return"
            | "self" | "Self" | "static" | "struct" | "super" | "trait" | "true" | "type"
            | "unsafe" | "use" | "where" | "while" => true,
            _ => false,
        }
    }

    /// Returns `true` if input is a reserved keyword
    ///
    /// Reference: https://doc.rust-lang.org/reference/keywords.html#reserved-keywords
    pub fn is_reserved(input: impl AsRef<str>) -> bool {
        match input.as_ref() {
            "abstract" | "become" | "box" | "do" | "final" | "gen" | "macro" | "override"
            | "priv" | "try" | "typeof" | "unsized" | "virtual" | "yield" => true,
            _ => false,
        }
    }

    /// Returns `true` is input is `RESERVED_RAW_IDENTIFIER` (subset of strict keywords):
    /// `( _ | crate | self | Self | super )`
    ///
    /// Reference: https://doc.rust-lang.org/reference/identifiers.html#raw-identifiers
    pub fn is_reserved_raw(input: impl AsRef<str>) -> bool {
        match input.as_ref() {
            "_" | "crate" | "self" | "Self" | "super" => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO: test this module
}
