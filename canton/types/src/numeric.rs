use std::{error::Error, fmt, str::FromStr};

use bigdecimal::BigDecimal;

const MAX_PRECISION: u64 = 38;
const MAX_SCALE: i64 = 37;

// FIXME: This implementation needs a strong rework. Known issues:
//         - scale is truncated by bigdecimal
//         - display it falling back to 1E-7 notation

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
        let normalized = big.normalized();
        let scale = normalized.fractional_digit_count().max(0);
        if scale > MAX_SCALE {
            return Err(NumericError {
                kind: ErrorKind::ScaleOutOfBounds { scale },
            });
        }

        let big = normalized.with_scale(scale);
        let precision = big.digits();
        if precision > MAX_PRECISION {
            return Err(NumericError {
                kind: ErrorKind::PrecisionOutOfBounds { precision },
            });
        }

        Ok(Self { big })
    }

    pub fn parse(input: impl AsRef<str>) -> Result<Self, NumericError> {
        let input = input.as_ref();
        if !is_valid_numeric_input(input) {
            return Err(NumericError {
                kind: ErrorKind::InvalidFormat,
            });
        }

        let big = BigDecimal::from_str(input).map_err(|error| NumericError {
            kind: ErrorKind::ParseBigDecimal { error },
        })?;
        Self::from_big_decimal(big)
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

fn is_valid_numeric_input(input: &str) -> bool {
    let bytes = input.as_bytes();
    let mut idx = match bytes.first() {
        Some(b'+' | b'-') => 1,
        Some(_) => 0,
        None => return false,
    };
    if idx == bytes.len() {
        return false;
    }

    let int_start = idx;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    let int_digits = idx - int_start;
    if int_digits == 0 || int_digits > MAX_PRECISION as usize {
        return false;
    }

    if idx == bytes.len() {
        return true;
    }
    if bytes[idx] != b'.' {
        return false;
    }

    idx += 1;
    let frac_start = idx;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    let frac_digits = idx - frac_start;

    idx == bytes.len() && frac_digits <= MAX_SCALE as usize
}

#[derive(Debug)]
pub struct NumericError {
    kind: ErrorKind,
}

impl fmt::Display for NumericError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid numeric: ")?;
        self.kind.fmt(f)
    }
}

impl Error for NumericError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let ErrorKind::ParseBigDecimal { error } = &self.kind {
            Some(error)
        } else {
            None
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("numeric does not match `[+-]?\\d{{1,38}}(\\.\\d{{0,37}})?`")]
    InvalidFormat,

    #[error("numeric scale is out of bounds (got {scale}, max: {MAX_SCALE})")]
    ScaleOutOfBounds { scale: i64 },

    #[error(
        "numeric precision is out of bounds (got {precision} significant digits, max: {MAX_PRECISION})"
    )]
    PrecisionOutOfBounds { precision: u64 },

    #[error("failed to parse decimal")]
    ParseBigDecimal {
        #[source]
        error: bigdecimal::ParseBigDecimalError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn parse_normalizes_equivalent_inputs() {
        let lhs = Numeric::parse("+00042.1000").unwrap();
        let rhs = Numeric::parse("42.1").unwrap();

        assert_eq!(lhs, rhs);
    }

    #[rstest]
    #[case("1E10")]
    #[case("0.00000000000000000000000000000000000001")]
    fn parse_rejects_invalid_format(#[case] input: &str) {
        let err = Numeric::parse(input).unwrap_err();

        assert!(matches!(err.kind, ErrorKind::InvalidFormat));
    }

    #[test]
    fn parse_rejects_precision_overflow() {
        let err = Numeric::parse("99999999999999999999999999999999999999.1").unwrap_err();

        assert!(matches!(
            err.kind,
            ErrorKind::PrecisionOutOfBounds { precision: 39 }
        ));
    }
}
