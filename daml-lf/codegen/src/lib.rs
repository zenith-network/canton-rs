use std::path::Path;

use daml_lf::dar::DarFile;

mod config;
mod errors;
mod external_paths;
mod gen_set_builder;
mod generator;
mod helpers;
mod ident;
mod ids;
mod path;
mod type_sets;

#[cfg(feature = "v2")]
mod v2;

#[cfg(not(any(feature = "v2")))]
compile_error!("At least one of features [\"v2\"] needs to be enabled for the code generator");

pub use config::Config;
pub use errors::Error;
pub use generator::GenOutput;

use generator::Generator;

/// Read `.dar` file from `dar_path` and generate code with given `config`
///
/// Intended for use in `build.rs` scripts
pub fn generate(dar_path: impl AsRef<Path>, config: Config) -> Result<GenOutput, Error> {
    println!("cargo::rerun-if-changed={}", dar_path.as_ref().display());
    let mut dar = DarFile::read_from(dar_path)?;
    Generator::generate(&mut dar, config)
}
