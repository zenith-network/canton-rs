use core::{
    fmt,
    num::{NonZeroU32, ParseIntError, TryFromIntError},
    str::FromStr,
};

use crate::U31;

// If std::num::ZeroablePrimitive will become public and stable we could implement it for U31,
// so that we could use std::num::NonZero<U31>. But now we have to do this as a separate
// concrete type. The name matches names like std::num::NonZeroU32.

/// Non-zero unsigned integer less or equal to [`i32::MAX`].
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonZeroU31(NonZeroU32);

impl NonZeroU31 {
    /// The smallest value, one.
    pub const MIN: Self = Self(NonZeroU32::MIN);

    /// The largest value, [`i32::MAX`].
    pub const MAX: Self = Self(NonZeroU32::new(U31::MAX.get()).unwrap());

    /// The number of value bits.
    pub const BITS: u32 = U31::BITS;

    /// Create a new `NonZeroU31`.
    ///
    /// Returns `None` if `value` is zero or greater than `i32::MAX`.
    pub const fn new(value: u32) -> Option<Self> {
        if value <= U31::MAX.get() {
            match NonZeroU32::new(value) {
                Some(value) => Some(Self(value)),
                None => None,
            }
        } else {
            None
        }
    }

    /// Return the underlying integer.
    pub const fn get(self) -> u32 {
        self.0.get()
    }

    /// Parses a non-zero integer from a string slice with digits in a given base.
    ///
    /// The string is expected to be an optional `+` sign followed by only digits.
    /// Leading and trailing non-digit characters (including whitespace) represent
    /// an error. Underscores (which are accepted in Rust literals) also represent
    /// an error. Zero and values greater than `i32::MAX` also represent an error.
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
        let value = match NonZeroU32::from_str_radix(src, radix) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        if value.get() <= U31::MAX.get() {
            Ok(Self(value))
        } else {
            // Parsing as i32 produces the standard positive-overflow error.
            match i32::from_str_radix(src, radix) {
                Err(error) => Err(error),
                Ok(_) => unreachable!(),
            }
        }
    }

    /// Checked integer addition. Computes `self + rhs`, returning `None`
    /// if overflow occurred.
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        Self::new(self.get() + rhs.get())
    }

    /// Checked integer subtraction. Computes `self - rhs`, returning
    /// `None` if the result is zero or underflow occurred.
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        match self.get().checked_sub(rhs.get()) {
            Some(value) => Self::new(value),
            None => None,
        }
    }

    /// Checked integer multiplication. Computes `self * rhs`, returning
    /// `None` if overflow occurred.
    pub const fn checked_mul(self, rhs: Self) -> Option<Self> {
        match self.0.checked_mul(rhs.0) {
            Some(value) => Self::new(value.get()),
            None => None,
        }
    }

    /// Checked integer division. Computes `self / rhs`, returning `None`
    /// if the quotient is zero.
    pub const fn checked_div(self, rhs: Self) -> Option<Self> {
        Self::new(self.get() / rhs.get())
    }

    /// Saturating integer addition. Computes `self + rhs`, saturating at
    /// [`Self::MAX`] instead of overflowing.
    pub const fn saturating_add(self, rhs: Self) -> Self {
        match self.checked_add(rhs) {
            Some(value) => value,
            None => Self::MAX,
        }
    }

    /// Saturating integer subtraction. Computes `self - rhs`, saturating
    /// at [`Self::MIN`] if the result would be less than one.
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        match self.checked_sub(rhs) {
            Some(value) => value,
            None => Self::MIN,
        }
    }

    /// Saturating integer multiplication. Computes `self * rhs`,
    /// saturating at [`Self::MAX`] instead of overflowing.
    pub const fn saturating_mul(self, rhs: Self) -> Self {
        match self.checked_mul(rhs) {
            Some(value) => value,
            None => Self::MAX,
        }
    }
}

impl FromStr for NonZeroU31 {
    type Err = ParseIntError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Self::from_str_radix(src, 10)
    }
}

macro_rules! impl_format {
    ($($trait:ident),+ $(,)?) => {$(
        impl fmt::$trait for NonZeroU31 {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::$trait::fmt(&self.0, f)
            }
        }
    )+};
}

impl_format!(
    Debug, Display, Binary, Octal, LowerHex, UpperHex, LowerExp, UpperExp
);

impl TryFrom<U31> for NonZeroU31 {
    type Error = TryFromIntError;

    fn try_from(value: U31) -> Result<Self, Self::Error> {
        Ok(Self(NonZeroU32::try_from(value.get())?))
    }
}

impl From<NonZeroU31> for U31 {
    fn from(value: NonZeroU31) -> Self {
        Self::new(value.get()).unwrap()
    }
}

impl TryFrom<NonZeroU32> for NonZeroU31 {
    type Error = TryFromIntError;

    fn try_from(value: NonZeroU32) -> Result<Self, Self::Error> {
        let _ = i32::try_from(value.get())?;
        Ok(Self(value))
    }
}

impl From<NonZeroU31> for NonZeroU32 {
    fn from(value: NonZeroU31) -> Self {
        value.0
    }
}

macro_rules! impl_try_from_primitive {
    ($($source:ty),+ $(,)?) => {$(
        impl TryFrom<$source> for NonZeroU31 {
            type Error = TryFromIntError;

            fn try_from(value: $source) -> Result<Self, Self::Error> {
                Self::try_from(U31::try_from(value)?)
            }
        }
    )+};
}

impl_try_from_primitive!(
    bool, u8, u16, char, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

macro_rules! impl_into_primitive {
    ($($target:ty),+ $(,)?) => {$(
        impl From<NonZeroU31> for $target {
            fn from(value: NonZeroU31) -> Self {
                value.get() as Self
            }
        }
    )+};
}

// Every target can represent the full NonZeroU31 range without loss.
impl_into_primitive!(u32, u64, u128, i32, i64, i128, f64);

macro_rules! impl_try_into_primitive {
    ($($target:ty),+ $(,)?) => {$(
        impl TryFrom<NonZeroU31> for $target {
            type Error = <Self as TryFrom<u32>>::Error;

            fn try_from(value: NonZeroU31) -> Result<Self, Self::Error> {
                Self::try_from(value.get())
            }
        }
    )+};
}

impl_try_into_primitive!(u8, u16, i8, i16, usize, isize, char);

#[cfg(test)]
mod tests {
    use std::{
        mem::{align_of, size_of},
        num::{IntErrorKind, NonZeroU32, ParseIntError},
    };

    use rstest::rstest;

    use super::NonZeroU31;
    use crate::U31;

    #[rstest]
    #[case::zero(0, None)]
    #[case::minimum(1, Some(1))]
    #[case::ordinary(42, Some(42))]
    #[case::maximum(i32::MAX as u32, Some(i32::MAX as u32))]
    #[case::above_maximum(i32::MAX as u32 + 1, None)]
    #[case::u32_maximum(u32::MAX, None)]
    fn test_new(#[case] input: u32, #[case] expected: Option<u32>) {
        assert_eq!(NonZeroU31::new(input).map(NonZeroU31::get), expected);
        assert_eq!(
            NonZeroU31::try_from(input).ok().map(NonZeroU31::get),
            expected
        );
    }

    #[test]
    fn test_conversions() {
        let value = NonZeroU31::new(42).unwrap();

        // In-range integer inputs.
        assert_eq!(NonZeroU31::try_from(42u8), Ok(value));
        assert_eq!(NonZeroU31::try_from(42u16), Ok(value));
        assert_eq!(NonZeroU31::try_from(42u32), Ok(value));
        assert_eq!(NonZeroU31::try_from(42u64), Ok(value));
        assert_eq!(NonZeroU31::try_from(42u128), Ok(value));
        assert_eq!(NonZeroU31::try_from(42usize), Ok(value));
        assert_eq!(NonZeroU31::try_from(42i8), Ok(value));
        assert_eq!(NonZeroU31::try_from(42i16), Ok(value));
        assert_eq!(NonZeroU31::try_from(42i32), Ok(value));
        assert_eq!(NonZeroU31::try_from(42i64), Ok(value));
        assert_eq!(NonZeroU31::try_from(42i128), Ok(value));
        assert_eq!(NonZeroU31::try_from(42isize), Ok(value));
        assert_eq!(NonZeroU31::try_from(i32::MAX), Ok(NonZeroU31::MAX));

        // Zero is rejected for every primitive source type.
        assert!(NonZeroU31::try_from(0u8).is_err());
        assert!(NonZeroU31::try_from(0u16).is_err());
        assert!(NonZeroU31::try_from(0u32).is_err());
        assert!(NonZeroU31::try_from(0u64).is_err());
        assert!(NonZeroU31::try_from(0u128).is_err());
        assert!(NonZeroU31::try_from(0usize).is_err());
        assert!(NonZeroU31::try_from(0i8).is_err());
        assert!(NonZeroU31::try_from(0i16).is_err());
        assert!(NonZeroU31::try_from(0i32).is_err());
        assert!(NonZeroU31::try_from(0i64).is_err());
        assert!(NonZeroU31::try_from(0i128).is_err());
        assert!(NonZeroU31::try_from(0isize).is_err());
        assert!(NonZeroU31::try_from(false).is_err());
        assert!(NonZeroU31::try_from('\0').is_err());

        // Inputs outside NonZeroU31's range.
        assert!(NonZeroU31::try_from(u32::MAX).is_err());
        assert!(NonZeroU31::try_from(u64::MAX).is_err());
        assert!(NonZeroU31::try_from(u128::MAX).is_err());
        assert!(NonZeroU31::try_from(i64::MAX).is_err());
        assert!(NonZeroU31::try_from(i128::MAX).is_err());
        assert!(NonZeroU31::try_from(-1i8).is_err());
        assert!(NonZeroU31::try_from(-1i16).is_err());
        assert!(NonZeroU31::try_from(-1i32).is_err());
        assert!(NonZeroU31::try_from(-1i64).is_err());
        assert!(NonZeroU31::try_from(-1i128).is_err());
        assert!(NonZeroU31::try_from(-1isize).is_err());

        // Boolean and character inputs.
        assert_eq!(NonZeroU31::try_from(true), Ok(NonZeroU31::MIN));
        assert_eq!(NonZeroU31::try_from(char::MAX).unwrap().get(), 0x10ffff);

        // Every NonZeroU31 fits in these target types.
        assert_eq!(u32::from(NonZeroU31::MAX), i32::MAX as u32);
        assert_eq!(u64::from(NonZeroU31::MAX), i32::MAX as u64);
        assert_eq!(u128::from(NonZeroU31::MAX), i32::MAX as u128);
        assert_eq!(i32::from(NonZeroU31::MAX), i32::MAX);
        assert_eq!(i64::from(NonZeroU31::MAX), i32::MAX as i64);
        assert_eq!(i128::from(NonZeroU31::MAX), i32::MAX as i128);
        assert_eq!(f64::from(NonZeroU31::MAX), i32::MAX as f64);

        // Narrowing conversions accept the target's maximum and reject the next value.
        assert_eq!(u8::try_from(NonZeroU31::new(255).unwrap()), Ok(u8::MAX));
        assert!(u8::try_from(NonZeroU31::new(256).unwrap()).is_err());
        assert_eq!(u16::try_from(NonZeroU31::new(65535).unwrap()), Ok(u16::MAX));
        assert!(u16::try_from(NonZeroU31::new(65536).unwrap()).is_err());
        assert_eq!(i8::try_from(NonZeroU31::new(127).unwrap()), Ok(i8::MAX));
        assert!(i8::try_from(NonZeroU31::new(128).unwrap()).is_err());
        assert_eq!(i16::try_from(NonZeroU31::new(32767).unwrap()), Ok(i16::MAX));
        assert!(i16::try_from(NonZeroU31::new(32768).unwrap()).is_err());
        assert_eq!(
            usize::try_from(NonZeroU31::MAX),
            usize::try_from(i32::MAX as u32)
        );
        assert_eq!(
            isize::try_from(NonZeroU31::MAX),
            isize::try_from(i32::MAX as u32)
        );

        // Characters and decimal strings.
        assert_eq!(
            char::try_from(NonZeroU31::new(0x10ffff).unwrap()),
            Ok(char::MAX)
        );
        assert!(char::try_from(NonZeroU31::new(0xd800).unwrap()).is_err());
        assert!(char::try_from(NonZeroU31::MAX).is_err());
        assert_eq!("+00042".parse::<NonZeroU31>(), Ok(value));
        assert!("0".parse::<NonZeroU31>().is_err());
        assert!("2147483648".parse::<NonZeroU31>().is_err());
    }

    #[rstest]
    #[case::minimum(NonZeroU31::MIN)]
    #[case::ordinary(NonZeroU31::new(42).unwrap())]
    #[case::maximum(NonZeroU31::MAX)]
    fn test_u31_and_non_zero_u32_conversions(#[case] value: NonZeroU31) {
        let u31 = U31::from(value);
        let non_zero_u32 = NonZeroU32::from(value);
        assert_eq!(u31.get(), value.get());
        assert_eq!(non_zero_u32.get(), value.get());
        assert_eq!(NonZeroU31::try_from(u31), Ok(value));
        assert_eq!(NonZeroU31::try_from(non_zero_u32), Ok(value));
    }

    #[test]
    fn test_invalid_u31_and_non_zero_u32_conversions() {
        assert_eq!(
            NonZeroU31::try_from(U31::MIN).unwrap_err(),
            NonZeroU32::try_from(0u32).unwrap_err()
        );
        for raw in [i32::MAX as u32 + 1, u32::MAX] {
            assert_eq!(
                NonZeroU31::try_from(NonZeroU32::new(raw).unwrap()).unwrap_err(),
                i32::try_from(raw).unwrap_err()
            );
        }
    }

    #[rstest]
    #[case::minimum("1", 10, Ok(1))]
    #[case::plus_and_leading_zeros("+00042", 10, Ok(42))]
    #[case::maximum_decimal("2147483647", 10, Ok(i32::MAX as u32))]
    #[case::maximum_hex("7fffffff", 16, Ok(i32::MAX as u32))]
    #[case::binary("101010", 2, Ok(42))]
    #[case::maximum_radix("z", 36, Ok(35))]
    #[case::zero("0", 10, Err(IntErrorKind::Zero))]
    #[case::plus_and_zero_digits("+000", 10, Err(IntErrorKind::Zero))]
    #[case::zero_hex("00", 16, Err(IntErrorKind::Zero))]
    #[case::empty("", 10, Err(IntErrorKind::Empty))]
    #[case::sign_only("+", 10, Err(IntErrorKind::InvalidDigit))]
    #[case::negative_zero("-0", 10, Err(IntErrorKind::InvalidDigit))]
    #[case::negative("-1", 10, Err(IntErrorKind::InvalidDigit))]
    #[case::leading_space(" 42", 10, Err(IntErrorKind::InvalidDigit))]
    #[case::trailing_space("42 ", 10, Err(IntErrorKind::InvalidDigit))]
    #[case::underscore("4_2", 10, Err(IntErrorKind::InvalidDigit))]
    #[case::invalid_digit("bad", 10, Err(IntErrorKind::InvalidDigit))]
    #[case::above_maximum("2147483648", 10, Err(IntErrorKind::PosOverflow))]
    #[case::u32_maximum("4294967295", 10, Err(IntErrorKind::PosOverflow))]
    #[case::above_u32_maximum("4294967296", 10, Err(IntErrorKind::PosOverflow))]
    #[case::above_maximum_hex("80000000", 16, Err(IntErrorKind::PosOverflow))]
    fn test_parse(
        #[case] input: &str,
        #[case] radix: u32,
        #[case] expected: Result<u32, IntErrorKind>,
    ) {
        let result = NonZeroU31::from_str_radix(input, radix)
            .map(NonZeroU31::get)
            .map_err(|error| *error.kind());
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case::zero(0)]
    #[case::below_minimum(1)]
    #[case::above_maximum(37)]
    #[should_panic(expected = "radix must lie in the range")]
    fn test_invalid_radix(#[case] radix: u32) {
        let _ = NonZeroU31::from_str_radix("1", radix);
    }

    #[test]
    fn test_value_traits_and_formatting() {
        const VALUE: NonZeroU31 = NonZeroU31::new(42).unwrap();
        const RAW: u32 = VALUE.get();
        const PARSED: Result<NonZeroU31, ParseIntError> = NonZeroU31::from_str_radix("42", 10);
        assert_eq!(RAW, 42);
        assert_eq!(PARSED, Ok(VALUE));
        assert_eq!(NonZeroU31::MIN.get(), 1);
        assert_eq!(NonZeroU31::MAX.get(), i32::MAX as u32);
        assert_eq!(NonZeroU31::BITS, 31);
        assert!(NonZeroU31::MIN < VALUE && VALUE < NonZeroU31::MAX);

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
    fn test_layout() {
        assert_eq!(size_of::<NonZeroU31>(), size_of::<u32>());
        assert_eq!(align_of::<NonZeroU31>(), align_of::<u32>());
        assert_eq!(size_of::<Option<NonZeroU31>>(), size_of::<u32>());
        assert_eq!(align_of::<Option<NonZeroU31>>(), align_of::<u32>());
    }

    #[rstest]
    #[case::minimum(1, 1, Some(2), 2)]
    #[case::ordinary(7, 3, Some(10), 10)]
    #[case::maximum(i32::MAX as u32 - 1, 1, Some(i32::MAX as u32), i32::MAX as u32)]
    #[case::overflow(i32::MAX as u32, 1, None, i32::MAX as u32)]
    #[case::both_maximum(i32::MAX as u32, i32::MAX as u32, None, i32::MAX as u32)]
    fn test_add(
        #[case] lhs: u32,
        #[case] rhs: u32,
        #[case] checked: Option<u32>,
        #[case] saturated: u32,
    ) {
        let lhs = NonZeroU31::new(lhs).unwrap();
        let rhs = NonZeroU31::new(rhs).unwrap();
        assert_eq!(lhs.checked_add(rhs).map(NonZeroU31::get), checked);
        assert_eq!(lhs.saturating_add(rhs).get(), saturated);
    }

    #[rstest]
    #[case::ordinary(7, 3, Some(4), 4)]
    #[case::minimum_result(2, 1, Some(1), 1)]
    #[case::zero_result(1, 1, None, 1)]
    #[case::underflow(1, 2, None, 1)]
    #[case::equal_maximums(i32::MAX as u32, i32::MAX as u32, None, 1)]
    fn test_sub(
        #[case] lhs: u32,
        #[case] rhs: u32,
        #[case] checked: Option<u32>,
        #[case] saturated: u32,
    ) {
        let lhs = NonZeroU31::new(lhs).unwrap();
        let rhs = NonZeroU31::new(rhs).unwrap();
        assert_eq!(lhs.checked_sub(rhs).map(NonZeroU31::get), checked);
        assert_eq!(lhs.saturating_sub(rhs).get(), saturated);
    }

    #[rstest]
    #[case::minimum(1, 1, Some(1), 1)]
    #[case::ordinary(7, 3, Some(21), 21)]
    #[case::maximum(i32::MAX as u32, 1, Some(i32::MAX as u32), i32::MAX as u32)]
    #[case::overflow(i32::MAX as u32, 2, None, i32::MAX as u32)]
    #[case::u32_overflow(65536, 65536, None, i32::MAX as u32)]
    fn test_mul(
        #[case] lhs: u32,
        #[case] rhs: u32,
        #[case] checked: Option<u32>,
        #[case] saturated: u32,
    ) {
        let lhs = NonZeroU31::new(lhs).unwrap();
        let rhs = NonZeroU31::new(rhs).unwrap();
        assert_eq!(lhs.checked_mul(rhs).map(NonZeroU31::get), checked);
        assert_eq!(lhs.saturating_mul(rhs).get(), saturated);
    }

    #[rstest]
    #[case::ordinary(7, 3, Some(2))]
    #[case::maximum(i32::MAX as u32, 1, Some(i32::MAX as u32))]
    #[case::minimum(1, 1, Some(1))]
    #[case::equal_maximums(i32::MAX as u32, i32::MAX as u32, Some(1))]
    #[case::zero_quotient(1, 2, None)]
    fn test_division(#[case] lhs: u32, #[case] rhs: u32, #[case] quotient: Option<u32>) {
        let lhs = NonZeroU31::new(lhs).unwrap();
        let rhs = NonZeroU31::new(rhs).unwrap();
        assert_eq!(lhs.checked_div(rhs).map(NonZeroU31::get), quotient);
    }

    #[test]
    fn test_const_arithmetic() {
        const VALUE: NonZeroU31 = NonZeroU31::new(42).unwrap();
        const ADDED: Option<NonZeroU31> = VALUE.checked_add(VALUE);
        const SUBTRACTED: Option<NonZeroU31> = VALUE.checked_sub(VALUE);
        const MULTIPLIED: Option<NonZeroU31> = NonZeroU31::MAX.checked_mul(VALUE);
        const DIVIDED: Option<NonZeroU31> = NonZeroU31::MIN.checked_div(VALUE);
        const SATURATED_ADD: NonZeroU31 = NonZeroU31::MAX.saturating_add(VALUE);
        const SATURATED_SUB: NonZeroU31 = VALUE.saturating_sub(VALUE);
        const SATURATED_MUL: NonZeroU31 = NonZeroU31::MAX.saturating_mul(VALUE);
        assert_eq!(ADDED.map(NonZeroU31::get), Some(84));
        assert_eq!(SUBTRACTED, None);
        assert_eq!(MULTIPLIED, None);
        assert_eq!(DIVIDED, None);
        assert_eq!(SATURATED_ADD, NonZeroU31::MAX);
        assert_eq!(SATURATED_SUB, NonZeroU31::MIN);
        assert_eq!(SATURATED_MUL, NonZeroU31::MAX);
    }
}
