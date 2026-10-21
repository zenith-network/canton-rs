//! Ledger API has some inconsistency in how they do pagination.
//! Sometimes tokens are optional bytes, sometimes they are possibly empty strings.
//! We represent it here with two strict types and provide a generic type for a page.
//! Paged responses may have additional fields: in this case the page needs to be wrapped into a
//! method-specific return type.

use ledger_api_value::v2::errors::ValueError;

/// Page token in a form of raw bytes.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PageBytesToken {
    token: Vec<u8>,
}

impl PageBytesToken {
    /// Returns `None`, if provided token is empty.
    pub fn new(token: Vec<u8>) -> Option<Self> {
        (!token.is_empty()).then_some(Self { token })
    }

    /// Page token to get the first page ("not set" token).
    pub const fn first() -> Option<Self> {
        None
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.token
    }

    /// Convert into representation expected by Protobuf-s.
    pub fn into_proto(token: Option<Self>) -> Option<Vec<u8>> {
        token.map(Into::into)
    }

    /// Try to convert into token from representation given by Protobuf.
    pub fn try_from_proto(token: Option<Vec<u8>>) -> Result<Option<Self>, ValueError> {
        token
            .map(|token| Self::new(token).ok_or_else(|| ValueError::raw_message("empty token")))
            .transpose()
    }
}

impl From<PageBytesToken> for Vec<u8> {
    fn from(value: PageBytesToken) -> Self {
        value.token
    }
}

/// Page token in a form of a string.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PageStringToken {
    token: String,
}

impl PageStringToken {
    /// Returns `None` if provided token is empty.
    pub fn new(token: String) -> Option<Self> {
        (!token.is_empty()).then_some(Self { token })
    }

    /// Page token to get the first page ("not set" token).
    pub const fn first() -> Option<Self> {
        None
    }

    pub fn as_str(&self) -> &str {
        &self.token
    }

    /// Convert into representation expected by Protobuf-s.
    pub fn into_proto(token: Option<Self>) -> String {
        token.map(Into::into).unwrap_or_default()
    }

    /// Try to convert into token from representation given by Protobuf.
    pub fn try_from_proto(token: String) -> Result<Option<Self>, ValueError> {
        // Although this never fails, we keep fallible function here for further compatibility:
        // at some point we may add an actual format.
        Ok((!token.is_empty()).then_some(Self { token }))
    }
}

impl From<PageStringToken> for String {
    fn from(value: PageStringToken) -> Self {
        value.token
    }
}

/// Generic page.
#[derive(Clone, Debug)]
pub struct Page<Token, Item> {
    /// Items in the page.
    pub items: Vec<Item>,

    /// If this page is the last page, this is set to `None`.
    pub next_page_token: Option<Token>,
}
