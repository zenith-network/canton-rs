use canton_proto::com::digitalasset::canton::crypto::v30 as proto;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SigningPublicKey {
    // TODO: implement this
}

impl From<SigningPublicKey> for proto::SigningPublicKey {
    fn from(value: SigningPublicKey) -> Self {
        todo!()
    }
}

impl TryFrom<proto::SigningPublicKey> for SigningPublicKey {
    type Error = ();

    fn try_from(value: proto::SigningPublicKey) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SigningKeysWithThreshold {
    /// Designated signing keys
    pub keys: Vec<SigningPublicKey>,

    /// Authorization threshold
    pub threshold: u32,
}

impl From<SigningKeysWithThreshold> for proto::SigningKeysWithThreshold {
    fn from(value: SigningKeysWithThreshold) -> Self {
        Self {
            keys: value.keys.into_iter().map(Into::into).collect(),
            threshold: value.threshold,
        }
    }
}

impl TryFrom<proto::SigningKeysWithThreshold> for SigningKeysWithThreshold {
    type Error = ();

    fn try_from(value: proto::SigningKeysWithThreshold) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PublicKey {
    SigningPublicKey(SigningPublicKey),
    // EncryptionPublicKey(EncryptionPublicKey),
}

impl From<PublicKey> for proto::PublicKey {
    fn from(value: PublicKey) -> Self {
        todo!()
    }
}

impl TryFrom<proto::PublicKey> for PublicKey {
    type Error = ();

    fn try_from(value: proto::PublicKey) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Signature {}

impl From<Signature> for proto::Signature {
    fn from(value: Signature) -> Self {
        todo!()
    }
}

impl TryFrom<proto::Signature> for Signature {
    type Error = ();

    fn try_from(value: proto::Signature) -> Result<Self, Self::Error> {
        todo!()
    }
}
