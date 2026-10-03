use std::{
    collections::BTreeMap,
    fs,
    path::{self, Path, PathBuf},
};

use canton_types::PackageId;
use daml_lf::{
    dar::DarFile,
    package::{Package, SealedPackage},
};
use syn::Ident;

use crate::{
    Config, Error,
    errors::{OutdirNotSet, OutputError},
    gen_graph_builder::GenGraphBuilder,
    generator::{GenCtx, Generator},
    package_ident_generator::{PackageIdentGenerator, PackageWithIdent},
    type_attributes::TypeAttributes,
};

/// Output of the code generation
#[derive(Clone, Debug)]
pub struct Output {
    /// Main generated file
    ///
    /// Include this to your lib.rs
    pub main: PathBuf,

    /// All generated files
    pub files: Vec<PathBuf>,
}

/// Map from package IDs to sealed packages with generated identifiers
pub type Packages<'a> = BTreeMap<PackageId, PackageWithIdent<'a>>;

/// Dispatcher of generation for a DAR file
#[derive(Clone, Debug)]
pub struct Dispatcher {
    config: Config,
}

impl Dispatcher {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn generate(&self, dar: &mut DarFile) -> Result<Output, Error> {
        let outdir = self.get_outdir()?;
        let external_paths = Default::default();
        let paths = Default::default();

        // Extract and seal packages from DAR
        let raw_packages = Self::read_packages(dar)?;
        let sealed_packages = Self::seal_packages(&raw_packages)?;

        // Generate identifiers for packages
        let packages = PackageIdentGenerator::new(sealed_packages).generate();

        let type_attrs = TypeAttributes::resolve(&self.config.type_attrs, &packages).unwrap(); // FIXME: remove unwrap

        // Construct generation context
        let ctx = GenCtx::new(&packages, paths, external_paths, type_attrs);

        // Build generation graph
        let gen_graph = GenGraphBuilder::build(&ctx);

        // Run code generation
        let generated_packages = Generator::generate(gen_graph, &ctx);

        let mut files = Vec::new();

        for (package_id, package) in generated_packages {
            let file = package.render();
            let ident = &ctx.packages()[&package_id].ident;
            let path = Self::package_file_path(&outdir, ident);
            Self::write_file(&file, &path)?;
            files.push(path);
        }

        let main_package_id = Self::get_main_package_id(dar)?;
        let main_package_ident = &ctx.packages()[&main_package_id].ident;
        let main_file = Self::generate_main_file(&files, main_package_ident);
        let main = Self::main_file_path(&outdir);
        Self::write_file(&main_file, &main)?;
        files.push(main.clone());

        Ok(Output { main, files })
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

    fn get_main_package_id(dar: &mut DarFile) -> Result<PackageId, Error> {
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
        items.push(syn::parse_quote! { pub use self::#main_package_ident::*; });
        syn::File {
            shebang: None,
            frontmatter: None,
            attrs: Vec::new(),
            items,
        }
    }

    fn main_file_path(outdir: impl AsRef<Path>) -> PathBuf {
        outdir.as_ref().join("main_package.rs")
    }

    fn package_file_path(outdir: impl AsRef<Path>, package_ident: &Ident) -> PathBuf {
        outdir.as_ref().join(format!("{package_ident}.rs"))
    }

    fn write_file(file: &syn::File, path: impl AsRef<Path>) -> Result<(), Error> {
        #[cfg(feature = "format")]
        let output = prettyplease::unparse(file);
        #[cfg(not(feature = "format"))]
        let output = quote::ToTokens::into_token_stream(file).to_string();

        fs::write(path, output).map_err(OutputError::from)?;
        Ok(())
    }

    fn get_outdir(&self) -> Result<PathBuf, Error> {
        Ok(path::absolute(resolve_outdir(self.config.outdir.clone())?).unwrap()) // FIXME: remove unwrap
    }
}

/// If path is not set and `"env"` feature is enabled, try to get the path from `OUT_DIR` env var
pub fn resolve_outdir(#[allow(unused_mut)] mut path: Option<PathBuf>) -> Result<PathBuf, Error> {
    #[cfg(feature = "env")]
    {
        path = path.or_else(|| std::env::var("OUT_DIR").ok().map(PathBuf::from));
    }
    path.ok_or(OutdirNotSet.into())
}
