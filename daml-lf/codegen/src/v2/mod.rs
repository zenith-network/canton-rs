use canton_types::NonEmpty;
use daml_lf::v2::sealed::DottedName;

use crate::ids::OwnedDottedName;

pub mod module_deps_resolver;
pub mod module_gen_set_builder;
pub mod module_generator;
pub mod package_gen_set_builder;
pub mod package_generator;

pub fn dotted_name_to_owned<'a>(name: &DottedName<'a>) -> OwnedDottedName {
    NonEmpty {
        base: name.base().into_iter().map(|s| s.to_string()).collect(),
        tail: name.tail().to_owned(),
    }
}

// impl<'a> From<TypeConId<'a>> for OwnedTypeConId {
//     fn from(value: TypeConId<'a>) -> Self {
//         Self {
//             module: value.module().into(),
//             name: dotted_name_to_owned(&value.name()),
//         }
//     }
// }

// impl<'a> From<ModuleId<'a>> for OwnedModuleId {
//     fn from(value: ModuleId<'a>) -> Self {
//         Self {
//             package_id: value.package_id().into(),
//             module_name: dotted_name_to_owned(&value.module_name()),
//         }
//     }
// }

// impl<'a> From<SelfOrImportedPackageId<'a>> for OwnedSelfOrImportedPackageId {
//     fn from(value: SelfOrImportedPackageId<'a>) -> Self {
//         match value {
//             SelfOrImportedPackageId::SelfPackageId => Self::SelfPackageId,
//             SelfOrImportedPackageId::ImportedPackageId(id) => {
//                 Self::ImportedPackageId(id.to_owned())
//             }
//         }
//     }
// }
