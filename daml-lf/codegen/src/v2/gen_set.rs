use std::collections::{HashMap, HashSet};

use canton_types::NonEmpty;
use daml_lf::v2::sealed::DottedName;

// /// Generation set (package ID -> package gen set)
// ///
// /// Defines the types to generate. Resolved before generation starts. Each package and module is
// /// queries to generate only specified types.
// pub type GenSet<'a> = HashMap<String, PackageGenSet<'a>>;

// /// Generation set for a package (module name -> module gen set)
// pub type PackageGenSet<'a> = HashMap<DottedName<'a>, ModuleGenSet<'a>>;

// /// Generation set for a module (set of names of data types to generate within a module)
// pub type ModuleGenSet<'a> = HashSet<DottedName<'a>>;
