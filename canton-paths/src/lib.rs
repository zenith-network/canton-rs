//! This library defines [`Paths`] which is a helper for resolving paths on `canton` crate.
//!
//! Used in code generation type of workflows: DAR codegen, derive macros, etc.

use syn::{Path, parse_quote};

/// Paths to Canton types/traits which are used in the generated code
#[derive(Clone)]
#[cfg_attr(feature = "extra", derive(Debug, PartialEq, Eq, Hash))]
pub struct Paths {
    root: Path,
}

impl Paths {
    /// Default main crate path: `::canton`
    pub fn default_root() -> Path {
        parse_quote! { ::canton }
    }

    /// Build all paths from main crate path
    pub fn from_root(root: Path) -> Self {
        Self { root }
    }

    /// Main crate path
    ///
    /// Default: `::canton`
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path to `types` module in Canton crate
    ///
    /// Default: `::canton::types`
    pub fn types(&self) -> Path {
        let mut types = self.root.clone();
        types.segments.push(parse_quote! { types });
        types
    }

    /// Default: `::canton::types::ContractId`
    pub fn contract_id(&self) -> Path {
        let mut path = self.types();
        path.segments.push(parse_quote! { ContractId });
        path
    }

    /// Default: `::canton::types::DottedName`
    pub fn dotted_name(&self) -> Path {
        let mut path = self.types();
        path.segments.push(parse_quote! { DottedName });
        path
    }

    /// Default: `::canton::types::Name`
    pub fn name(&self) -> Path {
        let mut path = self.types();
        path.segments.push(parse_quote! { Name });
        path
    }

    /// Default: `::canton::types::PackageId`
    pub fn package_id(&self) -> Path {
        let mut path = self.types();
        path.segments.push(parse_quote! { PackageId });
        path
    }

    /// Default: `::canton::types::PackageName`
    pub fn package_name(&self) -> Path {
        let mut path = self.types();
        path.segments.push(parse_quote! { PackageName });
        path
    }

    /// Default: `::canton::types::LedgerString`
    pub fn ledger_string(&self) -> Path {
        let mut path = self.types();
        path.segments.push(parse_quote! { LedgerString });
        path
    }

    /// Default: `::canton::types::NonEmpty`
    pub fn non_empty(&self) -> Path {
        let mut path = self.types();
        path.segments.push(parse_quote! { NonEmpty });
        path
    }

    /// Path to `ledger_api::types` module in Canton crate
    ///
    /// Default: `::canton::ledger_api::types`
    pub fn ledger_api_types(&self) -> Path {
        let mut path = self.root.clone();
        path.segments.push(parse_quote! { ledger_api });
        path.segments.push(parse_quote! { types });
        path
    }

    /// Path to `ledger_api::types::v2` module in Canton crate
    ///
    /// Default: `::canton::ledger_api::types::v2`
    pub fn ledger_api_types_v2(&self) -> Path {
        let mut path = self.ledger_api_types();
        path.segments.push(parse_quote! { v2 });
        path
    }

    /// Path to `value` module in Canton crate
    ///
    /// Default: `::canton::ledger_api::types::value`
    pub fn value(&self) -> Path {
        let mut path = self.ledger_api_types();
        path.segments.push(parse_quote! { value });
        path
    }

    /// Path to `value::v2` module in Canton crate
    ///
    /// Default: `::canton::ledger_api::types::value::v2`
    pub fn value_v2(&self) -> Path {
        let mut path = self.value();
        path.segments.push(parse_quote! { v2 });
        path
    }

    /// Path to `IntoValue` trait
    ///
    /// Default: `::canton::ledger_api::types::value::v2::IntoValue`
    pub fn into_value_trait(&self) -> Path {
        let mut path = self.value_v2();
        path.segments.push(parse_quote! { IntoValue });
        path
    }

    /// Path to `TryFromValue` trait
    ///
    /// Default: `::canton::ledger_api::types::value::v2::TryFromValue`
    pub fn try_from_value_trait(&self) -> Path {
        let mut path = self.value_v2();
        path.segments.push(parse_quote! { TryFromValue });
        path
    }

    /// Path to `Value` trait
    ///
    /// Default: `::canton::ledger_api::types::value::v2::Value`
    pub fn value_trait(&self) -> Path {
        let mut path = self.value_v2();
        path.segments.push(parse_quote! { Value });
        path
    }

    /// Path to `IntoRecord` trait
    ///
    /// Default: `::canton::ledger_api::types::value::v2::IntoRecord`
    pub fn into_record_trait(&self) -> Path {
        let mut path = self.value_v2();
        path.segments.push(parse_quote! { IntoRecord });
        path
    }

    /// Path to `TryFromRecord` trait
    ///
    /// Default: `::canton::ledger_api::types::value::v2::TryFromRecord`
    pub fn try_from_record_trait(&self) -> Path {
        let mut path = self.value_v2();
        path.segments.push(parse_quote! { TryFromRecord });
        path
    }

    /// Path to `Record` trait
    ///
    /// Default: `::canton::ledger_api::types::value::v2::Record`
    pub fn record_trait(&self) -> Path {
        let mut path = self.value_v2();
        path.segments.push(parse_quote! { Record });
        path
    }
}

impl Default for Paths {
    /// Initialize paths with default values (with `::canton` as root)
    fn default() -> Self {
        let default_path = Self::default_root();
        Self::from_root(default_path)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use syn::{File, parse_quote};
    use trybuild::TestCases;

    use super::Paths;

    fn test_paths(paths: Paths) {
        let root = paths.root();
        let types = paths.types();
        let ledger_api_types = paths.ledger_api_types();
        let ledger_api_types_v2 = paths.ledger_api_types_v2();
        let value = paths.value();
        let value_v2 = paths.value_v2();
        let into_value_trait = paths.into_value_trait();
        let try_from_value_trait = paths.try_from_value_trait();
        let value_trait = paths.value_trait();
        let into_record_trait = paths.into_record_trait();
        let try_from_record_trait = paths.try_from_record_trait();
        let record_trait = paths.record_trait();
        let contract_id = paths.contract_id();
        let dotted_name = paths.dotted_name();
        let name = paths.name();
        let ledger_string = paths.ledger_string();
        let package_id = paths.package_id();
        let package_name = paths.package_name();
        let non_empty = paths.non_empty();

        let example_code: File = parse_quote! {
            #![allow(unused_imports)]

            use canton as my_canton;

            use #root as _;
            use #types as _;
            use #ledger_api_types as _;
            use #ledger_api_types_v2 as _;
            use #value as _;
            use #value_v2 as _;
            use #into_value_trait as _;
            use #try_from_value_trait as _;
            use #value_trait as _;
            use #into_record_trait as _;
            use #try_from_record_trait as _;
            use #record_trait as _;
            use #contract_id as _;
            use #dotted_name as _;
            use #name as _;
            use #ledger_string as _;
            use #package_id as _;
            use #package_name as _;
            use #non_empty as _;

            fn main() {}
        };
        let content = prettyplease::unparse(&example_code);

        eprintln!("{content}");

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.rs");

        fs::write(&path, content).unwrap();

        let t = TestCases::new();
        t.pass(&path);
    }

    #[test]
    fn test_build_default_paths() {
        test_paths(Default::default());
    }

    #[test]
    fn test_build_custom_paths() {
        test_paths(Paths::from_root(parse_quote!(my_canton)));
    }
}
