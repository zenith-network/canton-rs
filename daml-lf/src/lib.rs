pub mod dalf;
pub mod dar;
pub mod dar_manifest;
pub mod hash;

#[allow(clippy::non_minimal_cfg)]
#[cfg(any(feature = "v2"))]
pub mod package;

#[cfg(feature = "v2")]
pub mod v2;

pub use daml_lf_archive_proto as proto;

#[derive(Debug, thiserror::Error)]
pub enum MalformedPackage {
    #[cfg(feature = "v2")]
    #[error(transparent)]
    V2(#[from] v2::errors::MalformedPackage),
}
