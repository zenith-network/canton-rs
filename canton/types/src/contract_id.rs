use std::{fmt, hash::Hash, marker::PhantomData};

use crate::{AnyTemplate, LedgerString, errors::LedgerStringError};

/// Contract ID
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContractId<T = AnyTemplate> {
    value: LedgerString,
    phantom_data: PhantomData<T>,
}

impl<T> ContractId<T> {
    pub fn new(value: String) -> Result<Self, ContractIdError> {
        Ok(Self {
            value: LedgerString::new(value)?,
            phantom_data: PhantomData,
        })
    }

    pub fn from_ledger_string(value: LedgerString) -> Self {
        Self {
            value,
            phantom_data: PhantomData,
        }
    }

    /// Erase the type of the template from self
    pub fn into_any(self) -> ContractId<AnyTemplate> {
        ContractId {
            value: self.value,
            phantom_data: PhantomData,
        }
    }

    pub fn from_any(any: ContractId<AnyTemplate>) -> Self {
        Self {
            value: any.value,
            phantom_data: PhantomData,
        }
    }
}

impl ContractId<AnyTemplate> {
    pub fn into_typed<T>(self) -> ContractId<T> {
        ContractId::from_any(self)
    }
}

impl<T> fmt::Debug for ContractId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<T> fmt::Display for ContractId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<T> From<ContractId<T>> for String {
    fn from(cid: ContractId<T>) -> Self {
        cid.value.into()
    }
}

impl<T> AsRef<str> for ContractId<T> {
    fn as_ref(&self) -> &str {
        self.value.as_ref()
    }
}

impl<T> PartialEq<str> for ContractId<T> {
    fn eq(&self, other: &str) -> bool {
        self.value == other
    }
}

impl<T> PartialEq<String> for ContractId<T> {
    fn eq(&self, other: &String) -> bool {
        &self.value == other
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid contract ID: {0}")]
pub struct ContractIdError(#[from] LedgerStringError);
