use daml_lf::{MalformedPackage, dalf::DalfError, dar::DarError, proto::daml_lf_version::Version};

#[derive(Debug, thiserror::Error)]
#[error("unsupported Daml LF version")]
pub struct UnsupportedVersion {
    version: Version,
}

impl UnsupportedVersion {
    pub fn new(version: Version) -> Self {
        Self { version }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("output directory is not set")]
pub struct OutdirNotSet;

#[derive(Debug, thiserror::Error)]
#[error("generated code ({file}) has a syntax error")]
pub struct SyntaxError {
    file: String,
    #[source]
    source: syn::Error,
}

impl SyntaxError {
    pub fn new(file: String, source: syn::Error) -> Self {
        Self { file, source }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed to write output file")]
pub struct OutputError {
    #[from]
    source: std::io::Error,
}

#[derive(Debug, thiserror::Error)]
#[error("failed to generate code")]
pub enum Error {
    DarError(#[from] DarError),
    DalfError(#[from] DalfError),
    UnsupportedVersion(#[from] UnsupportedVersion),
    MalformedPackage(#[from] MalformedPackage),
    OutdirNotSet(#[from] OutdirNotSet),
    SyntaxError(#[from] SyntaxError),
    OutputError(#[from] OutputError),
}
