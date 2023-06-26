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

    // Filesize units: ISO/IEC 80000
    Kibibyte,
    Mebibyte,
    Gibibyte,
    Tebibyte,
    Pebibyte,
    Exbibyte,

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
    Millennium,
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
            Unit::Millennium => Value::Duration {
                val: NuDuration {
                    quantity: size,
                    unit: Unit::Millennium,
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
            Unit::Millennium => (1000 * 100 * 12, Unit::Month),
            _ => {
                unimplemented!("no unit_scale for this unit");
            }
        }
    }
}

pub type UnitGroup<'unit> = (Unit, &'unit str, Option<(Unit, i64)>);

pub const FILESIZE_UNIT_GROUPS: &[UnitGroup] = &[
    (Unit::Kilobyte, "KB", Some((Unit::Byte, 1000))),
    (Unit::Megabyte, "MB", Some((Unit::Kilobyte, 1000))),
    (Unit::Gigabyte, "GB", Some((Unit::Megabyte, 1000))),
    (Unit::Terabyte, "TB", Some((Unit::Gigabyte, 1000))),
    (Unit::Petabyte, "PB", Some((Unit::Terabyte, 1000))),
    (Unit::Exabyte, "EB", Some((Unit::Petabyte, 1000))),
    (Unit::Zettabyte, "ZB", Some((Unit::Exabyte, 1000))),
    (Unit::Kibibyte, "KIB", Some((Unit::Byte, 1024))),
    (Unit::Mebibyte, "MIB", Some((Unit::Kibibyte, 1024))),
    (Unit::Gibibyte, "GIB", Some((Unit::Mebibyte, 1024))),
    (Unit::Tebibyte, "TIB", Some((Unit::Gibibyte, 1024))),
    (Unit::Pebibyte, "PIB", Some((Unit::Tebibyte, 1024))),
    (Unit::Exbibyte, "EIB", Some((Unit::Pebibyte, 1024))),
    (Unit::Zebibyte, "ZIB", Some((Unit::Exbibyte, 1024))),
    (Unit::Byte, "B", None),
];

pub const DURATION_UNIT_GROUPS: &[UnitGroup] = &[
    (Unit::Minute, "m", None),
    (Unit::Day, "d", None),
    (Unit::Hour, "h", None),
    (Unit::Year, "y", None),
    (Unit::Second, "s", None),
    (Unit::Nanosecond, "ns", None),
    (Unit::Microsecond, "us", None),
    (Unit::Millisecond, "ms", None),
    (Unit::Hour, "hr", None),
    (Unit::Day, "da", None),
    (Unit::Week, "wk", None),
    (Unit::Month, "mo", None),
    (Unit::Year, "yr", None),
    (Unit::Second, "sec", None),
    (Unit::Minute, "min", None),
    (Unit::Hour, "hrs", None),
    (Unit::Day, "day", None),
    (Unit::Week, "wks", None),
    (Unit::Month, "mos", None),
    /* and not (Unit::Month, "mon", None), reserved for "monday" */
    (Unit::Quarter, "qtr", None),
    (Unit::Year, "yrs", None),
    (Unit::Second, "secs", None),
    (Unit::Minute, "mins", None),
    (Unit::Hour, "hour", None),
    (Unit::Day, "days", None),
    (Unit::Week, "week", None),
    (Unit::Quarter, "qtrs", None),
    (Unit::Year, "year", None),
    (Unit::Hour, "hours", None),
    (Unit::Week, "weeks", None),
    (Unit::Month, "month", None),
    (Unit::Year, "years", None),
    (Unit::Second, "second", None),
    (Unit::Minute, "minute", None),
    (Unit::Month, "months", None),
    (Unit::Second, "seconds", None),
    (Unit::Minute, "minutes", None),
    (Unit::Quarter, "quarter", None),
    (Unit::Century, "century", None),
    (Unit::Quarter, "quarters", None),
    (Unit::Millennium, "millennia", None),
    (Unit::Microsecond, "\u{00B5}s", None),
    (Unit::Microsecond, "\u{03BC}s", None),
    (Unit::Century, "centuries", None),
    (Unit::Millennium, "millennium", None),
    (Unit::Nanosecond, "nanosecond", None),
    (Unit::Nanosecond, "nanoseconds", None),
    (Unit::Microsecond, "microsecond", None),
    (Unit::Millisecond, "millisecond", None),
    (Unit::Microsecond, "microseconds", None),
    (Unit::Millisecond, "milliseconds", None),
];
