use crate::{Config, FromValue, IntoValue, ShellError, Span, Type, Value};
use byte_unit::UnitType;
use nu_utils::get_system_locale;
use num_format::ToFormattedString;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    iter::Sum,
    ops::{Add, Mul, Neg, Sub},
};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Filesize(i64);

impl Filesize {
    pub const ZERO: Self = Self(0);

    pub const fn new(bytes: i64) -> Self {
        Self(bytes)
    }

    pub const fn get(&self) -> i64 {
        self.0
    }

    pub const fn is_positive(self) -> bool {
        self.0.is_positive()
    }

    pub const fn is_negative(self) -> bool {
        self.0.is_negative()
    }

    pub const fn signum(self) -> Self {
        Self(self.0.signum())
    }

    pub const fn from_unit(value: i64, unit: FilesizeUnit) -> Option<Self> {
        if let Some(bytes) = value.checked_mul(unit.as_bytes() as i64) {
            Some(Self(bytes))
        } else {
            None
        }
    }
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

impl TryFrom<u64> for Filesize {
    type Error = <u64 as TryInto<i64>>::Error;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        value.try_into().map(Self)
    }
}

impl TryFrom<Filesize> for u64 {
    type Error = <i64 as TryInto<u64>>::Error;

    fn try_from(filesize: Filesize) -> Result<Self, Self::Error> {
        filesize.0.try_into()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Error)]
pub struct TryFromFloatError(());

impl fmt::Display for TryFromFloatError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "out of range float type conversion attempted")
    }
}

impl TryFrom<f64> for Filesize {
    type Error = TryFromFloatError;

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

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if i64::MIN as f32 <= value && value <= i64::MAX as f32 {
            Ok(Self(value as i64))
        } else {
            Err(TryFromFloatError(()))
        }
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
        format_filesize(*self, "auto", Some(false)).fmt(f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilesizeUnit {
    B,
    KB,
    MB,
    GB,
    TB,
    PB,
    EB,
    KiB,
    MiB,
    GiB,
    TiB,
    PiB,
    EiB,
}

impl FilesizeUnit {
    pub const fn as_bytes(&self) -> u64 {
        match self {
            Self::B => 1,
            Self::KB => 10_u64.pow(3),
            Self::MB => 10_u64.pow(6),
            Self::GB => 10_u64.pow(9),
            Self::TB => 10_u64.pow(12),
            Self::PB => 10_u64.pow(15),
            Self::EB => 10_u64.pow(18),
            Self::KiB => 1 << 10,
            Self::MiB => 1 << 20,
            Self::GiB => 1 << 30,
            Self::TiB => 1 << 40,
            Self::PiB => 1 << 50,
            Self::EiB => 1 << 60,
        }
    }

    pub const fn as_filesize(&self) -> Filesize {
        Filesize::new(self.as_bytes() as i64)
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::B => "B",
            Self::KB => "KB",
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

    pub const fn is_decimal(&self) -> bool {
        match self {
            Self::B | Self::KB | Self::MB | Self::GB | Self::TB | Self::PB | Self::EB => true,
            Self::KiB | Self::MiB | Self::GiB | Self::TiB | Self::PiB | Self::EiB => false,
        }
    }

    pub const fn is_binary(&self) -> bool {
        match self {
            Self::KB | Self::MB | Self::GB | Self::TB | Self::PB | Self::EB => false,
            Self::B | Self::KiB | Self::MiB | Self::GiB | Self::TiB | Self::PiB | Self::EiB => true,
        }
    }
}

impl fmt::Display for FilesizeUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

pub fn format_filesize_from_conf(filesize: Filesize, config: &Config) -> String {
    // We need to take into account config.filesize_metric so, if someone asks for KB
    // and filesize_metric is false, return KiB
    format_filesize(
        filesize,
        &config.filesize.format,
        Some(config.filesize.metric),
    )
}

// filesize_metric is explicit when printed a value according to user config;
// other places (such as `format filesize`) don't.
pub fn format_filesize(
    filesize: Filesize,
    format_value: &str,
    filesize_metric: Option<bool>,
) -> String {
    // Allow the user to specify how they want their numbers formatted

    // When format_value is "auto" or an invalid value, the returned ByteUnit doesn't matter
    // and is always B.
    let filesize_unit = get_filesize_format(format_value, filesize_metric);
    let byte = byte_unit::Byte::from_u64(filesize.0.unsigned_abs());
    let adj_byte = if let Some(unit) = filesize_unit {
        byte.get_adjusted_unit(unit)
    } else {
        // When filesize_metric is None, format_value should never be "auto", so this
        // unwrap_or() should always work.
        byte.get_appropriate_unit(if filesize_metric.unwrap_or(false) {
            UnitType::Decimal
        } else {
            UnitType::Binary
        })
    };

    match adj_byte.get_unit() {
        byte_unit::Unit::B => {
            let locale = get_system_locale();
            let locale_byte = adj_byte.get_value() as u64;
            let locale_byte_string = locale_byte.to_formatted_string(&locale);
            let locale_signed_byte_string = if filesize.is_negative() {
                format!("-{locale_byte_string}")
            } else {
                locale_byte_string
            };

            if filesize_unit.is_none() {
                format!("{locale_signed_byte_string} B")
            } else {
                locale_signed_byte_string
            }
        }
        _ => {
            if filesize.is_negative() {
                format!("-{:.1}", adj_byte)
            } else {
                format!("{:.1}", adj_byte)
            }
        }
    }
}

/// Get the filesize unit, or None if format is "auto"
fn get_filesize_format(
    format_value: &str,
    filesize_metric: Option<bool>,
) -> Option<byte_unit::Unit> {
    // filesize_metric always overrides the unit of filesize_format.
    let metric = filesize_metric.unwrap_or(!format_value.ends_with("ib"));

    if metric {
        match format_value {
            "b" => Some(byte_unit::Unit::B),
            "kb" | "kib" => Some(byte_unit::Unit::KB),
            "mb" | "mib" => Some(byte_unit::Unit::MB),
            "gb" | "gib" => Some(byte_unit::Unit::GB),
            "tb" | "tib" => Some(byte_unit::Unit::TB),
            "pb" | "pib" => Some(byte_unit::Unit::TB),
            "eb" | "eib" => Some(byte_unit::Unit::EB),
            _ => None,
        }
    } else {
        match format_value {
            "b" => Some(byte_unit::Unit::B),
            "kb" | "kib" => Some(byte_unit::Unit::KiB),
            "mb" | "mib" => Some(byte_unit::Unit::MiB),
            "gb" | "gib" => Some(byte_unit::Unit::GiB),
            "tb" | "tib" => Some(byte_unit::Unit::TiB),
            "pb" | "pib" => Some(byte_unit::Unit::TiB),
            "eb" | "eib" => Some(byte_unit::Unit::EiB),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(1000, Some(true), "auto", "1.0 KB")]
    #[case(1000, Some(false), "auto", "1,000 B")]
    #[case(1000, Some(false), "kb", "1.0 KiB")]
    #[case(3000, Some(false), "auto", "2.9 KiB")]
    #[case(3_000_000, None, "auto", "2.9 MiB")]
    #[case(3_000_000, None, "kib", "2929.7 KiB")]
    fn test_filesize(
        #[case] val: i64,
        #[case] filesize_metric: Option<bool>,
        #[case] filesize_format: String,
        #[case] exp: &str,
    ) {
        assert_eq!(
            exp,
            format_filesize(Filesize::new(val), &filesize_format, filesize_metric)
        );
    }
}
