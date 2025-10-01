use crate::parse_date_from_string;
use chrono::{Local, TimeZone, Utc};
use fancy_regex::{Regex, RegexBuilder};
use nu_engine::command_prelude::*;
use std::sync::LazyLock;

#[derive(Clone)]
pub struct DetectType;

impl Command for DetectType {
    fn name(&self) -> &str {
        "detect type"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::String, Type::Any), (Type::Any, Type::Any)])
            .switch(
                "prefer-filesize",
                "For ints display them as human-readable file sizes",
                Some('f'),
            )
            .category(Category::Strings)
            .allow_variants_without_examples(true)
    }

    fn description(&self) -> &str {
        "Infer Nushell datatype from a string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "conversion"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Bool from string",
                example: "'true' | detect type",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Bool is case insensitive",
                example: "'FALSE' | detect type",
                result: Some(Value::test_bool(false)),
            },
            Example {
                description: "Int from plain digits",
                example: "'42' | detect type",
                result: Some(Value::test_int(42)),
            },
            Example {
                description: "Int with underscores",
                example: "'1_000_000' | detect type",
                result: Some(Value::test_int(1_000_000)),
            },
            Example {
                description: "Int with commas",
                example: "'1,234,567' | detect type",
                result: Some(Value::test_int(1_234_567)),
            },
            #[allow(clippy::approx_constant, reason = "approx PI in examples is fine")]
            Example {
                description: "Float from decimal",
                example: "'3.14' | detect type",
                result: Some(Value::test_float(3.14)),
            },
            Example {
                description: "Float in scientific notation",
                example: "'6.02e23' | detect type",
                result: Some(Value::test_float(6.02e23)),
            },
            Example {
                description: "Prefer filesize for ints",
                example: "'1024' | detect type -f",
                result: Some(Value::test_filesize(1024)),
            },
            Example {
                description: "Date Y-M-D",
                example: "'2022-01-01' | detect type",
                result: Some(Value::test_date(
                    Local.with_ymd_and_hms(2022, 1, 1, 0, 0, 0).unwrap().into(),
                )),
            },
            Example {
                description: "Date with time and offset",
                example: "'2022-01-01T00:00:00Z' | detect type",
                result: Some(Value::test_date(
                    Utc.with_ymd_and_hms(2022, 1, 1, 0, 0, 0).unwrap().into(),
                )),
            },
            Example {
                description: "Date D-M-Y",
                example: "'31-12-2021' | detect type",
                result: Some(Value::test_date(
                    Local
                        .with_ymd_and_hms(2021, 12, 31, 0, 0, 0)
                        .unwrap()
                        .into(),
                )),
            },
            Example {
                description: "Unknown stays a string",
                example: "'not-a-number' | detect type",
                result: Some(Value::test_string("not-a-number")),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let metadata = input
            .metadata()
            .map(|metadata| metadata.with_content_type(None));
        let span = call.head;
        let display_as_filesize = call.has_flag(engine_state, stack, "prefer-filesize")?;
        let val = input.into_value(call.head)?;
        let val = process(val, display_as_filesize, span)?;
        Ok(val.into_pipeline_data_with_metadata(metadata))
    }
}

// This function will check if a value matches a regular expression for a particular datatype.
// If it does, it will convert the value to that datatype.
fn process(val: Value, display_as_filesize: bool, span: Span) -> Result<Value, ShellError> {
    // step 1: convert value to string
    let val_str = val.coerce_str().unwrap_or_default();

    // step 2: bounce string up against regexes
    if BOOLEAN_RE.is_match(&val_str).unwrap_or(false) {
        let bval = val_str
            .to_lowercase()
            .parse::<bool>()
            .map_err(|_| ShellError::CantConvert {
                to_type: "string".to_string(),
                from_type: "bool".to_string(),
                span,
                help: Some(format!(
                    r#""{val_str}" does not represent a valid boolean value"#
                )),
            })?;

        Ok(Value::bool(bval, span))
    } else if FLOAT_RE.is_match(&val_str).unwrap_or(false) {
        let fval = val_str
            .parse::<f64>()
            .map_err(|_| ShellError::CantConvert {
                to_type: "float".to_string(),
                from_type: "string".to_string(),
                span,
                help: Some(format!(
                    r#""{val_str}" does not represent a valid floating point value"#
                )),
            })?;

        Ok(Value::float(fval, span))
    } else if INTEGER_RE.is_match(&val_str).unwrap_or(false) {
        let ival = val_str
            .parse::<i64>()
            .map_err(|_| ShellError::CantConvert {
                to_type: "int".to_string(),
                from_type: "string".to_string(),
                span,
                help: Some(format!(
                    r#""{val_str}" does not represent a valid integer value"#
                )),
            })?;

        if display_as_filesize {
            Ok(Value::filesize(ival, span))
        } else {
            Ok(Value::int(ival, span))
        }
    } else if INTEGER_WITH_DELIMS_RE.is_match(&val_str).unwrap_or(false) {
        let mut val_str = val_str.into_owned();
        val_str.retain(|x| !['_', ','].contains(&x));

        let ival = val_str
            .parse::<i64>()
            .map_err(|_| ShellError::CantConvert {
                to_type: "int".to_string(),
                from_type: "string".to_string(),
                span,
                help: Some(format!(
                    r#""{val_str}" does not represent a valid integer value"#
                )),
            })?;

        if display_as_filesize {
            Ok(Value::filesize(ival, span))
        } else {
            Ok(Value::int(ival, span))
        }
    } else if DATETIME_DMY_RE.is_match(&val_str).unwrap_or(false) {
        let dt = parse_date_from_string(&val_str, span).map_err(|_| ShellError::CantConvert {
            to_type: "datetime".to_string(),
            from_type: "string".to_string(),
            span,
            help: Some(format!(
                r#""{val_str}" does not represent a valid DATETIME_MDY_RE value"#
            )),
        })?;

        Ok(Value::date(dt, span))
    } else if DATETIME_YMD_RE.is_match(&val_str).unwrap_or(false) {
        let dt = parse_date_from_string(&val_str, span).map_err(|_| ShellError::CantConvert {
            to_type: "datetime".to_string(),
            from_type: "string".to_string(),
            span,
            help: Some(format!(
                r#""{val_str}" does not represent a valid DATETIME_YMD_RE value"#
            )),
        })?;

        Ok(Value::date(dt, span))
    } else if DATETIME_YMDZ_RE.is_match(&val_str).unwrap_or(false) {
        let dt = parse_date_from_string(&val_str, span).map_err(|_| ShellError::CantConvert {
            to_type: "datetime".to_string(),
            from_type: "string".to_string(),
            span,
            help: Some(format!(
                r#""{val_str}" does not represent a valid DATETIME_YMDZ_RE value"#
            )),
        })?;

        Ok(Value::date(dt, span))
    } else {
        // If we don't know what it is, just return whatever it was passed in as
        Ok(val)
    }
}

// region: datatype regexes
const DATETIME_DMY_PATTERN: &str = r#"(?x)
        ^
        ['"]?                        # optional quotes
        (?:\d{1,2})                  # day
        [-/]                         # separator
        (?P<month>[01]?\d{1})        # month
        [-/]                         # separator
        (?:\d{4,})                   # year
        (?:
            [T\ ]                    # separator
            (?:\d{2})                # hour
            :?                       # separator
            (?:\d{2})                # minute
            (?:
                :?                   # separator
                (?:\d{2})            # second
                (?:
                    \.(?:\d{1,9})    # subsecond
                )?
            )?
        )?
        ['"]?                        # optional quotes
        $
        "#;

static DATETIME_DMY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(DATETIME_DMY_PATTERN).expect("datetime_dmy_pattern should be valid")
});
const DATETIME_YMD_PATTERN: &str = r#"(?x)
        ^
        ['"]?                      # optional quotes
        (?:\d{4,})                 # year
        [-/]                       # separator
        (?P<month>[01]?\d{1})      # month
        [-/]                       # separator
        (?:\d{1,2})                # day
        (?:
            [T\ ]                  # separator
            (?:\d{2})              # hour
            :?                     # separator
            (?:\d{2})              # minute
            (?:
                :?                 # separator
                (?:\d{2})          # seconds
                (?:
                    \.(?:\d{1,9})  # subsecond
                )?
            )?
        )?
        ['"]?                      # optional quotes
        $
        "#;
static DATETIME_YMD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(DATETIME_YMD_PATTERN).expect("datetime_ymd_pattern should be valid")
});
//2023-03-24 16:44:17.865147299 -05:00
const DATETIME_YMDZ_PATTERN: &str = r#"(?x)
        ^
        ['"]?                  # optional quotes
        (?:\d{4,})             # year
        [-/]                   # separator
        (?P<month>[01]?\d{1})  # month
        [-/]                   # separator
        (?:\d{1,2})            # day
        [T\ ]                  # separator
        (?:\d{2})              # hour
        :?                     # separator
        (?:\d{2})              # minute
        (?:
            :?                 # separator
            (?:\d{2})          # second
            (?:
                \.(?:\d{1,9})  # subsecond
            )?
        )?
        \s?                    # optional space
        (?:
            # offset (e.g. +01:00)
            [+-](?:\d{2})
            :?
            (?:\d{2})
            # or Zulu suffix
            |Z
        )
        ['"]?                  # optional quotes
        $
        "#;
static DATETIME_YMDZ_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(DATETIME_YMDZ_PATTERN).expect("datetime_ymdz_pattern should be valid")
});

static FLOAT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*[-+]?((\d*\.\d+)([eE][-+]?\d+)?|inf|NaN|(\d+)[eE][-+]?\d+|\d+\.)$")
        .expect("float pattern should be valid")
});

static INTEGER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*-?(\d+)$").expect("integer pattern should be valid"));

static INTEGER_WITH_DELIMS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*-?(\d{1,3}([,_]\d{3})+)$")
        .expect("integer with delimiters pattern should be valid")
});

static BOOLEAN_RE: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"^\s*(true)$|^(false)$")
        .case_insensitive(true)
        .build()
        .expect("boolean pattern should be valid")
});
// endregion:

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(DetectType)
    }

    #[test]
    fn test_float_parse() {
        // The regex should work on all these but nushell's float parser is more strict
        assert!(FLOAT_RE.is_match("0.1").unwrap());
        assert!(FLOAT_RE.is_match("3.0").unwrap());
        assert!(FLOAT_RE.is_match("3.00001").unwrap());
        assert!(FLOAT_RE.is_match("-9.9990e-003").unwrap());
        assert!(FLOAT_RE.is_match("9.9990e+003").unwrap());
        assert!(FLOAT_RE.is_match("9.9990E+003").unwrap());
        assert!(FLOAT_RE.is_match("9.9990E+003").unwrap());
        assert!(FLOAT_RE.is_match(".5").unwrap());
        assert!(FLOAT_RE.is_match("2.5E-10").unwrap());
        assert!(FLOAT_RE.is_match("2.5e10").unwrap());
        assert!(FLOAT_RE.is_match("NaN").unwrap());
        assert!(FLOAT_RE.is_match("-NaN").unwrap());
        assert!(FLOAT_RE.is_match("-inf").unwrap());
        assert!(FLOAT_RE.is_match("inf").unwrap());
        assert!(FLOAT_RE.is_match("-7e-05").unwrap());
        assert!(FLOAT_RE.is_match("7e-05").unwrap());
        assert!(FLOAT_RE.is_match("+7e+05").unwrap());
    }

    #[test]
    fn test_int_parse() {
        assert!(INTEGER_RE.is_match("0").unwrap());
        assert!(INTEGER_RE.is_match("1").unwrap());
        assert!(INTEGER_RE.is_match("10").unwrap());
        assert!(INTEGER_RE.is_match("100").unwrap());
        assert!(INTEGER_RE.is_match("1000").unwrap());
        assert!(INTEGER_RE.is_match("10000").unwrap());
        assert!(INTEGER_RE.is_match("100000").unwrap());
        assert!(INTEGER_RE.is_match("1000000").unwrap());
        assert!(INTEGER_RE.is_match("10000000").unwrap());
        assert!(INTEGER_RE.is_match("100000000").unwrap());
        assert!(INTEGER_RE.is_match("1000000000").unwrap());
        assert!(INTEGER_RE.is_match("10000000000").unwrap());
        assert!(INTEGER_RE.is_match("100000000000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("1_000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("10_000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("100_000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("1_000_000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("10_000_000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("100_000_000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("1_000_000_000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("10_000_000_000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("100_000_000_000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("1,000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("10,000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("100,000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("1,000,000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("10,000,000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("100,000,000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("1,000,000,000").unwrap());
        assert!(INTEGER_WITH_DELIMS_RE.is_match("10,000,000,000").unwrap());
    }

    #[test]
    fn test_bool_parse() {
        assert!(BOOLEAN_RE.is_match("true").unwrap());
        assert!(BOOLEAN_RE.is_match("false").unwrap());
        assert!(!BOOLEAN_RE.is_match("1").unwrap());
        assert!(!BOOLEAN_RE.is_match("0").unwrap());
    }

    #[test]
    fn test_datetime_ymdz_pattern() {
        assert!(DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00Z").unwrap());
        assert!(
            DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00.123456789Z")
                .unwrap()
        );
        assert!(
            DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00+01:00")
                .unwrap()
        );
        assert!(
            DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00.123456789+01:00")
                .unwrap()
        );
        assert!(
            DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00-01:00")
                .unwrap()
        );
        assert!(
            DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00.123456789-01:00")
                .unwrap()
        );
        assert!(DATETIME_YMDZ_RE.is_match("'2022-01-01T00:00:00Z'").unwrap());

        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00").unwrap());
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00.").unwrap());
        assert!(
            !DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00.123456789")
                .unwrap()
        );
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00+01").unwrap());
        assert!(
            !DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00+01:0")
                .unwrap()
        );
        assert!(
            !DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00+1:00")
                .unwrap()
        );
        assert!(
            !DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00.123456789+01")
                .unwrap()
        );
        assert!(
            !DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00.123456789+01:0")
                .unwrap()
        );
        assert!(
            !DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00.123456789+1:00")
                .unwrap()
        );
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00-01").unwrap());
        assert!(
            !DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00-01:0")
                .unwrap()
        );
        assert!(
            !DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00-1:00")
                .unwrap()
        );
        assert!(
            !DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00.123456789-01")
                .unwrap()
        );
        assert!(
            !DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00.123456789-01:0")
                .unwrap()
        );
        assert!(
            !DATETIME_YMDZ_RE
                .is_match("2022-01-01T00:00:00.123456789-1:00")
                .unwrap()
        );
    }

    #[test]
    fn test_datetime_ymd_pattern() {
        assert!(DATETIME_YMD_RE.is_match("2022-01-01").unwrap());
        assert!(DATETIME_YMD_RE.is_match("2022/01/01").unwrap());
        assert!(DATETIME_YMD_RE.is_match("2022-01-01T00:00:00").unwrap());
        assert!(
            DATETIME_YMD_RE
                .is_match("2022-01-01T00:00:00.000000000")
                .unwrap()
        );
        assert!(DATETIME_YMD_RE.is_match("'2022-01-01'").unwrap());

        // The regex isn't this specific, but it would be nice if it were
        // assert!(!DATETIME_YMD_RE.is_match("2022-13-01").unwrap());
        // assert!(!DATETIME_YMD_RE.is_match("2022-01-32").unwrap());
        // assert!(!DATETIME_YMD_RE.is_match("2022-01-01T24:00:00").unwrap());
        // assert!(!DATETIME_YMD_RE.is_match("2022-01-01T00:60:00").unwrap());
        // assert!(!DATETIME_YMD_RE.is_match("2022-01-01T00:00:60").unwrap());
        assert!(
            !DATETIME_YMD_RE
                .is_match("2022-01-01T00:00:00.0000000000")
                .unwrap()
        );
    }

    #[test]
    fn test_datetime_dmy_pattern() {
        assert!(DATETIME_DMY_RE.is_match("31-12-2021").unwrap());
        assert!(DATETIME_DMY_RE.is_match("01/01/2022").unwrap());
        assert!(DATETIME_DMY_RE.is_match("15-06-2023 12:30").unwrap());
        assert!(!DATETIME_DMY_RE.is_match("2022-13-01").unwrap());
        assert!(!DATETIME_DMY_RE.is_match("2022-01-32").unwrap());
        assert!(!DATETIME_DMY_RE.is_match("2022-01-01 24:00").unwrap());
    }
}
