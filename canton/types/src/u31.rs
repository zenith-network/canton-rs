use core::{
    fmt,
    iter::{Product, Sum},
    num::{ParseIntError, TryFromIntError},
    ops,
    str::FromStr,
};

/// Unsigned integer less or equal to `i32::MAX`.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct U31(u32);

impl U31 {
    /// The smallest value, zero.
    pub const MIN: Self = Self(0);

    /// The largest value, [`i32::MAX`].
    pub const MAX: Self = Self(i32::MAX as u32);

    /// The number of value bits.
    pub const BITS: u32 = 31;

    /// Create a new `U31`.
    ///
    /// Returns `None` if `value` is greater than `i32::MAX`.
    pub const fn new(value: u32) -> Option<Self> {
        if value <= i32::MAX as u32 {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Return the underlying integer.
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Parses an integer from a string slice with digits in a given base.
    ///
    /// The string is expected to be an optional `+` sign followed by only digits.
    /// Leading and trailing non-digit characters (including whitespace) represent
    /// an error. Underscores (which are accepted in Rust literals) also represent
    /// an error.
    ///
    /// Digits are a subset of these characters, depending on `radix`:
    /// * `0-9`
    /// * `a-z`
    /// * `A-Z`
    ///
    /// # Panics
    ///
    /// This function panics if `radix` is not in the range from 2 to 36.
    ///
    /// # See also
    /// If the string to be parsed is in base 10 (decimal),
    /// [`from_str`] or [`str::parse`] can also be used.
    ///
    /// [`from_str`]: FromStr::from_str
    pub const fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseIntError> {
        let value = match u32::from_str_radix(src, radix) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        match Self::new(value) {
            Some(value) => Ok(value),
            // Parsing as i32 produces the standard positive-overflow error.
            None => match i32::from_str_radix(src, radix) {
                Err(error) => Err(error),
                Ok(_) => unreachable!(),
            },
        }
    }

    /// Checked integer addition. Computes `self + rhs`, returning `None`
    /// if overflow occurred.
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        Self::new(self.0 + rhs.0)
    }

    /// Checked integer subtraction. Computes `self - rhs`, returning
    /// `None` if overflow occurred.
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        match self.0.checked_sub(rhs.0) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Checked integer multiplication. Computes `self * rhs`, returning
    /// `None` if overflow occurred.
    pub const fn checked_mul(self, rhs: Self) -> Option<Self> {
        match self.0.checked_mul(rhs.0) {
            Some(value) => Self::new(value),
            None => None,
        }
    }

    /// Checked integer division. Computes `self / rhs`, returning `None`
    /// if `rhs == 0`.
    pub const fn checked_div(self, rhs: Self) -> Option<Self> {
        match self.0.checked_div(rhs.0) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Saturating integer addition. Computes `self + rhs`, saturating at
    /// the numeric bounds instead of overflowing.
    pub const fn saturating_add(self, rhs: Self) -> Self {
        match self.checked_add(rhs) {
            Some(value) => value,
            None => Self::MAX,
        }
    }

    /// Saturating integer subtraction. Computes `self - rhs`, saturating
    /// at the numeric bounds instead of overflowing.
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    /// Saturating integer multiplication. Computes `self * rhs`,
    /// saturating at the numeric bounds instead of overflowing.
    pub const fn saturating_mul(self, rhs: Self) -> Self {
        match self.checked_mul(rhs) {
            Some(value) => value,
            None => Self::MAX,
        }
    }
}

impl FromStr for U31 {
    type Err = ParseIntError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Self::from_str_radix(src, 10)
    }
}

macro_rules! impl_format {
    ($($trait:ident),+ $(,)?) => {$(
        impl fmt::$trait for U31 {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::$trait::fmt(&self.0, f)
            }
        }
    )+};
}

impl_format!(
    Debug, Display, Binary, Octal, LowerHex, UpperHex, LowerExp, UpperExp
);

macro_rules! impl_from_primitive {
    ($($source:ty),+ $(,)?) => {$(
        impl From<$source> for U31 {
            fn from(value: $source) -> Self {
                Self(u32::from(value))
            }
        }
    )+};
}

impl_from_primitive!(bool, u8, u16, char);

macro_rules! impl_into_primitive {
    ($($target:ty),+ $(,)?) => {$(
        impl From<U31> for $target {
            fn from(value: U31) -> Self {
                value.0 as Self
            }
        }
    )+};
}

// Every target can represent the full U31 range without loss.
impl_into_primitive!(u32, u64, u128, i32, i64, i128, f64);

macro_rules! impl_try_from_primitive {
    ($($source:ty),+ $(,)?) => {$(
        impl TryFrom<$source> for U31 {
            type Error = TryFromIntError;

            fn try_from(value: $source) -> Result<Self, Self::Error> {
                let value = u32::try_from(value)?;
                let _ = i32::try_from(value)?;
                Ok(Self(value))
            }
        }
    )+};
}

impl_try_from_primitive!(u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

macro_rules! impl_try_into_primitive {
    ($($target:ty),+ $(,)?) => {$(
        impl TryFrom<U31> for $target {
            type Error = <Self as TryFrom<u32>>::Error;

            fn try_from(value: U31) -> Result<Self, Self::Error> {
                Self::try_from(value.0)
            }
        }
    )+};
}

impl_try_into_primitive!(u8, u16, i8, i16, usize, isize, char);

macro_rules! impl_arithmetic {
    ($trait:ident, $method:ident, $assign:ident, $assign_method:ident,
     |$lhs:ident, $rhs:ident| $result:expr) => {
        impl ops::$trait for U31 {
            type Output = Self;

            #[inline]
            fn $method(self, rhs: Self) -> Self {
                let $lhs = self.0;
                let $rhs = rhs.0;
                Self($result)
            }
        }

        impl ops::$trait<&U31> for U31 {
            type Output = U31;

            #[inline]
            fn $method(self, rhs: &U31) -> U31 {
                ops::$trait::$method(self, *rhs)
            }
        }

        impl ops::$trait<U31> for &U31 {
            type Output = U31;

            #[inline]
            fn $method(self, rhs: U31) -> U31 {
                ops::$trait::$method(*self, rhs)
            }
        }

        impl ops::$trait<&U31> for &U31 {
            type Output = U31;

            #[inline]
            fn $method(self, rhs: &U31) -> U31 {
                ops::$trait::$method(*self, *rhs)
            }
        }

        impl ops::$assign for U31 {
            #[inline]
            fn $assign_method(&mut self, rhs: Self) {
                *self = ops::$trait::$method(*self, rhs);
            }
        }

        impl ops::$assign<&U31> for U31 {
            #[inline]
            fn $assign_method(&mut self, rhs: &U31) {
                ops::$assign::$assign_method(self, *rhs);
            }
        }
    };
}

// Signed addition and multiplication overflow at i32::MAX, while unsigned
// subtraction underflows at zero. Masking preserves 31-bit results when checks
// are disabled, without adding a release-mode overflow branch.
impl_arithmetic!(Add, add, AddAssign, add_assign, |lhs, rhs| {
    (lhs as i32 + rhs as i32) as u32 & U31::MAX.0
});
impl_arithmetic!(Sub, sub, SubAssign, sub_assign, |lhs, rhs| {
    (lhs - rhs) & U31::MAX.0
});
impl_arithmetic!(Mul, mul, MulAssign, mul_assign, |lhs, rhs| {
    (lhs as i32 * rhs as i32) as u32 & U31::MAX.0
});
impl_arithmetic!(Div, div, DivAssign, div_assign, |lhs, rhs| lhs / rhs);

impl Sum for U31 {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::MIN, |lhs, rhs| lhs + rhs)
    }
}

impl<'a> Sum<&'a U31> for U31 {
    fn sum<I: Iterator<Item = &'a U31>>(iter: I) -> Self {
        iter.copied().sum()
    }
}

impl Product for U31 {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self(1), |lhs, rhs| lhs * rhs)
    }
}

impl<'a> Product<&'a U31> for U31 {
    fn product<I: Iterator<Item = &'a U31>>(iter: I) -> Self {
        iter.copied().product()
    }
}

#[cfg(test)]
mod tests {
    use std::num::{IntErrorKind, ParseIntError};

    use rstest::rstest;

    use super::U31;

    #[rstest]
    #[case::zero(0, Some(U31::MIN))]
    #[case::ordinary(42, Some(U31(42)))]
    #[case::maximum(i32::MAX as u32, Some(U31::MAX))]
    #[case::above_maximum(i32::MAX as u32 + 1, None)]
    #[case::u32_maximum(u32::MAX, None)]
    fn test_new(#[case] input: u32, #[case] expected: Option<U31>) {
        assert_eq!(U31::new(input), expected);
        assert_eq!(U31::try_from(input).ok(), expected);
    }

    #[test]
    fn test_conversions() {
        // In-range integer inputs.
        assert_eq!(U31::try_from(42u32), Ok(U31(42)));
        assert_eq!(U31::try_from(42u64), Ok(U31(42)));
        assert_eq!(U31::try_from(42u128), Ok(U31(42)));
        assert_eq!(U31::try_from(42usize), Ok(U31(42)));
        assert_eq!(U31::try_from(42i8), Ok(U31(42)));
        assert_eq!(U31::try_from(42i16), Ok(U31(42)));
        assert_eq!(U31::try_from(42i32), Ok(U31(42)));
        assert_eq!(U31::try_from(42i64), Ok(U31(42)));
        assert_eq!(U31::try_from(42i128), Ok(U31(42)));
        assert_eq!(U31::try_from(42isize), Ok(U31(42)));
        assert_eq!(U31::try_from(i32::MAX), Ok(U31::MAX));

        // Inputs outside U31's range.
        assert!(U31::try_from(u32::MAX).is_err());
        assert!(U31::try_from(u64::MAX).is_err());
        assert!(U31::try_from(u128::MAX).is_err());
        assert!(U31::try_from(i64::MAX).is_err());
        assert!(U31::try_from(i128::MAX).is_err());
        assert!(U31::try_from(-1i8).is_err());
        assert!(U31::try_from(-1i16).is_err());
        assert!(U31::try_from(-1i32).is_err());
        assert!(U31::try_from(-1i64).is_err());
        assert!(U31::try_from(-1i128).is_err());
        assert!(U31::try_from(-1isize).is_err());

        // Infallible conversions from smaller types.
        assert_eq!(U31::from(false), U31::MIN);
        assert_eq!(U31::from(true), U31(1));
        assert_eq!(U31::from(u8::MAX).get(), 255);
        assert_eq!(U31::from(u16::MAX).get(), 65535);
        assert_eq!(U31::from(char::MAX), U31(0x10ffff));

        // Every U31 fits in these target types.
        assert_eq!(u32::from(U31::MAX), i32::MAX as u32);
        assert_eq!(u64::from(U31::MAX), i32::MAX as u64);
        assert_eq!(u128::from(U31::MAX), i32::MAX as u128);
        assert_eq!(i32::from(U31::MAX), i32::MAX);
        assert_eq!(i64::from(U31::MAX), i32::MAX as i64);
        assert_eq!(i128::from(U31::MAX), i32::MAX as i128);
        assert_eq!(f64::from(U31::MAX), i32::MAX as f64);

        // Narrowing conversions accept the target's maximum and reject the next value.
        assert_eq!(u8::try_from(U31(255)), Ok(u8::MAX));
        assert!(u8::try_from(U31(256)).is_err());
        assert_eq!(u16::try_from(U31(65535)), Ok(u16::MAX));
        assert!(u16::try_from(U31(65536)).is_err());
        assert_eq!(i8::try_from(U31(127)), Ok(i8::MAX));
        assert!(i8::try_from(U31(128)).is_err());
        assert_eq!(i16::try_from(U31(32767)), Ok(i16::MAX));
        assert!(i16::try_from(U31(32768)).is_err());
        assert_eq!(usize::try_from(U31::MAX), usize::try_from(i32::MAX as u32));
        assert_eq!(isize::try_from(U31::MAX), isize::try_from(i32::MAX as u32));

        // Characters and decimal strings.
        assert_eq!(char::try_from(U31(0x10ffff)), Ok(char::MAX));
        assert!(char::try_from(U31(0xd800)).is_err());
        assert!(char::try_from(U31::MAX).is_err());
        assert_eq!("+00042".parse::<U31>(), Ok(U31(42)));
        assert!("2147483648".parse::<U31>().is_err());
    }

    #[rstest]
    #[case::zero("0", 10, Ok(U31::MIN))]
    #[case::plus_and_leading_zeros("+00042", 10, Ok(U31(42)))]
    #[case::maximum_decimal("2147483647", 10, Ok(U31::MAX))]
    #[case::maximum_hex("7fffffff", 16, Ok(U31::MAX))]
    #[case::binary("101010", 2, Ok(U31(42)))]
    #[case::empty("", 10, Err(IntErrorKind::Empty))]
    #[case::negative_zero("-0", 10, Err(IntErrorKind::InvalidDigit))]
    #[case::negative("-1", 10, Err(IntErrorKind::InvalidDigit))]
    #[case::leading_space(" 42", 10, Err(IntErrorKind::InvalidDigit))]
    #[case::trailing_space("42 ", 10, Err(IntErrorKind::InvalidDigit))]
    #[case::invalid_digit("bad", 10, Err(IntErrorKind::InvalidDigit))]
    #[case::above_maximum("2147483648", 10, Err(IntErrorKind::PosOverflow))]
    #[case::above_u32_maximum("4294967296", 10, Err(IntErrorKind::PosOverflow))]
    #[case::above_maximum_hex("80000000", 16, Err(IntErrorKind::PosOverflow))]
    fn test_parse(
        #[case] input: &str,
        #[case] radix: u32,
        #[case] expected: Result<U31, IntErrorKind>,
    ) {
        let result = U31::from_str_radix(input, radix).map_err(|error| *error.kind());
        assert_eq!(result, expected);
    }

    #[test]
    fn test_value_traits_and_formatting() {
        const VALUE: U31 = U31::new(42).unwrap();
        const RAW: u32 = VALUE.get();
        const PARSED: Result<U31, ParseIntError> = U31::from_str_radix("42", 10);
        assert_eq!(RAW, 42);
        assert_eq!(PARSED, Ok(VALUE));
        assert_eq!(U31::BITS, 31);
        assert_eq!(U31::default(), U31::MIN);
        assert!(U31::MIN < VALUE && VALUE < U31::MAX);

        assert_eq!(format!("{VALUE:?}"), "42");
        assert_eq!(format!("{VALUE}"), "42");
        assert_eq!(format!("{VALUE:+06}"), "+00042");
        assert_eq!(format!("{VALUE:#b}"), "0b101010");
        assert_eq!(format!("{VALUE:#o}"), "0o52");
        assert_eq!(format!("{VALUE:#08x}"), "0x00002a");
        assert_eq!(format!("{VALUE:X}"), "2A");
        assert_eq!(format!("{VALUE:e}"), "4.2e1");
        assert_eq!(format!("{VALUE:E}"), "4.2E1");
    }

    #[test]
    #[expect(
        clippy::op_ref,
        reason = "exercise the borrowed operator implementations"
    )]
    fn test_operators_and_assignments() {
        let lhs = U31(12);
        let rhs = U31(5);
        assert_eq!(lhs + rhs, U31(17));
        assert_eq!(lhs + &rhs, U31(17));
        assert_eq!(&lhs + rhs, U31(17));
        assert_eq!(&lhs + &rhs, U31(17));
        assert_eq!(lhs - rhs, U31(7));
        assert_eq!(lhs - &rhs, U31(7));
        assert_eq!(&lhs - rhs, U31(7));
        assert_eq!(&lhs - &rhs, U31(7));
        assert_eq!(lhs * rhs, U31(60));
        assert_eq!(lhs * &rhs, U31(60));
        assert_eq!(&lhs * rhs, U31(60));
        assert_eq!(&lhs * &rhs, U31(60));
        assert_eq!(lhs / rhs, U31(2));
        assert_eq!(lhs / &rhs, U31(2));
        assert_eq!(&lhs / rhs, U31(2));
        assert_eq!(&lhs / &rhs, U31(2));

        let mut value = lhs;
        value += rhs;
        assert_eq!(value, U31(17));
        value -= rhs;
        assert_eq!(value, U31(12));
        value *= rhs;
        assert_eq!(value, U31(60));
        value /= rhs;
        assert_eq!(value, U31(12));

        value += &rhs;
        assert_eq!(value, U31(17));
        value -= &rhs;
        assert_eq!(value, U31(12));
        value *= &rhs;
        assert_eq!(value, U31(60));
        value /= &rhs;
        assert_eq!(value, U31(12));
    }

    #[test]
    fn test_iterators() {
        let values = [U31(2), U31(3), U31(4)];
        assert_eq!(values.into_iter().sum::<U31>(), U31(9));
        assert_eq!(values.iter().sum::<U31>(), U31(9));
        assert_eq!(values.into_iter().product::<U31>(), U31(24));
        assert_eq!(values.iter().product::<U31>(), U31(24));
        let empty: [U31; 0] = [];
        assert_eq!(empty.into_iter().sum::<U31>(), U31::MIN);
        assert_eq!(empty.iter().sum::<U31>(), U31::MIN);
        assert_eq!(empty.into_iter().product::<U31>(), U31(1));
        assert_eq!(empty.iter().product::<U31>(), U31(1));
    }

    #[rstest]
    #[case::zero(U31::MIN, U31::MIN, Some(U31::MIN), U31::MIN)]
    #[case::ordinary(U31(7), U31(3), Some(U31(10)), U31(10))]
    #[case::maximum(U31::MAX, U31::MIN, Some(U31::MAX), U31::MAX)]
    #[case::overflow(U31::MAX, U31(1), None, U31::MAX)]
    fn test_add(
        #[case] lhs: U31,
        #[case] rhs: U31,
        #[case] checked: Option<U31>,
        #[case] saturated: U31,
    ) {
        assert_eq!(lhs.checked_add(rhs), checked);
        assert_eq!(lhs.saturating_add(rhs), saturated);
    }

    #[rstest]
    #[case::zero(U31::MIN, U31::MIN, Some(U31::MIN), U31::MIN)]
    #[case::ordinary(U31(7), U31(3), Some(U31(4)), U31(4))]
    #[case::underflow(U31::MIN, U31(1), None, U31::MIN)]
    #[case::equal_maximums(U31::MAX, U31::MAX, Some(U31::MIN), U31::MIN)]
    fn test_sub(
        #[case] lhs: U31,
        #[case] rhs: U31,
        #[case] checked: Option<U31>,
        #[case] saturated: U31,
    ) {
        assert_eq!(lhs.checked_sub(rhs), checked);
        assert_eq!(lhs.saturating_sub(rhs), saturated);
    }

    #[rstest]
    #[case::zero(U31::MAX, U31::MIN, Some(U31::MIN), U31::MIN)]
    #[case::ordinary(U31(7), U31(3), Some(U31(21)), U31(21))]
    #[case::maximum(U31::MAX, U31(1), Some(U31::MAX), U31::MAX)]
    #[case::overflow(U31::MAX, U31(2), None, U31::MAX)]
    #[case::u32_overflow(U31(65536), U31(65536), None, U31::MAX)]
    fn test_mul(
        #[case] lhs: U31,
        #[case] rhs: U31,
        #[case] checked: Option<U31>,
        #[case] saturated: U31,
    ) {
        assert_eq!(lhs.checked_mul(rhs), checked);
        assert_eq!(lhs.saturating_mul(rhs), saturated);
    }

    #[rstest]
    #[case::ordinary(U31(7), U31(3), Some(U31(2)))]
    #[case::maximum(U31::MAX, U31(1), Some(U31::MAX))]
    #[case::zero(U31::MIN, U31(1), Some(U31::MIN))]
    #[case::zero_divisor(U31(1), U31::MIN, None)]
    fn test_division(#[case] lhs: U31, #[case] rhs: U31, #[case] quotient: Option<U31>) {
        assert_eq!(lhs.checked_div(rhs), quotient);
    }

    #[test]
    #[should_panic(expected = "divide by zero")]
    fn test_zero_divisor() {
        let _ = U31(1) / U31::MIN;
    }
}
