use std::{
    env,
    error::Error as StdError,
    fmt, fs,
    path::{Path, PathBuf},
    process::exit,
};

use tonic_prost_build::{Builder, Config};

pub fn main() {
    if let Err(error) = try_main() {
        eprintln!("{error:#}");
        exit(1);
    }
}

fn try_main() -> Result<(), Error> {
    let manifest_dir_var = env::var("CARGO_MANIFEST_DIR")
        .map_err(|err| Error::new("failed to get CARGO_MANIFEST_DIR", err))?;
    let manifest_dir = PathBuf::from(manifest_dir_var);

    let source_dir = manifest_dir.join("protobuf");

    let protos = collect_protos(&source_dir)?;

    if protos.is_empty() {
        return Ok(());
    }

    let includes = collect_include_dirs(&manifest_dir, source_dir)?;

    let mut config = Config::new();
    config.enable_type_names();

    let mut builder = tonic_prost_build::configure();
    builder = extern_paths(builder);

    builder
        .compile_with_config(config, &protos, &includes)
        .map_err(|err| Error::new("failed to generate code from Protobuf sources", err))
}

fn collect_protos(source_dir: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut protos = Vec::new();

    if env::var_os("CARGO_FEATURE_V30").is_some() {
        let dirs = &[
            "com/digitalasset/canton/admin/crypto/v30",
            "com/digitalasset/canton/admin/health/v30",
            "com/digitalasset/canton/admin/mediator/v30",
            "com/digitalasset/canton/admin/participant/v30",
            "com/digitalasset/canton/admin/pruning/v30",
            "com/digitalasset/canton/admin/sequencer/v30",
            "com/digitalasset/canton/admin/time/v30",
            "com/digitalasset/canton/admin/topology/v30",
            "com/digitalasset/canton/topology/admin/v30",
        ];
        for dir in dirs {
            visit_dir(&source_dir.join(dir), &mut protos)?;
        }
    }

    visit_dir(
        &source_dir.join("com/digitalasset/canton/admin/scalapb"),
        &mut protos,
    )?;
    visit_dir(
        &source_dir.join("com/digitalasset/canton/topology/admin/scalapb"),
        &mut protos,
    )?;

    Ok(protos)
}

fn visit_dir(dir: &Path, accumulator: &mut Vec<PathBuf>) -> Result<(), Error> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in
        fs::read_dir(dir).map_err(|err| Error::new("failed to collect Protobuf sources", err))?
    {
        let entry = entry.map_err(|err| Error::new("failed to collect Protobuf sources", err))?;
        let path = entry.path();

        if path.is_dir() {
            visit_dir(&path, accumulator)?;
        } else if path.extension().is_some_and(|ext| ext == "proto") {
            accumulator.push(path);
        }
    }

    Ok(())
}

fn collect_include_dirs(manifest_dir: &Path, source_dir: PathBuf) -> Result<Vec<PathBuf>, Error> {
    let vendored_dir = manifest_dir.join("vendored");
    Ok(vec![source_dir, vendored_dir])
}

fn extern_paths(builder: Builder) -> Builder {
    builder
}

#[derive(Debug)]
struct Error {
    message: String,
    source: Option<Box<dyn StdError + 'static>>,
}

impl Error {
    fn new<E: StdError + 'static>(message: impl Into<String>, source: E) -> Self {
        Self {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    fn full_message(&self) -> String {
        let mut message = self.message.clone();
        let mut sources = Vec::new();

        let mut source = self.source();
        while let Some(error) = source {
            sources.push(format!(
                " - {}",
                error
                    .to_string()
                    .lines()
                    .collect::<Vec<_>>()
                    .join("\n     ")
            ));
            source = error.source();
        }

        if !sources.is_empty() {
            message.push_str(&format!("\n\nCaused by:\n{}", sources.join("\n")));
        }

        message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "{}", self.full_message())
        } else {
            self.message.fmt(f)
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_deref()
    }
}
