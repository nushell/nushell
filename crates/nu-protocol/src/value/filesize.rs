use crate::Config;
use byte_unit::UnitType;
use nu_utils::get_system_locale;
use num_format::ToFormattedString;

pub fn format_filesize_from_conf(num_bytes: i64, config: &Config) -> String {
    // We need to take into account config.filesize_metric so, if someone asks for KB
    // and filesize_metric is false, return KiB
    format_filesize(
        num_bytes,
        config.filesize_format.as_str(),
        Some(config.filesize_metric),
    )
}

// filesize_metric is explicit when printed a value according to user config;
// other places (such as `format filesize`) don't.
pub fn format_filesize(
    num_bytes: i64,
    format_value: &str,
    filesize_metric: Option<bool>,
) -> String {
    // Allow the user to specify how they want their numbers formatted

    // When format_value is "auto" or an invalid value, the returned ByteUnit doesn't matter
    // and is always B.
    let filesize_unit = get_filesize_format(format_value, filesize_metric);
    let byte = byte_unit::Byte::from_u64(num_bytes.unsigned_abs());
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
            let locale_signed_byte_string = if num_bytes.is_negative() {
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
            if num_bytes.is_negative() {
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
    macro_rules! either {
        ($metric:ident, $binary:ident) => {
            Some(if metric {
                byte_unit::Unit::$metric
            } else {
                byte_unit::Unit::$binary
            })
        };
    }
    match format_value {
        "b" => Some(byte_unit::Unit::B),
        "kb" | "kib" => either!(KB, KiB),
        "mb" | "mib" => either!(MB, MiB),
        "gb" | "gib" => either!(GB, GiB),
        "tb" | "tib" => either!(TB, TiB),
        "pb" | "pib" => either!(TB, TiB),
        "eb" | "eib" => either!(EB, EiB),
        _ => None,
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
        assert_eq!(exp, format_filesize(val, &filesize_format, filesize_metric));
    }
}
