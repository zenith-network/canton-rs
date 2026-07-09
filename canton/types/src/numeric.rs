use std::{fmt, str::FromStr};

use bigdecimal::BigDecimal;

/// A Numeric, that is a decimal value with precision 38 (at most 38 significant digits) and a scale
/// between 0 and 37 (significant digits on the right of the decimal point). The field has to match
/// the regex
///
/// ```plaintext
/// [+-]?\d{1,38}(.\d{0,37})?
/// ```
///
/// and should be representable by a Numeric without loss of precision.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Numeric {
    big: BigDecimal,
}

impl Numeric {
    pub fn from_big_decimal(big: BigDecimal) -> Result<Self, NumericError> {
        let _ = big;
        todo!()
    }

    pub fn parse(input: impl AsRef<str>) -> Result<Self, NumericError> {
        let _ = input;
        todo!()
    }
}

impl From<u64> for Numeric {
    fn from(value: u64) -> Self {
        Self { big: value.into() }
    }
}

impl fmt::Display for Numeric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.big.fmt(f)
    }
}

impl FromStr for Numeric {
    type Err = NumericError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid numeric")]
pub struct NumericError;
