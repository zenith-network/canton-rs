use canton_proto::com::digitalasset::canton::crypto::v30 as proto;
use canton_types::topology::Fingerprint;
use chrono::{DateTime, Duration, Utc};
use protobuf_utils::{InvalidProtoField as _, InvalidProtoFieldValue, UnspecifiedEnumValue};

use crate::v30::{ConvertError, SigningKeySpec, error::TimestampOutOfRange};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SignatureFormat {
    /// Signature scheme specific signature format
    /// Legacy format no longer used, except for migrations
    Raw,

    /// ASN.1 + DER-encoding of the `r` and `s` integers, as defined in
    /// <https://datatracker.ietf.org/doc/html/rfc3279#section-2.2.3>
    ///
    /// Used for ECDSA signatures
    Der,

    /// Concatenation of the integers `r || s` in little-endian form, as defined in
    /// <https://datatracker.ietf.org/doc/html/rfc8032#section-3.3>
    ///
    /// Note that this is different from the format defined in IEEE P1363, which uses concatenation
    /// in big-endian form.
    ///
    /// Used for EdDSA signatures
    Concat,

    /// Symbolic crypto, must only be used for testing
    Symbolic,
}

impl From<SignatureFormat> for proto::SignatureFormat {
    fn from(value: SignatureFormat) -> Self {
        match value {
            SignatureFormat::Raw => Self::Raw,
            SignatureFormat::Der => Self::Der,
            SignatureFormat::Concat => Self::Concat,
            SignatureFormat::Symbolic => Self::Symbolic,
        }
    }
}

impl TryFrom<proto::SignatureFormat> for SignatureFormat {
    type Error = ConvertError;

    fn try_from(value: proto::SignatureFormat) -> Result<Self, Self::Error> {
        match value {
            proto::SignatureFormat::Unspecified => Err(UnspecifiedEnumValue::new().into()),
            proto::SignatureFormat::Raw => Ok(Self::Raw),
            proto::SignatureFormat::Der => Ok(Self::Der),
            proto::SignatureFormat::Concat => Ok(Self::Concat),
            proto::SignatureFormat::Symbolic => Ok(Self::Symbolic),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SigningAlgorithmSpec {
    /// EdDSA Signature based on Curve25519 with SHA-512
    ///
    /// <http://ed25519.cr.yp.to/>
    Ed25519,

    /// Elliptic Curve Digital Signature Algorithm with SHA256
    EcDsaSha256,

    /// Elliptic Curve Digital Signature Algorithm with SHA384
    EcDsaSha384,

    /// PQC Signing algorithm spec for ML-DSA-65 as defined in
    /// <https://doi.org/10.6028/NIST.FIPS.204>
    MlDsa65,
}

impl From<SigningAlgorithmSpec> for proto::SigningAlgorithmSpec {
    fn from(value: SigningAlgorithmSpec) -> Self {
        match value {
            SigningAlgorithmSpec::Ed25519 => Self::Ed25519,
            SigningAlgorithmSpec::EcDsaSha256 => Self::EcDsaSha256,
            SigningAlgorithmSpec::EcDsaSha384 => Self::EcDsaSha384,
            SigningAlgorithmSpec::MlDsa65 => Self::MlDsa65,
        }
    }
}

impl TryFrom<proto::SigningAlgorithmSpec> for SigningAlgorithmSpec {
    type Error = UnspecifiedEnumValue;

    /// Returns error if the value is `UNSPECIFIED`
    fn try_from(value: proto::SigningAlgorithmSpec) -> Result<Self, Self::Error> {
        match value {
            proto::SigningAlgorithmSpec::Unspecified => Err(UnspecifiedEnumValue::new()),
            proto::SigningAlgorithmSpec::Ed25519 => Ok(Self::Ed25519),
            proto::SigningAlgorithmSpec::EcDsaSha256 => Ok(Self::EcDsaSha256),
            proto::SigningAlgorithmSpec::EcDsaSha384 => Ok(Self::EcDsaSha384),
            proto::SigningAlgorithmSpec::MlDsa65 => Ok(Self::MlDsa65),
        }
    }
}

/// Encoded signature
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Signature {
    format: SignatureFormat,
    signature: Vec<u8>,
    signed_by: Fingerprint,
    signing_algorithm_spec: Option<SigningAlgorithmSpec>,
    signature_delegation: Option<SignatureDelegation>,
}

impl Signature {
    /// Create a new signature
    ///
    /// This function does not check, that signature is actually a `signing_algorithm_spec`
    /// signature encoded with `format`. User is responsible for that.
    pub fn new(
        format: SignatureFormat,
        signature: Vec<u8>,
        signed_by: Fingerprint,
        signing_algorithm_spec: SigningAlgorithmSpec,
    ) -> Self {
        Self {
            format,
            signature,
            signed_by,
            signing_algorithm_spec: Some(signing_algorithm_spec),
            signature_delegation: None,
        }
    }

    /// Add a delegation signature
    ///
    /// Note that with signature delegation `signed_by` must be set to authorizing key.
    pub fn with_delegation(mut self, delegation: SignatureDelegation) -> Self {
        self.signature_delegation = Some(delegation);
        self
    }

    /// Create a symbolic signature for testing.
    ///
    /// Appends a four-byte, big-endian counter to the input bytes. Provides no cryptographic
    /// authentication.
    pub fn symbolic(bytes: Vec<u8>, signed_by: Fingerprint, counter: i32) -> Self {
        let mut signature = bytes;
        signature.extend_from_slice(&counter.to_be_bytes());

        Self {
            format: SignatureFormat::Symbolic,
            signature,
            signed_by,
            signing_algorithm_spec: None,
            signature_delegation: None,
        }
    }

    /// Create "no signature" placeholder.
    ///
    /// This is not a valid signature for verification.
    pub fn no_signature() -> Self {
        Self {
            format: SignatureFormat::Symbolic,
            signature: Vec::new(),
            signed_by: Fingerprint::new_unchecked("no-fingerprint".into()),
            signing_algorithm_spec: None,
            signature_delegation: None,
        }
    }

    /// Encoding format of the signature
    pub fn format(&self) -> SignatureFormat {
        self.format
    }

    /// Signature encoded according to the `format`
    pub fn signature(&self) -> &[u8] {
        &self.signature
    }

    /// The fingerprint/ID of the keypair used to create this signature and needed to verify.
    ///
    /// If a signature delegation is defined, this fingerprint/ID is not used to produce the
    /// signature, and, instead is used to sign the delegation.
    pub fn signed_by(&self) -> &Fingerprint {
        &self.signed_by
    }

    /// The signing algorithm specification used to produce this signature.
    pub fn signing_algorithm_spec(&self) -> Option<SigningAlgorithmSpec> {
        self.signing_algorithm_spec
    }

    /// Signature delegation where a long-term signing key authorizes a session signing key to sign
    /// on its behalf.
    pub fn signature_delegation(&self) -> Option<&SignatureDelegation> {
        self.signature_delegation.as_ref()
    }
}

impl From<Signature> for proto::Signature {
    fn from(value: Signature) -> Self {
        Self {
            format: proto::SignatureFormat::from(value.format).into(),
            signature: value.signature,
            signed_by: value.signed_by.into(),
            signing_algorithm_spec: value
                .signing_algorithm_spec
                .map(proto::SigningAlgorithmSpec::from)
                // If signing algorithm is not set, default to UNSPECIFIED
                .unwrap_or_default()
                .into(),
            signature_delegation: value.signature_delegation.map(Into::into),
        }
    }
}

impl TryFrom<proto::Signature> for Signature {
    type Error = ConvertError;

    fn try_from(value: proto::Signature) -> Result<Self, Self::Error> {
        let format = proto::SignatureFormat::try_from(value.format)
            .validated_of::<proto::Signature>("format")?
            .try_into()
            .validated_of::<proto::Signature>("format")?;
        let signed_by =
            Fingerprint::try_from(value.signed_by).validated_of::<proto::Signature>("signed_by")?;
        let signing_algorithm_spec =
            proto::SigningAlgorithmSpec::try_from(value.signing_algorithm_spec)
                .validated_of::<proto::Signature>("signing_algorithm_spec")?
                .try_into()
                .ok(); // UNSPECIFIED algorithm is treated as None for backwards compatibility
        let signature_delegation = value
            .signature_delegation
            .map(TryInto::try_into)
            .transpose()?;
        Ok(Self {
            format,
            signature: value.signature,
            signed_by,
            signing_algorithm_spec,
            signature_delegation,
        })
    }
}

/// Encoded signature delegation
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SignatureDelegation {
    session_key: Vec<u8>,
    session_key_spec: SigningKeySpec,
    validity_period_from_inclusive: DateTime<Utc>,
    validity_period_duration: Duration,
    format: SignatureFormat,
    signature: Vec<u8>,
    signing_algorithm_spec: Option<SigningAlgorithmSpec>,
}

impl SignatureDelegation {
    /// Get a builder for signature delegation
    pub fn builder() -> SignatureDelegationBuilder {
        SignatureDelegationBuilder::new()
    }

    /// ASN.1 + DER-encoding of X.509 SubjectPublicKeyInfo structure of the session public key:
    /// <https://datatracker.ietf.org/doc/html/rfc5280#section-4.1.>
    ///
    /// This key was used to produce the actual signature.
    pub fn session_key(&self) -> &[u8] {
        &self.session_key
    }

    /// The signing key spec for the session key.
    pub fn session_key_spec(&self) -> SigningKeySpec {
        self.session_key_spec
    }

    /// A timestamp that indicates when the session key is considered valid.
    pub fn validity_period_from_inclusive(&self) -> DateTime<Utc> {
        self.validity_period_from_inclusive
    }

    /// Duration indicating how long the session key remains valid.
    ///
    /// The validity period starts from `validity_period_from_inclusive` above.
    pub fn validity_period_duration(&self) -> Duration {
        self.validity_period_duration
    }

    /// Encoding format of the signature.
    pub fn format(&self) -> SignatureFormat {
        self.format
    }

    /// The signature of the combined hash of the session key fingerprint, validity period, and
    /// synchronizer ID, by the long-term key. This signature authorizes the session key to act on
    /// behalf of the long-term key.
    pub fn signature(&self) -> &[u8] {
        &self.signature
    }

    /// The signing algorithm specification used to produce this signature
    pub fn signing_algorithm_spec(&self) -> Option<SigningAlgorithmSpec> {
        self.signing_algorithm_spec
    }
}

impl From<SignatureDelegation> for proto::SignatureDelegation {
    /// Panics on invalid duration.
    fn from(value: SignatureDelegation) -> Self {
        Self {
            session_key: value.session_key,
            session_key_spec: proto::SigningKeySpec::from(value.session_key_spec).into(),
            validity_period_from_inclusive: value.validity_period_from_inclusive.timestamp_micros(),
            validity_period_duration_seconds: u32::try_from(
                value.validity_period_duration.num_seconds(),
            )
            // Safety: safe to unwrap because of the internal invariant
            .expect("invalid duration"),
            format: proto::SignatureFormat::from(value.format).into(),
            signature: value.signature,
            signing_algorithm_spec: value
                .signing_algorithm_spec
                .map(proto::SigningAlgorithmSpec::from)
                // If signing algorithm is not set, default to UNSPECIFIED
                .unwrap_or_default()
                .into(),
        }
    }
}

impl TryFrom<proto::SignatureDelegation> for SignatureDelegation {
    type Error = ConvertError;

    fn try_from(value: proto::SignatureDelegation) -> Result<Self, Self::Error> {
        let mut builder = Self::builder()
            .session_key(value.session_key)
            .signature(value.signature)
            .session_key_spec(
                proto::SigningKeySpec::try_from(value.session_key_spec)
                    .validated_of::<proto::SignatureDelegation>("session_key_spec")?
                    .try_into()
                    .validated_of::<proto::SignatureDelegation>("session_key_spec")?,
            )
            .validity_period_from_inclusive(
                DateTime::from_timestamp_micros(value.validity_period_from_inclusive).ok_or_else(
                    || TimestampOutOfRange::new(value.validity_period_from_inclusive),
                )?,
            )
            .validity_period_duration(Duration::seconds(
                value.validity_period_duration_seconds.into(),
            ))
            .format(
                proto::SignatureFormat::try_from(value.format)
                    .validated_of::<proto::SignatureDelegation>("format")?
                    .try_into()
                    .validated_of::<proto::SignatureDelegation>("format")?,
            );

        // UNSPECIFIED algorithm is treated as None for backwards compatibility
        if let Ok(signing_algorithm_spec) =
            proto::SigningAlgorithmSpec::try_from(value.signing_algorithm_spec)
                .validated_of::<proto::SignatureDelegation>("signing_algorithm_spec")?
                .try_into()
        {
            builder = builder.signing_algorithm_spec(signing_algorithm_spec);
        }

        builder
            .build()
            .map_err(|error| {
                let field_name = match error {
                    SignatureDelegationError::SessionKeyIsEmpty => "session_key",
                    SignatureDelegationError::SessionKeySpecUnset => "session_key_spec",
                    SignatureDelegationError::SignatureIsEmpty => "signature",
                    SignatureDelegationError::ValidityTimestampUnset
                    | SignatureDelegationError::InvalidValidityTimestamp => {
                        "validity_period_from_inclusive"
                    }
                    SignatureDelegationError::ValidityPeriodUnset
                    | SignatureDelegationError::InvalidValidityPeriod => {
                        "validity_period_duration_seconds"
                    }
                    SignatureDelegationError::FormatUnset => "format",
                };
                InvalidProtoFieldValue::new_of::<proto::SignatureDelegation>(field_name, error)
            })
            .map_err(Into::into)
    }
}

// NOTE: We provide a builder here, because the type has quite a lot of fields and lacks reasonable
// defaults

/// Signature delegation builder
#[derive(Clone, Debug, Default)]
pub struct SignatureDelegationBuilder {
    session_key: Vec<u8>,
    session_key_spec: Option<SigningKeySpec>,
    validity_period_from_inclusive: Option<DateTime<Utc>>,
    validity_period_duration: Option<Duration>,
    format: Option<SignatureFormat>,
    signature: Vec<u8>,
    signing_algorithm_spec: Option<SigningAlgorithmSpec>,
}

impl SignatureDelegationBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set session key
    pub fn session_key(mut self, session_key: Vec<u8>) -> Self {
        self.session_key = session_key;
        self
    }

    /// Set session key specification
    pub fn session_key_spec(mut self, spec: SigningKeySpec) -> Self {
        self.session_key_spec = Some(spec);
        self
    }

    /// Set validity period timestamp
    pub fn validity_period_from_inclusive(mut self, timestamp: DateTime<Utc>) -> Self {
        self.validity_period_from_inclusive = Some(timestamp);
        self
    }

    /// Set validity period duration
    pub fn validity_period_duration(mut self, duration: Duration) -> Self {
        self.validity_period_duration = Some(duration);
        self
    }

    /// Set signature encoding format
    pub fn format(mut self, format: SignatureFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Set signature bytes
    pub fn signature(mut self, signature: Vec<u8>) -> Self {
        self.signature = signature;
        self
    }

    /// Set signing algorithm specification
    pub fn signing_algorithm_spec(mut self, spec: SigningAlgorithmSpec) -> Self {
        self.signing_algorithm_spec = Some(spec);
        self
    }

    /// Build signature delegation
    pub fn build(self) -> Result<SignatureDelegation, SignatureDelegationError> {
        let Self {
            session_key,
            session_key_spec,
            validity_period_from_inclusive,
            validity_period_duration,
            format,
            signature,
            signing_algorithm_spec,
        } = self;
        if session_key.is_empty() {
            return Err(SignatureDelegationError::SessionKeyIsEmpty);
        }
        let session_key_spec =
            session_key_spec.ok_or(SignatureDelegationError::SessionKeySpecUnset)?;

        let validity_period_from_inclusive = validity_period_from_inclusive
            .ok_or(SignatureDelegationError::ValidityTimestampUnset)?;

        // Check that validity_period_from_inclusive matches Canton requirements
        const MIN_TIMESTAMP_MICROS: i64 = -62_135_596_800_000_000;
        const MAX_TIMESTAMP_MICROS: i64 = 253_402_300_799_999_999;
        let micros = validity_period_from_inclusive.timestamp_micros();

        // Require an exact round trip through Unix microseconds.
        // This also rejects Chrono leap-second representations.
        if !(MIN_TIMESTAMP_MICROS..=MAX_TIMESTAMP_MICROS).contains(&micros)
            || DateTime::<Utc>::from_timestamp_micros(micros)
                != Some(validity_period_from_inclusive)
        {
            return Err(SignatureDelegationError::InvalidValidityTimestamp);
        }

        let validity_period_duration =
            validity_period_duration.ok_or(SignatureDelegationError::ValidityPeriodUnset)?;

        // Check that validity_period_duration matches Canton requirements
        if !(1..=i64::from(i32::MAX)).contains(&validity_period_duration.num_seconds())
            || validity_period_duration.subsec_nanos() != 0
        {
            return Err(SignatureDelegationError::InvalidValidityPeriod);
        }

        let format = format.ok_or(SignatureDelegationError::FormatUnset)?;

        if signature.is_empty() {
            return Err(SignatureDelegationError::SignatureIsEmpty);
        }

        Ok(SignatureDelegation {
            session_key,
            session_key_spec,
            validity_period_from_inclusive,
            validity_period_duration,
            format,
            signature,
            signing_algorithm_spec,
        })
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error(transparent)]
pub enum SignatureDelegationError {
    #[error("session key is empty")]
    SessionKeyIsEmpty,
    #[error("session key specification is not set")]
    SessionKeySpecUnset,
    #[error("signature is empty")]
    SignatureIsEmpty,
    #[error("validity period timestamp is not set")]
    ValidityTimestampUnset,
    #[error("invalid validity period timestamp")]
    InvalidValidityTimestamp,
    #[error("validity period is not set")]
    ValidityPeriodUnset,
    #[error("invalid validity period (must be a whole number of seconds in 1..=2147483647)")]
    InvalidValidityPeriod,
    #[error("encoding format is not set")]
    FormatUnset,
}
