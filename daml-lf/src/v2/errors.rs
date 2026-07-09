use std::{error::Error, num::TryFromIntError};

#[derive(Debug, thiserror::Error)]
#[error("{msg}")]
pub struct MalformedPackage {
    msg: String,
    #[source]
    source: Option<Box<dyn Error + 'static + Send + Sync>>,
}

impl MalformedPackage {
    pub fn new(msg: String, source: Option<Box<dyn Error + 'static + Send + Sync>>) -> Self {
        Self { msg, source }
    }

    pub fn without_source(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            source: None,
        }
    }
}

impl<T: Into<String>> From<T> for MalformedPackage {
    fn from(value: T) -> Self {
        Self {
            msg: value.into(),
            source: None,
        }
    }
}

pub trait MalformedPackageContext: Sized {
    type Ok;

    fn context(self, msg: impl Into<String>) -> Result<Self::Ok, MalformedPackage>;

    fn default_context(self) -> Result<Self::Ok, MalformedPackage> {
        self.context("malformed package")
    }
}

impl<T, E: Error + 'static + Send + Sync> MalformedPackageContext for Result<T, E> {
    type Ok = T;

    fn context(self, msg: impl Into<String>) -> Result<T, MalformedPackage> {
        self.map_err(|err| MalformedPackage {
            msg: msg.into(),
            source: Some(Box::new(err)),
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("index out of bounds (index = {index}, len = {len})")]
pub struct IndexOutOfBounds {
    index: usize,
    len: usize,
}

impl IndexOutOfBounds {
    pub const fn new(index: usize, len: usize) -> Self {
        Self { index, len }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("interned string error")]
pub enum InternedStringError {
    TryFromIntError(#[from] TryFromIntError),
    IndexOutOfBounds(#[from] IndexOutOfBounds),
}

#[derive(Debug, thiserror::Error)]
#[error("interned dotted name error")]
pub enum InternedDottedNameError {
    TryFromIntError(#[from] TryFromIntError),
    IndexOutOfBounds(#[from] IndexOutOfBounds),
    InternedStringError(#[from] InternedStringError),
}
