//! Sealed Daml LF v2 representation
//!
//! Essentially, sealing means converting interned value indices to references and validating the
//! package.

mod builtin_type;
mod def_interface;
mod dotted_name;
mod field_with_type;
mod ids;
mod interface_method;
mod module;
mod package;
mod package_metadata;
mod template_choice;
mod type_var_with_kind;
mod var_with_type;

pub mod def_data_type;
pub mod def_template;
pub mod kind;
pub mod type_;

pub use builtin_type::BuiltinType;
pub use def_data_type::DefDataType;
pub use def_interface::DefInterface;
pub use def_template::DefTemplate;
pub use dotted_name::DottedName;
pub use field_with_type::FieldWithType;
pub use ids::{ModuleId, SelfOrImportedPackageId, TypeConId};
pub use interface_method::InterfaceMethod;
pub use kind::Kind;
pub use module::Module;
pub use package::Package;
pub use package_metadata::PackageMetadata;
pub use template_choice::TemplateChoice;
pub use type_::Type;
pub use type_var_with_kind::TypeVarWithKind;
pub use var_with_type::VarWithType;

pub type SealedPackage<'a> = Package<'a>;
