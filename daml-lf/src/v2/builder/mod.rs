//! Typed helpers for constructing small Daml LF packages without a Daml compiler.
//!
//! The builders own a small structural LF model which is lowered to protobuf when the
//! package is built. Expressions and raw protobuf mutation are intentionally unavailable.
//!
//! ```
//! use daml_lf::v2::builder::{
//!     DataTypeBuilder, Field, ModuleBuilder, PackageBuilder, Type,
//! };
//!
//! let package = PackageBuilder::new("main-package", "1.0.0")
//!     .module(ModuleBuilder::new("Main").data_type(DataTypeBuilder::record(
//!         "Contract",
//!         [Field::new("owner", Type::party())],
//!     )))
//!     .build();
//!
//! assert_eq!(package.modules.len(), 1);
//! ```

mod data_type;
mod lower;
mod package;
mod template;
mod type_;

pub use self::{
    data_type::{DataTypeBuilder, Field},
    package::{ModuleBuilder, PackageBuilder},
    template::{Binder, ChoiceBuilder, InterfaceBuilder, InterfaceMethod, TemplateBuilder},
    type_::{Kind, Type, TypeConRef, TypeParameter},
};

// Re-export for convenience
pub use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2::BuiltinType;

#[cfg(test)]
mod tests;
