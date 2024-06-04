use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    str::FromStr,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilesizeUnit {
    // Filesize units: metric
    Byte,
    Kilobyte,
    Megabyte,
    Gigabyte,
    Terabyte,
    Petabyte,
    Exabyte,

    // Filesize units: ISO/IEC 80000
    Kibibyte,
    Mebibyte,
    Gibibyte,
    Tebibyte,
    Pebibyte,
    Exbibyte,
}

impl FilesizeUnit {
    pub const fn is_metric(&self) -> bool {
        match self {
            FilesizeUnit::Byte
            | FilesizeUnit::Kilobyte
            | FilesizeUnit::Megabyte
            | FilesizeUnit::Gigabyte
            | FilesizeUnit::Terabyte
            | FilesizeUnit::Petabyte
            | FilesizeUnit::Exabyte => true,
            FilesizeUnit::Kibibyte
            | FilesizeUnit::Mebibyte
            | FilesizeUnit::Gibibyte
            | FilesizeUnit::Tebibyte
            | FilesizeUnit::Pebibyte
            | FilesizeUnit::Exbibyte => false,
        }
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            FilesizeUnit::Byte => "B",
            FilesizeUnit::Kilobyte => "KB",
            FilesizeUnit::Megabyte => "MB",
            FilesizeUnit::Gigabyte => "GB",
            FilesizeUnit::Terabyte => "TB",
            FilesizeUnit::Petabyte => "PB",
            FilesizeUnit::Exabyte => "EB",
            FilesizeUnit::Kibibyte => "KiB",
            FilesizeUnit::Mebibyte => "MiB",
            FilesizeUnit::Gibibyte => "GiB",
            FilesizeUnit::Tebibyte => "TiB",
            FilesizeUnit::Pebibyte => "PiB",
            FilesizeUnit::Exbibyte => "EiB",
        }
    }

    pub const fn as_bytes_u64(&self) -> u64 {
        const BASE_10: u64 = 1000;
        const BASE_2: u64 = 1024;

        match self {
            FilesizeUnit::Byte => 1,
            FilesizeUnit::Kilobyte => BASE_10,
            FilesizeUnit::Megabyte => BASE_10.pow(2),
            FilesizeUnit::Gigabyte => BASE_10.pow(3),
            FilesizeUnit::Terabyte => BASE_10.pow(4),
            FilesizeUnit::Petabyte => BASE_10.pow(5),
            FilesizeUnit::Exabyte => BASE_10.pow(6),
            FilesizeUnit::Kibibyte => BASE_2,
            FilesizeUnit::Mebibyte => BASE_2.pow(2),
            FilesizeUnit::Gibibyte => BASE_2.pow(3),
            FilesizeUnit::Tebibyte => BASE_2.pow(4),
            FilesizeUnit::Pebibyte => BASE_2.pow(5),
            FilesizeUnit::Exbibyte => BASE_2.pow(6),
        }
    }

    pub const fn as_bytes_i64(&self) -> i64 {
        self.as_bytes_u64() as i64
    }
}

impl Display for FilesizeUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<FilesizeUnit> for byte_unit::Unit {
    fn from(unit: FilesizeUnit) -> Self {
        match unit {
            FilesizeUnit::Byte => byte_unit::Unit::B,
            FilesizeUnit::Kilobyte => byte_unit::Unit::KB,
            FilesizeUnit::Megabyte => byte_unit::Unit::MB,
            FilesizeUnit::Gigabyte => byte_unit::Unit::GB,
            FilesizeUnit::Terabyte => byte_unit::Unit::TB,
            FilesizeUnit::Petabyte => byte_unit::Unit::PB,
            FilesizeUnit::Exabyte => byte_unit::Unit::EB,
            FilesizeUnit::Kibibyte => byte_unit::Unit::KiB,
            FilesizeUnit::Mebibyte => byte_unit::Unit::MiB,
            FilesizeUnit::Gibibyte => byte_unit::Unit::GiB,
            FilesizeUnit::Tebibyte => byte_unit::Unit::TiB,
            FilesizeUnit::Pebibyte => byte_unit::Unit::PiB,
            FilesizeUnit::Exbibyte => byte_unit::Unit::EiB,
        }
    }
}

impl FromStr for FilesizeUnit {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const ERR: &str = "expected either 'B', 'KB', 'KiB', 'MB', 'MiB', 'GB', 'GiB', 'TB', 'TiB', 'PB', 'PiB', 'EB', or 'EiB'";
        Ok(match s.to_ascii_lowercase().as_str() {
            "b" => FilesizeUnit::Byte,
            "kb" => FilesizeUnit::Kilobyte,
            "kib" => FilesizeUnit::Kibibyte,
            "mb" => FilesizeUnit::Megabyte,
            "mib" => FilesizeUnit::Mebibyte,
            "gb" => FilesizeUnit::Gigabyte,
            "gib" => FilesizeUnit::Gibibyte,
            "tb" => FilesizeUnit::Terabyte,
            "tib" => FilesizeUnit::Tebibyte,
            "pb" => FilesizeUnit::Petabyte,
            "pib" => FilesizeUnit::Pebibyte,
            "eb" => FilesizeUnit::Exabyte,
            "eib" => FilesizeUnit::Exbibyte,
            _ => return Err(ERR),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurationUnit {
    Nanosecond,
    Microsecond,
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
}

impl DurationUnit {
    pub const fn as_str(&self) -> &'static str {
        match self {
            DurationUnit::Nanosecond => "ns",
            DurationUnit::Microsecond => "µs",
            DurationUnit::Millisecond => "ms",
            DurationUnit::Second => "sec",
            DurationUnit::Minute => "min",
            DurationUnit::Hour => "hr",
            DurationUnit::Day => "day",
            DurationUnit::Week => "wk",
        }
    }

    pub const fn as_nanos_u64(&self) -> u64 {
        const BASE_10: u64 = 10;
        const NS_PER_SEC: u64 = BASE_10.pow(9);

        match self {
            DurationUnit::Nanosecond => 1,
            DurationUnit::Microsecond => BASE_10.pow(3),
            DurationUnit::Millisecond => BASE_10.pow(6),
            DurationUnit::Second => NS_PER_SEC,
            DurationUnit::Minute => 60 * NS_PER_SEC,
            DurationUnit::Hour => 60 * 60 * NS_PER_SEC,
            DurationUnit::Day => 24 * 60 * 60 * NS_PER_SEC,
            DurationUnit::Week => 7 * 24 * 60 * 60 * NS_PER_SEC,
        }
    }

    pub const fn as_nanos_i64(&self) -> i64 {
        self.as_nanos_u64() as i64
    }
}

impl Display for DurationUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DurationUnit {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "ns" => DurationUnit::Nanosecond,
            "us" | "µs" | "μs" => DurationUnit::Microsecond,
            "ms" => DurationUnit::Millisecond,
            "sec" => DurationUnit::Second,
            "min" => DurationUnit::Minute,
            "hr" => DurationUnit::Hour,
            "day" => DurationUnit::Day,
            "wk" => DurationUnit::Week,
            _ => return Err(
                "expected either 'ns', 'us'/'µs'/'μs', 'ms', 'sec', 'min', 'hr', 'day', or 'wk'",
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_lossless_signed() {
        assert!(i64::try_from(DurationUnit::Week.as_nanos_u64()).is_ok());
    }

    #[test]
    fn duration_lossless_float() {
        let nanos = DurationUnit::Week.as_nanos_i64();
        assert_eq!(nanos, nanos as f64 as i64);
    }

    #[test]
    fn filesize_lossless_signed() {
        assert!(i64::try_from(FilesizeUnit::Exbibyte.as_bytes_u64()).is_ok());
    }

    #[test]
    fn filesize_lossless_float() {
        let bytes = FilesizeUnit::Exbibyte.as_bytes_i64();
        assert_eq!(bytes, bytes as f64 as i64);
    }
}
