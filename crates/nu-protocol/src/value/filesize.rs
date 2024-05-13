use crate::{ast::FilesizeUnit, Config};
use byte_unit::UnitType;
use nu_utils::get_system_locale;
use num_format::ToFormattedString;

pub fn format_filesize_from_conf(num_bytes: i64, config: &Config) -> String {
    // We need to take into account config.filesize_metric so, if someone asks for KB
    // and filesize_metric is false, return KiB
    format_filesize(
        num_bytes,
        todo!(), // config.filesize_format.as_str(),
        Some(config.filesize_metric),
    )
}

// filesize_metric is explicit when printed a value according to user config;
// other places (such as `format filesize`) don't.
pub fn format_filesize(num_bytes: i64, unit: Option<FilesizeUnit>, metric: Option<bool>) -> String {
    let byte = byte_unit::Byte::from_u64(num_bytes.unsigned_abs());
    let adj_byte = if let Some(unit) = unit {
        byte.get_adjusted_unit(get_filesize_format(unit, metric).into())
    } else {
        byte.get_appropriate_unit(if metric.unwrap_or(false) {
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
            let locale_signed_byte_string = if num_bytes.is_negative() {
                format!("-{locale_byte_string}")
            } else {
                locale_byte_string
            };

            if unit.is_none() {
                format!("{locale_signed_byte_string} B")
            } else {
                locale_signed_byte_string
            }
        }
        _ => {
            if num_bytes.is_negative() {
                format!("-{:.1}", adj_byte)
            } else {
                format!("{:.1}", adj_byte)
            }
        }
    }
}

/// Get the filesize unit, or None if format is "auto"
fn get_filesize_format(unit: FilesizeUnit, metric: Option<bool>) -> FilesizeUnit {
    // filesize_metric always overrides the unit of filesize_format.
    if metric.unwrap_or(unit.is_metric()) {
        match unit {
            FilesizeUnit::Byte => FilesizeUnit::Byte,
            FilesizeUnit::Kilobyte | FilesizeUnit::Kibibyte => FilesizeUnit::Kilobyte,
            FilesizeUnit::Megabyte | FilesizeUnit::Mebibyte => FilesizeUnit::Megabyte,
            FilesizeUnit::Gigabyte | FilesizeUnit::Gibibyte => FilesizeUnit::Gigabyte,
            FilesizeUnit::Terabyte | FilesizeUnit::Tebibyte => FilesizeUnit::Terabyte,
            FilesizeUnit::Petabyte | FilesizeUnit::Pebibyte => FilesizeUnit::Petabyte,
            FilesizeUnit::Exabyte | FilesizeUnit::Exbibyte => FilesizeUnit::Exabyte,
        }
    } else {
        match unit {
            FilesizeUnit::Byte => FilesizeUnit::Byte,
            FilesizeUnit::Kilobyte | FilesizeUnit::Kibibyte => FilesizeUnit::Kibibyte,
            FilesizeUnit::Megabyte | FilesizeUnit::Mebibyte => FilesizeUnit::Mebibyte,
            FilesizeUnit::Gigabyte | FilesizeUnit::Gibibyte => FilesizeUnit::Gibibyte,
            FilesizeUnit::Terabyte | FilesizeUnit::Tebibyte => FilesizeUnit::Tebibyte,
            FilesizeUnit::Petabyte | FilesizeUnit::Pebibyte => FilesizeUnit::Pebibyte,
            FilesizeUnit::Exabyte | FilesizeUnit::Exbibyte => FilesizeUnit::Exbibyte,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(1000, Some(true), None, "1.0 KB")]
    #[case(1000, Some(false), None, "1,000 B")]
    #[case(1000, Some(false), Some(FilesizeUnit::Kilobyte), "1.0 KiB")]
    #[case(3000, Some(false), None, "2.9 KiB")]
    #[case(3_000_000, None, None, "2.9 MiB")]
    #[case(3_000_000, None, Some(FilesizeUnit::Kibibyte), "2929.7 KiB")]
    fn test_filesize(
        #[case] val: i64,
        #[case] metric: Option<bool>,
        #[case] unit: Option<FilesizeUnit>,
        #[case] exp: &str,
    ) {
        assert_eq!(exp, format_filesize(val, unit, metric));
    }
}
