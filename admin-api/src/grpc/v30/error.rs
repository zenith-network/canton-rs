/// Error while constructing or connecting an Admin API client.
#[derive(Debug, thiserror::Error)]
pub enum ClientBuildError {
    #[error(transparent)]
    Transport(#[from] tonic::transport::Error),
}
