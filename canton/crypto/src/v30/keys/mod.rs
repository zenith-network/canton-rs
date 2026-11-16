mod encryption_public_key;
mod public_key;
mod public_signing_key;
mod signing_keys_with_threshold;

pub use encryption_public_key::{EncodedEncryptionPublicKey, EncryptionPublicKeyError};
pub use public_key::EncodedPublicKey;
pub use public_signing_key::{
    DecodedSigningPublicKeyOwned, DecodedSigningPublicKeyRef, EncodedSigningPublicKey,
    SigningPublicKeyError,
};
pub use signing_keys_with_threshold::{SigningKeysWithThreshold, SigningKeysWithThresholdError};

/// Referenced decoded key
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodedKeyRef<'a> {
    #[cfg(feature = "spki")]
    Spki(spki::SubjectPublicKeyInfoRef<'a>),
    Raw(&'a [u8]),
    Symbolic(&'a [u8]),
}

/// Owned decoded key
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodedKeyOwned {
    #[cfg(feature = "spki")]
    Spki(spki::SubjectPublicKeyInfoOwned),
    Raw(Vec<u8>),
    Symbolic(Vec<u8>),
}
