use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use canton_types::{NonEmpty, PackageId};
use daml_lf::{
    dar::DarFile,
    package::{Package, SealedPackage, VersionedSealedPackage},
};
use syn::Ident;

#[cfg(feature = "v2")]
use crate::v2::package_generator::PackageGenerator as PackageGeneratorV2;
use crate::{
    Config, Error,
    errors::OutputError,
    gen_set_builder::{GenMode, GenSetBuilder},
    helpers::is_empty_mod,
};

/// Output of the code generation
#[derive(Clone, Debug)]
pub struct GenOutput {
    /// Main generated file
    ///
    /// Include this to your lib.rs
    pub main: PathBuf,

    /// All generated files
    pub files: Vec<PathBuf>,
}

/// Generator for a DAR file
pub struct Generator {}

impl Generator {
    pub fn generate<'a>(dar: &'a mut DarFile, config: Config) -> Result<GenOutput, Error> {
        let outdir = config.get_outdir()?;

        let packages = Self::read_packages(dar)?;
        let sealed_packages = Self::seal_packages(&packages)?;

        // FIXME: replace panic with error
        let main_package_id = Self::get_main_package_id(dar)?;

        let package_identifiers = Arc::new(Self::generate_package_identifiers(
            sealed_packages.keys().cloned(),
        ));

        let external_paths = Arc::new(Default::default());

        let gen_set = GenSetBuilder::build(
            &sealed_packages,
            main_package_id.clone(),
            GenMode::ResolveTemplates,
        );

        let mut files = Vec::new();
        for (package_id, package_gen_set) in gen_set.0 {
            // Safety: gen set contains only existing packages
            let package = &sealed_packages[&package_id];
            let ident = &package_identifiers[&package_id];

            match package.versioned() {
                #[cfg(feature = "v2")]
                VersionedSealedPackage::V2(sealed) => {
                    let mut pgen = PackageGeneratorV2::new(
                        package.daml_lf_version(),
                        package.package_id().clone(),
                        sealed,
                        ident.clone(),
                        Arc::clone(&package_identifiers),
                        Arc::clone(&external_paths),
                        package_gen_set,
                    );
                    let pmodule = pgen.gen_package()?;

                    if !is_empty_mod(&pmodule) {
                        let file = syn::File {
                            shebang: None,
                            attrs: Vec::new(),
                            items: vec![syn::Item::Mod(pmodule)],
                        };
                        let path = Self::package_file_path(&outdir, package);
                        Self::write_file(&file, &path)?;
                        files.push(path);
                    }
                }
            }
        }

        let main_package_ident = &package_identifiers[&main_package_id];
        let main_file = Self::generate_main_file(&files, main_package_ident);
        let main = Self::main_file_path(&outdir);
        Self::write_file(&main_file, &main)?;

        Ok(GenOutput { main, files })
    }

    fn read_packages(dar: &mut DarFile) -> Result<Vec<Package>, Error> {
        dar.dalfs()?
            .into_iter()
            .map(|dalf| dalf.to_package())
            .collect::<Result<_, _>>()
            .map_err(Into::into)
    }

    fn seal_packages(
        packages: &[Package],
    ) -> Result<BTreeMap<PackageId, SealedPackage<'_>>, Error> {
        packages
            .iter()
            .map(|package| {
                package
                    .seal()
                    .map(|sealed| (sealed.package_id().clone(), sealed))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(Into::into)
    }

    fn get_main_package_id<'a, 'b>(dar: &mut DarFile) -> Result<PackageId, Error> {
        let main_dalf = dar.main_dalf()?;
        Ok(main_dalf.hash().to_package_id())
    }

    fn generate_main_file(files: &[PathBuf], main_package_ident: &Ident) -> syn::File {
        let mut items: Vec<syn::Item> = files
            .iter()
            .map(|filepath| {
                let filepath_str = filepath.display().to_string();
                syn::parse_quote! { include!(#filepath_str); }
            })
            .collect::<Vec<_>>();
        items.push(syn::parse_quote! { pub use #main_package_ident::*; });
        syn::File {
            shebang: None,
            attrs: Vec::new(),
            items,
        }
    }

    fn main_file_path(outdir: impl AsRef<Path>) -> PathBuf {
        outdir.as_ref().join(format!("main_package.rs"))
    }

    fn package_file_path<'a>(
        outdir: impl AsRef<Path>,
        sealed_package: &SealedPackage<'a>,
    ) -> PathBuf {
        match sealed_package.versioned() {
            VersionedSealedPackage::V2(package) => {
                let metadata = package.metadata();
                let filename = format!("{}-{}.rs", metadata.name(), metadata.version());
                let filepath = outdir.as_ref().join(filename);
                filepath
            }
        }
    }

    fn write_file<'a>(file: &syn::File, path: impl AsRef<Path>) -> Result<(), Error> {
        let output = cfg_select! {
            feature = "format" => prettyplease::unparse(&file),
            _ => quote::ToTokens::into_token_stream(file).to_string(),
        };

        fs::write(path, output).map_err(OutputError::from)?;
        Ok(())
    }

    fn generate_package_identifiers<'a>(
        packages: impl Iterator<Item = PackageId>,
    ) -> HashMap<PackageId, Ident> {
        packages
            .map(|pid| {
                let ident = crate::ident::generate_snake_ident(format!("package_{pid}"));
                (pid, ident)
            })
            .collect::<HashMap<_, _>>()
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     use std::path::Path;
//     use tracing_test::traced_test;

//     #[test]
//     #[traced_test]
//     fn test_codegen_my_contracts() {
//         let mut config = dpm_build::Config::default();
//         config
//             .disable_multi_package()
//             .output("tests/assets/my-contracts.dar")
//             .package_root("tests/assets/my-contracts");
//         let path = config.build().expect("should be able to build Daml").output;

//         let output = test_codegen_main_dalf(path);
//         let output = prettyplease::unparse(&syn::parse2(output).unwrap());
//         println!("{}", output.to_string());
//     }

//     fn test_codegen_main_dalf(path: impl AsRef<Path>) -> TokenStream {
//         let mut dar = DarFile::read_from(path).expect("should be able to read DAR");
//         let dalf = dar.main_dalf().expect("should be able to read main DALF");

//         let mut generator = Generator::new(&mut dar, Config::default().outdir("."));
//         let (output, _) = generator
//             .generate_from_dalf(dalf)
//             .expect("should be able to generate code");
//         output
//     }
// }
