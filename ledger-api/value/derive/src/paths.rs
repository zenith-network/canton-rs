use syn::{Path, parse_quote};

/// Paths to Canton types/traits which are used in the generated code
#[derive(Clone)]
pub struct Paths {
    /// Main crate path
    ///
    /// Default: `::canton`
    pub root: Path,

    /// Path to `types` module in Canton crate
    ///
    /// Default: `::canton::types`
    pub types: Path,

    /// Path to `ledger_api::types` module in Canton crate
    ///
    /// Default: `::canton::ledger_api::types`
    pub ledger_api_types: Path,

    /// Path to `value` module in Canton crate
    ///
    /// Default: `::canton::ledger_api::types::value`
    pub value: Path,

    /// Path to `value::v2` module in Canton crate
    ///
    /// Default: `::canton::ledger_api::types::value::v2`
    pub value_v2: Path,

    /// Path to `IntoValue` trait
    ///
    /// Default: `::canton::ledger_api::types::value::v2::IntoValue`
    pub into_value_trait: Path,

    /// Path to `TryFromValue` trait
    ///
    /// Default: `::canton::ledger_api::types::value::v2::TryFromValue`
    pub try_from_value_trait: Path,

    /// Path to `Value` trait
    ///
    /// Default: `::canton::ledger_api::types::value::v2::Value`
    pub value_trait: Path,

    /// Path to `IntoRecord` trait
    ///
    /// Default: `::canton::ledger_api::types::value::v2::IntoRecord`
    pub into_record_trait: Path,

    /// Path to `TryFromRecord` trait
    ///
    /// Default: `::canton::ledger_api::types::value::v2::TryFromRecord`
    pub try_from_record_trait: Path,

    /// Path to `Record` trait
    ///
    /// Default: `::canton::ledger_api::types::value::v2::Record`
    pub record_trait: Path,
}

impl Paths {
    /// Default main crate path: `::canton`
    pub fn default_path() -> Path {
        parse_quote! { ::canton }
    }

    /// Build all paths from main crate path
    pub fn from_root(root: Path) -> Self {
        let mut types = root.clone();
        types.segments.push(parse_quote! { types });

        let mut ledger_api_types = root.clone();
        ledger_api_types.segments.push(parse_quote! { ledger_api });
        ledger_api_types.segments.push(parse_quote! { types });

        let mut value = ledger_api_types.clone();
        value.segments.push(parse_quote! { value });

        let mut value_v2 = value.clone();
        value_v2.segments.push(parse_quote! { v2 });

        let mut into_value_trait = value_v2.clone();
        into_value_trait.segments.push(parse_quote! { IntoValue });

        let mut try_from_value_trait = value_v2.clone();
        try_from_value_trait
            .segments
            .push(parse_quote! { TryFromValue });

        let mut value_trait = value_v2.clone();
        value_trait.segments.push(parse_quote! { Value });

        let mut into_record_trait = value_v2.clone();
        into_record_trait.segments.push(parse_quote! { IntoRecord });

        let mut try_from_record_trait = value_v2.clone();
        try_from_record_trait
            .segments
            .push(parse_quote! { TryFromRecord });

        let mut record_trait = value_v2.clone();
        record_trait.segments.push(parse_quote! { Record });

        Self {
            root,
            types,
            ledger_api_types,
            value,
            value_v2,
            into_value_trait,
            try_from_value_trait,
            value_trait,
            into_record_trait,
            try_from_record_trait,
            record_trait,
        }
    }
}

impl Default for Paths {
    /// Initialize paths with default values (with `::canton` as root)
    fn default() -> Self {
        let default_path = Self::default_path();
        Self::from_root(default_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;
    use quote::quote;

    #[test]
    fn test_default_paths() {
        let Paths {
            root,
            types,
            ledger_api_types,
            value,
            value_v2,
            into_value_trait,
            try_from_value_trait,
            value_trait,
            into_record_trait,
            try_from_record_trait,
            record_trait,
        } = Paths::default();

        let example_code = quote! {
            use #root;
            use #types;
            use #ledger_api_types;
            use #value;
            use #value_v2;
            use #into_value_trait;
            use #try_from_value_trait;
            use #value_trait;
            use #into_record_trait;
            use #try_from_record_trait;
            use #record_trait;
        };

        let result = prettyplease::unparse(&syn::parse2(example_code).unwrap());

        let expected = r"use ::canton;
use ::canton::types;
use ::canton::ledger_api::types;
use ::canton::ledger_api::types::value;
use ::canton::ledger_api::types::value::v2;
use ::canton::ledger_api::types::value::v2::IntoValue;
use ::canton::ledger_api::types::value::v2::TryFromValue;
use ::canton::ledger_api::types::value::v2::Value;
use ::canton::ledger_api::types::value::v2::IntoRecord;
use ::canton::ledger_api::types::value::v2::TryFromRecord;
use ::canton::ledger_api::types::value::v2::Record;
";

        assert_eq!(result, expected);
    }
}
