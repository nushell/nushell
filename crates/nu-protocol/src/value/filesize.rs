use crate::{FromValue, IntoValue, ShellError, Span, Type, Value};
use num_format::{Locale, WriteFormatted};
use serde::{Deserialize, Serialize};
use std::{
    char,
    fmt::{self, Write},
    iter::Sum,
    ops::{Add, Mul, Neg, Sub},
    str::FromStr,
};
use thiserror::Error;

/// A signed number of bytes.
///
/// [`Filesize`] is a wrapper around [`i64`]. Whereas [`i64`] is a dimensionless value, [`Filesize`] represents a
/// numerical value with a dimensional unit (byte).
///
/// A [`Filesize`] can be created from an [`i64`] using [`Filesize::new`] or the `From` or `Into` trait implementations.
/// To get the underlying [`i64`] value, use [`Filesize::get`] or the `From` or `Into` trait implementations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Filesize(i64);

impl Filesize {
    /// A [`Filesize`] of 0 bytes.
    pub const ZERO: Self = Self(0);

    /// The smallest possible [`Filesize`] value.
    pub const MIN: Self = Self(i64::MIN);

    /// The largest possible [`Filesize`] value.
    pub const MAX: Self = Self(i64::MAX);

    /// Create a new [`Filesize`] from a [`i64`] number of bytes.
    pub const fn new(bytes: i64) -> Self {
        Self(bytes)
    }

    /// Creates a [`Filesize`] from a signed multiple of a [`FilesizeUnit`].
    ///
    /// If the resulting number of bytes calculated by `value * unit.as_bytes()` overflows an
    /// [`i64`], then `None` is returned.
    pub const fn from_unit(value: i64, unit: FilesizeUnit) -> Option<Self> {
        if let Some(bytes) = value.checked_mul(unit.as_bytes() as i64) {
            Some(Self(bytes))
        } else {
            None
        }
    }

    /// Returns the underlying [`i64`] number of bytes in a [`Filesize`].
    pub const fn get(&self) -> i64 {
        self.0
    }

    /// Returns true if a [`Filesize`] is positive and false if it is zero or negative.
    pub const fn is_positive(self) -> bool {
        self.0.is_positive()
    }

    /// Returns true if a [`Filesize`] is negative and false if it is zero or positive.
    pub const fn is_negative(self) -> bool {
        self.0.is_negative()
    }

    /// Returns a [`Filesize`] representing the sign of `self`.
    /// - 0 if the file size is zero
    /// - 1 if the file size is positive
    /// - -1 if the file size is negative
    pub const fn signum(self) -> Self {
        Self(self.0.signum())
    }

    /// Returns the largest [`FilesizeUnit`] with a metric prefix that is smaller than or equal to `self`.
    ///
    /// # Examples
    /// ```
    /// # use nu_protocol::{Filesize, FilesizeUnit};
    ///
    /// let filesize = Filesize::from(FilesizeUnit::KB);
    /// assert_eq!(filesize.largest_metric_unit(), FilesizeUnit::KB);
    ///
    /// let filesize = Filesize::new(FilesizeUnit::KB.as_bytes() as i64 - 1);
    /// assert_eq!(filesize.largest_metric_unit(), FilesizeUnit::B);
    ///
    /// let filesize = Filesize::from(FilesizeUnit::KiB);
    /// assert_eq!(filesize.largest_metric_unit(), FilesizeUnit::KB);
    /// ```
    pub const fn largest_metric_unit(&self) -> FilesizeUnit {
        const KB: u64 = FilesizeUnit::KB.as_bytes();
        const MB: u64 = FilesizeUnit::MB.as_bytes();
        const GB: u64 = FilesizeUnit::GB.as_bytes();
        const TB: u64 = FilesizeUnit::TB.as_bytes();
        const PB: u64 = FilesizeUnit::PB.as_bytes();
        const EB: u64 = FilesizeUnit::EB.as_bytes();

        match self.0.unsigned_abs() {
            0..KB => FilesizeUnit::B,
            KB..MB => FilesizeUnit::KB,
            MB..GB => FilesizeUnit::MB,
            GB..TB => FilesizeUnit::GB,
            TB..PB => FilesizeUnit::TB,
            PB..EB => FilesizeUnit::PB,
            EB.. => FilesizeUnit::EB,
        }
    }

    /// Returns the largest [`FilesizeUnit`] with a binary prefix that is smaller than or equal to `self`.
    ///
    /// # Examples
    /// ```
    /// # use nu_protocol::{Filesize, FilesizeUnit};
    ///
    /// let filesize = Filesize::from(FilesizeUnit::KiB);
    /// assert_eq!(filesize.largest_binary_unit(), FilesizeUnit::KiB);
    ///
    /// let filesize = Filesize::new(FilesizeUnit::KiB.as_bytes() as i64 - 1);
    /// assert_eq!(filesize.largest_binary_unit(), FilesizeUnit::B);
    ///
    /// let filesize = Filesize::from(FilesizeUnit::MB);
    /// assert_eq!(filesize.largest_binary_unit(), FilesizeUnit::KiB);
    /// ```
    pub const fn largest_binary_unit(&self) -> FilesizeUnit {
        const KIB: u64 = FilesizeUnit::KiB.as_bytes();
        const MIB: u64 = FilesizeUnit::MiB.as_bytes();
        const GIB: u64 = FilesizeUnit::GiB.as_bytes();
        const TIB: u64 = FilesizeUnit::TiB.as_bytes();
        const PIB: u64 = FilesizeUnit::PiB.as_bytes();
        const EIB: u64 = FilesizeUnit::EiB.as_bytes();

        match self.0.unsigned_abs() {
            0..KIB => FilesizeUnit::B,
            KIB..MIB => FilesizeUnit::KiB,
            MIB..GIB => FilesizeUnit::MiB,
            GIB..TIB => FilesizeUnit::GiB,
            TIB..PIB => FilesizeUnit::TiB,
            PIB..EIB => FilesizeUnit::PiB,
            EIB.. => FilesizeUnit::EiB,
        }
    }

    // /// Returns a struct that can be used to display a [`Filesize`] scaled to the given
    // /// [`FilesizeUnit`].
    // ///
    // /// You can use [`largest_binary_unit`](Filesize::largest_binary_unit) or
    // /// [`largest_metric_unit`](Filesize::largest_metric_unit) to automatically determine a
    // /// [`FilesizeUnit`] of appropriate scale for a specific [`Filesize`].
    // ///
    // /// The default [`Display`](fmt::Display) implementation for [`Filesize`] is
    // /// `self.display(self.largest_metric_unit())`.
    // ///
    // /// # Examples
    // /// ```
    // /// # use nu_protocol::{Filesize, FilesizeUnit};
    // /// let filesize = Filesize::from_unit(4, FilesizeUnit::KiB).unwrap();
    // ///
    // /// assert_eq!(filesize.display(FilesizeUnit::B).to_string(), "4096 B");
    // /// assert_eq!(filesize.display(FilesizeUnit::KiB).to_string(), "4 KiB");
    // /// assert_eq!(filesize.display(filesize.largest_binary_unit()).to_string(), "4 KiB");
    // /// assert_eq!(filesize.display(filesize.largest_metric_unit()).to_string(), "4.096 kB");
    // /// ```
    // pub fn display(&self, options: &DisplayFilesizeOptions) -> DisplayFilesize {
    //     options.display(*self)
    // }
}

impl From<i64> for Filesize {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<Filesize> for i64 {
    fn from(filesize: Filesize) -> Self {
        filesize.0
    }
}

macro_rules! impl_from {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for Filesize {
                #[inline]
                fn from(value: $ty) -> Self {
                    Self(value.into())
                }
            }

            impl TryFrom<Filesize> for $ty {
                type Error = <i64 as TryInto<$ty>>::Error;

                #[inline]
                fn try_from(filesize: Filesize) -> Result<Self, Self::Error> {
                    filesize.0.try_into()
                }
            }
        )*
    };
}

impl_from!(u8, i8, u16, i16, u32, i32);

macro_rules! impl_try_from {
    ($($ty:ty),* $(,)?) => {
        $(
            impl TryFrom<$ty> for Filesize {
                type Error = <$ty as TryInto<i64>>::Error;

                #[inline]
                fn try_from(value: $ty) -> Result<Self, Self::Error> {
                    value.try_into().map(Self)
                }
            }

            impl TryFrom<Filesize> for $ty {
                type Error = <i64 as TryInto<$ty>>::Error;

                #[inline]
                fn try_from(filesize: Filesize) -> Result<Self, Self::Error> {
                    filesize.0.try_into()
                }
            }
        )*
    };
}

impl_try_from!(u64, usize, isize);

/// The error type returned when a checked conversion from a floating point type fails.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Error)]
pub struct TryFromFloatError(());

impl fmt::Display for TryFromFloatError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "out of range float type conversion attempted")
    }
}

impl TryFrom<f64> for Filesize {
    type Error = TryFromFloatError;

    #[inline]
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if i64::MIN as f64 <= value && value <= i64::MAX as f64 {
            Ok(Self(value as i64))
        } else {
            Err(TryFromFloatError(()))
        }
    }
}

impl TryFrom<f32> for Filesize {
    type Error = TryFromFloatError;

    #[inline]
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if i64::MIN as f32 <= value && value <= i64::MAX as f32 {
            Ok(Self(value as i64))
        } else {
            Err(TryFromFloatError(()))
        }
    }
}

impl FromValue for Filesize {
    fn from_value(value: Value) -> Result<Self, ShellError> {
        value.as_filesize()
    }

    fn expected_type() -> Type {
        Type::Filesize
    }
}

impl IntoValue for Filesize {
    fn into_value(self, span: Span) -> Value {
        Value::filesize(self.0, span)
    }
}

impl Add for Filesize {
    type Output = Option<Self>;

    fn add(self, rhs: Self) -> Self::Output {
        self.0.checked_add(rhs.0).map(Self)
    }
}

impl Sub for Filesize {
    type Output = Option<Self>;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0.checked_sub(rhs.0).map(Self)
    }
}

impl Mul<i64> for Filesize {
    type Output = Option<Self>;

    fn mul(self, rhs: i64) -> Self::Output {
        self.0.checked_mul(rhs).map(Self)
    }
}

impl Mul<Filesize> for i64 {
    type Output = Option<Filesize>;

    fn mul(self, rhs: Filesize) -> Self::Output {
        self.checked_mul(rhs.0).map(Filesize::new)
    }
}

impl Mul<f64> for Filesize {
    type Output = Option<Self>;

    fn mul(self, rhs: f64) -> Self::Output {
        let bytes = ((self.0 as f64) * rhs).round();
        if i64::MIN as f64 <= bytes && bytes <= i64::MAX as f64 {
            Some(Self(bytes as i64))
        } else {
            None
        }
    }
}

impl Mul<Filesize> for f64 {
    type Output = Option<Filesize>;

    fn mul(self, rhs: Filesize) -> Self::Output {
        let bytes = (self * (rhs.0 as f64)).round();
        if i64::MIN as f64 <= bytes && bytes <= i64::MAX as f64 {
            Some(Filesize(bytes as i64))
        } else {
            None
        }
    }
}

impl Neg for Filesize {
    type Output = Option<Self>;

    fn neg(self) -> Self::Output {
        self.0.checked_neg().map(Self)
    }
}

impl Sum<Filesize> for Option<Filesize> {
    fn sum<I: Iterator<Item = Filesize>>(iter: I) -> Self {
        let mut sum = Filesize::ZERO;
        for filesize in iter {
            sum = (sum + filesize)?;
        }
        Some(sum)
    }
}

impl fmt::Display for Filesize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", FilesizeFormat::new().format(*self))
    }
}

/// All the possible filesize units for a [`Filesize`].
///
/// This type contains both units with metric (SI) prefixes which are powers of 10 (e.g., kB = 1000 bytes)
/// and units with binary prefixes which are powers of 2 (e.g., KiB = 1024 bytes).
///
/// The number of bytes in a [`FilesizeUnit`] can be obtained using
/// [`as_bytes`](FilesizeUnit::as_bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilesizeUnit {
    /// One byte
    B,
    /// Kilobyte = 1000 bytes
    KB,
    /// Megabyte = 10<sup>6</sup> bytes
    MB,
    /// Gigabyte = 10<sup>9</sup> bytes
    GB,
    /// Terabyte = 10<sup>12</sup> bytes
    TB,
    /// Petabyte = 10<sup>15</sup> bytes
    PB,
    /// Exabyte = 10<sup>18</sup> bytes
    EB,
    /// Kibibyte = 1024 bytes
    KiB,
    /// Mebibyte = 2<sup>20</sup> bytes
    MiB,
    /// Gibibyte = 2<sup>30</sup> bytes
    GiB,
    /// Tebibyte = 2<sup>40</sup> bytes
    TiB,
    /// Pebibyte = 2<sup>50</sup> bytes
    PiB,
    /// Exbibyte = 2<sup>60</sup> bytes
    EiB,
}

impl FilesizeUnit {
    /// Returns the number of bytes in a [`FilesizeUnit`].
    pub const fn as_bytes(&self) -> u64 {
        match self {
            Self::B => 1,
            Self::KB => 10_u64.pow(3),
            Self::MB => 10_u64.pow(6),
            Self::GB => 10_u64.pow(9),
            Self::TB => 10_u64.pow(12),
            Self::PB => 10_u64.pow(15),
            Self::EB => 10_u64.pow(18),
            Self::KiB => 2_u64.pow(10),
            Self::MiB => 2_u64.pow(20),
            Self::GiB => 2_u64.pow(30),
            Self::TiB => 2_u64.pow(40),
            Self::PiB => 2_u64.pow(50),
            Self::EiB => 2_u64.pow(60),
        }
    }

    /// Convert a [`FilesizeUnit`] to a [`Filesize`].
    ///
    /// To create a [`Filesize`] from a multiple of a [`FilesizeUnit`] use [`Filesize::from_unit`].
    pub const fn as_filesize(&self) -> Filesize {
        Filesize::new(self.as_bytes() as i64)
    }

    /// Returns the symbol [`str`] for a [`FilesizeUnit`].
    ///
    /// The symbol is exactly the same as the enum case name in Rust code except for
    /// [`FilesizeUnit::KB`] which is `kB`.
    ///
    /// # Examples
    /// ```
    /// # use nu_protocol::FilesizeUnit;
    /// assert_eq!(FilesizeUnit::B.as_str(), "B");
    /// assert_eq!(FilesizeUnit::KB.as_str(), "kB");
    /// assert_eq!(FilesizeUnit::KiB.as_str(), "KiB");
    /// ```
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::B => "B",
            Self::KB => "kB",
            Self::MB => "MB",
            Self::GB => "GB",
            Self::TB => "TB",
            Self::PB => "PB",
            Self::EB => "EB",
            Self::KiB => "KiB",
            Self::MiB => "MiB",
            Self::GiB => "GiB",
            Self::TiB => "TiB",
            Self::PiB => "PiB",
            Self::EiB => "EiB",
        }
    }

    /// Returns `true` if a [`FilesizeUnit`] has a metric (SI) prefix (a power of 10).
    ///
    /// Note that this returns `true` for [`FilesizeUnit::B`] as well.
    pub const fn is_metric(&self) -> bool {
        match self {
            Self::B | Self::KB | Self::MB | Self::GB | Self::TB | Self::PB | Self::EB => true,
            Self::KiB | Self::MiB | Self::GiB | Self::TiB | Self::PiB | Self::EiB => false,
        }
    }

    /// Returns `true` if a [`FilesizeUnit`] has a binary prefix (a power of 2).
    ///
    /// Note that this returns `true` for [`FilesizeUnit::B`] as well.
    pub const fn is_binary(&self) -> bool {
        match self {
            Self::KB | Self::MB | Self::GB | Self::TB | Self::PB | Self::EB => false,
            Self::B | Self::KiB | Self::MiB | Self::GiB | Self::TiB | Self::PiB | Self::EiB => true,
        }
    }
}

impl From<FilesizeUnit> for Filesize {
    fn from(unit: FilesizeUnit) -> Self {
        unit.as_filesize()
    }
}

/// The error returned when failing to parse a [`FilesizeUnit`].
///
/// This occurs when the string being parsed does not exactly match the name of one of the
/// enum cases in [`FilesizeUnit`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Error)]
pub struct ParseFilesizeUnitError(());

impl fmt::Display for ParseFilesizeUnitError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "invalid file size unit")
    }
}

impl FromStr for FilesizeUnit {
    type Err = ParseFilesizeUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "B" => Self::B,
            "kB" => Self::KB,
            "MB" => Self::MB,
            "GB" => Self::GB,
            "TB" => Self::TB,
            "PB" => Self::PB,
            "EB" => Self::EB,
            "KiB" => Self::KiB,
            "MiB" => Self::MiB,
            "GiB" => Self::GiB,
            "TiB" => Self::TiB,
            "PiB" => Self::PiB,
            "EiB" => Self::EiB,
            _ => return Err(ParseFilesizeUnitError(())),
        })
    }
}

impl fmt::Display for FilesizeUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilesizeUnitFormat {
    Metric,
    Binary,
    Unit(FilesizeUnit),
}

impl FilesizeUnitFormat {
    pub const fn unit(unit: FilesizeUnit) -> Self {
        Self::Unit(unit)
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Metric => "metric",
            Self::Binary => "binary",
            Self::Unit(unit) => unit.as_str(),
        }
    }

    /// Returns `true` for [`DisplayFilesizeUnit::Metric`] or if the underlying [`FilesizeUnit`] is metric according to [`FilesizeUnit::is_metric`].
    ///
    /// Note that this returns `true` for [`FilesizeUnit::B`] as well.
    pub const fn is_metric(&self) -> bool {
        match self {
            Self::Metric => true,
            Self::Binary => false,
            Self::Unit(unit) => unit.is_metric(),
        }
    }

    /// Returns `true` for [`DisplayFilesizeUnit::Binary`] or if the underlying [`FilesizeUnit`] is binary according to [`FilesizeUnit::is_binary`].
    ///
    /// Note that this returns `true` for [`FilesizeUnit::B`] as well.
    pub const fn is_binary(&self) -> bool {
        match self {
            Self::Metric => false,
            Self::Binary => true,
            Self::Unit(unit) => unit.is_binary(),
        }
    }
}

impl From<FilesizeUnit> for FilesizeUnitFormat {
    fn from(unit: FilesizeUnit) -> Self {
        Self::unit(unit)
    }
}

/// The error returned when failing to parse a [`DisplayFilesizeUnit`].
///
/// This occurs when the string being parsed does not exactly match any of:
/// - `metric`
/// - `binary`
/// - the name of any of the enum cases in [`FilesizeUnit`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Error)]
pub struct ParseFilesizeUnitFormatError(());

impl fmt::Display for ParseFilesizeUnitFormatError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "invalid file size unit format")
    }
}

impl FromStr for FilesizeUnitFormat {
    type Err = ParseFilesizeUnitFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "metric" => Self::Metric,
            "binary" => Self::Binary,
            s => Self::unit(s.parse().map_err(|_| ParseFilesizeUnitFormatError(()))?),
        })
    }
}

impl fmt::Display for FilesizeUnitFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FilesizeFormat {
    unit: FilesizeUnitFormat,
    precision: Option<usize>,
    locale: Locale,
}

impl FilesizeFormat {
    pub fn new() -> Self {
        FilesizeFormat {
            unit: FilesizeUnitFormat::Metric,
            precision: None,
            locale: Locale::en_US_POSIX,
        }
    }

    pub fn unit(mut self, unit: impl Into<FilesizeUnitFormat>) -> Self {
        self.unit = unit.into();
        self
    }

    pub fn precision(mut self, precision: impl Into<Option<usize>>) -> Self {
        self.precision = precision.into();
        self
    }

    pub fn locale(mut self, locale: Locale) -> Self {
        self.locale = locale;
        self
    }

    pub fn format(&self, filesize: Filesize) -> FormattedFilesize {
        FormattedFilesize {
            format: *self,
            filesize,
        }
    }
}

impl Default for FilesizeFormat {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct FormattedFilesize {
    format: FilesizeFormat,
    filesize: Filesize,
}

impl fmt::Display for FormattedFilesize {
    fn fmt(&self, mut f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { filesize, format } = *self;
        let FilesizeFormat {
            unit,
            precision,
            locale,
        } = format;
        let unit = match unit {
            FilesizeUnitFormat::Metric => filesize.largest_metric_unit(),
            FilesizeUnitFormat::Binary => filesize.largest_binary_unit(),
            FilesizeUnitFormat::Unit(unit) => unit,
        };
        let Filesize(filesize) = filesize;
        let precision = precision.or(f.precision());

        // Format an exact, possibly fractional, string representation of `filesize`.
        let bytes = unit.as_bytes() as i64;
        let whole = filesize / bytes;
        let fract = (filesize % bytes).unsigned_abs();

        f.write_formatted(&whole, &locale)
            .map_err(|_| std::fmt::Error)?;

        if unit != FilesizeUnit::B && precision != Some(0) && !(precision.is_none() && fract == 0) {
            f.write_str(locale.decimal())?;

            let bytes = unit.as_bytes();
            let mut fract = fract * 10;
            let mut i = 0;
            loop {
                let q = fract / bytes;
                let r = fract % bytes;
                // Quick soundness proof:
                // r <= bytes                by definition of remainder `%`
                // => 10 * r <= 10 * bytes
                // => fract <= 10 * bytes    before next iteration, fract = r * 10
                // => fract / bytes <= 10
                // => q <= 10                next iteration, q = fract / bytes
                debug_assert!(q <= 10);
                f.write_char(char::from_digit(q as u32, 10).expect("q <= 10"))?;
                i += 1;
                if r == 0 || precision.is_some_and(|p| i >= p) {
                    break;
                }
                fract = r * 10;
            }

            if let Some(precision) = precision {
                for _ in 0..(precision - i) {
                    f.write_char('0')?;
                }
            }
        }

        write!(f, " {unit}")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(1024, FilesizeUnit::KB, "1.024 kB")]
    #[case(1024, FilesizeUnit::B, "1024 B")]
    #[case(1024, FilesizeUnit::KiB, "1 KiB")]
    #[case(3_000_000, FilesizeUnit::MB, "3 MB")]
    #[case(3_000_000, FilesizeUnit::KB, "3000 kB")]
    fn display_unit(#[case] bytes: i64, #[case] unit: FilesizeUnit, #[case] exp: &str) {
        assert_eq!(
            exp,
            FilesizeFormat::new()
                .unit(unit)
                .format(Filesize::new(bytes))
                .to_string()
        );
    }

    #[rstest]
    #[case(1000, "1000 B")]
    #[case(1024, "1 KiB")]
    #[case(1025, "1.0009765625 KiB")]
    fn display_auto_binary(#[case] val: i64, #[case] exp: &str) {
        assert_eq!(
            exp,
            FilesizeFormat::new()
                .unit(FilesizeUnitFormat::Binary)
                .format(Filesize::new(val))
                .to_string()
        );
    }

    #[rstest]
    #[case(999, "999 B")]
    #[case(1000, "1 kB")]
    #[case(1024, "1.024 kB")]
    fn display_auto_metric(#[case] val: i64, #[case] exp: &str) {
        assert_eq!(
            exp,
            FilesizeFormat::new()
                .unit(FilesizeUnitFormat::Metric)
                .format(Filesize::new(val))
                .to_string()
        );
    }
}
