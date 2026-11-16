mod sealed {
    /// Sealing trait for [`super::HashPurpose`]
    pub trait HashPurpose {}
}

/// The purpose of a hash serves to avoid hash collisions due to equal encodings for different
/// objects.
///
/// It is in general not possible to derive the purpose of the hash from the hash alone.
pub trait HashPurpose: sealed::HashPurpose {
    /// The identifier for the `HashPurpose`. Every `HashPurpose` type must have a unique ID.
    const ID: i32;

    /// Description of a hash purpose
    const DESC: &'static str;
}

/// Defines a simple ZST representing a hash purpose
///
/// Usage:
///
/// ```rust,ignore
/// define_hash_purpose!(MyPurpose, 42, "MyPurpose");
/// ```
macro_rules! define_hash_purpose {
    ($(#[$meta:meta])* $name:ident, $id:expr, $desc:expr $(,)?) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        pub struct $name;

        impl sealed::HashPurpose for $name {}

        impl HashPurpose for $name {
            const ID: i32 = $id;
            const DESC: &'static str = $desc;
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(<Self as HashPurpose>::DESC)
            }
        }
    };
}

define_hash_purpose! {
    /// Public key fingerprint hash purpose
    PublicKeyFingerprint, 12, "PublicKeyFingerprint"
}

define_hash_purpose! {
    /// Decentralized namespace hash purpose
    DecentralizedNamespace, 37, "DecentralizedNamespace"
}
