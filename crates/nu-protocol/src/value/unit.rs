use crate::NuDuration;
use crate::{Span, Value};
use serde::{Deserialize, Serialize};
use strum_macros::Display;

#[derive(
    Debug, Clone, Copy, Display, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Unit {
    // Filesize units: metric
    Byte,
    Kilobyte,
    Megabyte,
    Gigabyte,
    Terabyte,
    Petabyte,
    Exabyte,
    Zettabyte,

    // Filesize units: ISO/IEC 80000
    Kibibyte,
    Mebibyte,
    Gibibyte,
    Tebibyte,
    Pebibyte,
    Exbibyte,
    Zebibyte,

    // Duration units (these retain separate unit of measure)
    Nanosecond,
    Microsecond,
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Quarter,
    Year,
    Century,
    Millenium,
}

impl Unit {
    pub fn to_value(&self, size: i64, span: Span) -> Value {
        match self {
            Unit::Byte => Value::Filesize { val: size, span },
            Unit::Kilobyte => Value::Filesize {
                val: size * 1000,
                span,
            },
            Unit::Megabyte => Value::Filesize {
                val: size * 1000 * 1000,
                span,
            },
            Unit::Gigabyte => Value::Filesize {
                val: size * 1000 * 1000 * 1000,
                span,
            },
            Unit::Terabyte => Value::Filesize {
                val: size * 1000 * 1000 * 1000 * 1000,
                span,
            },
            Unit::Petabyte => Value::Filesize {
                val: size * 1000 * 1000 * 1000 * 1000 * 1000,
                span,
            },
            Unit::Exabyte => Value::Filesize {
                val: size * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                span,
            },
            Unit::Zettabyte => Value::Filesize {
                val: size * 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                span,
            },

            Unit::Kibibyte => Value::Filesize {
                val: size * 1024,
                span,
            },
            Unit::Mebibyte => Value::Filesize {
                val: size * 1024 * 1024,
                span,
            },
            Unit::Gibibyte => Value::Filesize {
                val: size * 1024 * 1024 * 1024,
                span,
            },
            Unit::Tebibyte => Value::Filesize {
                val: size * 1024 * 1024 * 1024 * 1024,
                span,
            },
            Unit::Pebibyte => Value::Filesize {
                val: size * 1024 * 1024 * 1024 * 1024 * 1024,
                span,
            },
            Unit::Exbibyte => Value::Filesize {
                val: size * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
                span,
            },
            Unit::Zebibyte => Value::Filesize {
                val: size * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
                span,
            },
            Unit::Nanosecond => Value::Duration {
                val: NuDuration {
                    quantity: size,
                    unit: Unit::Nanosecond,
                },
                span,
            },
            Unit::Microsecond => Value::Duration {
                val: NuDuration {
                    quantity: size,
                    unit: Unit::Microsecond,
                },
                span,
            },
            Unit::Millisecond => Value::Duration {
                val: NuDuration {
                    quantity: size,
                    unit: Unit::Millisecond,
                },
                span,
            },
            Unit::Second => Value::Duration {
                val: NuDuration {
                    quantity: size,
                    unit: Unit::Second,
                },
                span,
            },
            Unit::Minute => Value::Duration {
                val: NuDuration {
                    quantity: size,
                    unit: Unit::Minute,
                },
                span,
            },
            Unit::Hour => Value::Duration {
                val: NuDuration {
                    quantity: size,
                    unit: Unit::Hour,
                },
                span,
            },
            Unit::Day => Value::Duration {
                val: NuDuration {
                    quantity: size,
                    unit: Unit::Day,
                },
                span,
            },
            Unit::Week => Value::Duration {
                val: NuDuration {
                    quantity: size,
                    unit: Unit::Week,
                },
                span,
            },
            Unit::Month => Value::Duration {
                val: NuDuration {
                    quantity: size,
                    unit: Unit::Month,
                },
                span,
            },
            Unit::Quarter => Value::Duration {
                val: NuDuration {
                    quantity: size,
                    unit: Unit::Quarter,
                },
                span,
            },
            Unit::Year => Value::Duration {
                val: NuDuration {
                    quantity: size,
                    unit: Unit::Year,
                },
                span,
            },
            Unit::Century => Value::Duration {
                val: NuDuration {
                    quantity: size,
                    unit: Unit::Century,
                },
                span,
            },
            Unit::Millenium => Value::Duration {
                val: NuDuration {
                    quantity: size,
                    unit: Unit::Millenium,
                },
                span,
            },
        }
    }

    pub fn unit_scale(&self) -> (i64, Unit) {
        match self {
            Unit::Nanosecond => (1, Unit::Nanosecond),
            Unit::Microsecond => (1_000, Unit::Nanosecond),
            Unit::Millisecond => (1_000_000, Unit::Nanosecond),
            Unit::Second => (1_000_000_000, Unit::Nanosecond),
            Unit::Minute => (60 * 1_000_000_000, Unit::Nanosecond),
            Unit::Hour => (60 * 60 * 1_000_000_000, Unit::Nanosecond),
            Unit::Day => (24 * 60 * 60 * 1_000_000_000, Unit::Nanosecond),
            Unit::Week => (7 * 24 * 60 * 60 * 1_000_000_000, Unit::Nanosecond),
            Unit::Month => (1, Unit::Month),
            Unit::Quarter => (4, Unit::Month),
            Unit::Year => (12, Unit::Month),
            Unit::Century => (100 * 12, Unit::Month),
            Unit::Millenium => (1000 * 100 * 12, Unit::Month),
            _ => {
                unimplemented!("no unit_scale for this unit");
            }
        }
    }
}
