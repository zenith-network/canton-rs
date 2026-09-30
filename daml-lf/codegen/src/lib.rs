use std::path::Path;

use daml_lf::dar::DarFile;

mod config;
mod dispatcher;
mod errors;
mod external_paths;
mod gen_graph_builder;
mod generator;
mod graph;
mod ident;
mod ir;
mod package_ident_generator;
mod package_ref;
mod path;
mod type_attributes;

#[cfg(feature = "v2")]
mod v2;

#[cfg(not(any(feature = "v2")))]
compile_error!("At least one of features [\"v2\"] needs to be enabled for the code generator");

pub use config::Config;
pub use dispatcher::Output;
pub use errors::Error;

use dispatcher::Dispatcher;

/// Read `.dar` file from `dar_path` and generate code with given `config`
///
/// Intended for use in `build.rs` scripts
pub fn generate(dar_path: impl AsRef<Path>, config: Config) -> Result<Output, Error> {
    let mut dar = DarFile::read_from(dar_path)?;
    let dispatcher = Dispatcher::new(config);
    dispatcher.generate(&mut dar)
}
