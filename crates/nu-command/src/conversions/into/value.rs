use crate::parse_date_from_string;
use nu_engine::command_prelude::*;
use nu_protocol::PipelineIterator;
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use std::collections::HashSet;

#[derive(Clone)]
pub struct IntoValue;

impl Command for IntoValue {
    fn name(&self) -> &str {
        "into value"
    }

    fn signature(&self) -> Signature {
        Signature::build("into value")
            .input_output_types(vec![(Type::table(), Type::table())])
            .named(
                "columns",
                SyntaxShape::Table(vec![]),
                "list of columns to update",
                Some('c'),
            )
            .switch(
                "prefer-filesizes",
                "For ints display them as human-readable file sizes",
                Some('f'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Infer nushell datatype for each cell."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Infer Nushell values for each cell.",
                example: "$table | into value",
                result: None,
            },
            Example {
                description: "Infer Nushell values for each cell in the given columns.",
                example: "$table | into value -c [column1, column5]",
                result: None,
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
        let engine_state = engine_state.clone();
        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        let span = call.head;
        let display_as_filesizes = call.has_flag(&engine_state, stack, "prefer-filesizes")?;

        // the columns to update
        let columns: Option<Value> = call.get_flag(&engine_state, stack, "columns")?;
        let columns: Option<HashSet<String>> = match columns {
            Some(val) => Some(
                val.into_list()?
                    .into_iter()
                    .map(Value::coerce_into_string)
                    .collect::<Result<HashSet<String>, ShellError>>()?,
            ),
            None => None,
        };

        Ok(UpdateCellIterator {
            input: input.into_iter(),
            columns,
            display_as_filesizes,
            span,
        }
        .into_pipeline_data(ctrlc)
        .set_metadata(metadata))
    }
}

struct UpdateCellIterator {
    input: PipelineIterator,
    columns: Option<HashSet<String>>,
    display_as_filesizes: bool,
    span: Span,
}

impl Iterator for UpdateCellIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.input.next() {
            Some(val) => {
                if let Some(ref cols) = self.columns {
                    if !val.columns().any(|c| cols.contains(c)) {
                        return Some(val);
                    }
                }

                let span = val.span();
                match val {
                    Value::Record { val, .. } => Some(Value::record(
                        val.into_owned()
                            .into_iter()
                            .map(|(col, val)| match &self.columns {
                                Some(cols) if !cols.contains(&col) => (col, val),
                                _ => (
                                    col,
                                    match process_cell(val, self.display_as_filesizes, span) {
                                        Ok(val) => val,
                                        Err(err) => Value::error(err, span),
                                    },
                                ),
                            })
                            .collect(),
                        span,
                    )),
                    val => match process_cell(val, self.display_as_filesizes, self.span) {
                        Ok(val) => Some(val),
                        Err(err) => Some(Value::error(err, self.span)),
                    },
                }
            }
            None => None,
        }
    }
}

// This function will check each cell to see if it matches a regular expression
// for a particular datatype. If it does, it will convert the cell to that datatype.
fn process_cell(val: Value, display_as_filesizes: bool, span: Span) -> Result<Value, ShellError> {
    // step 1: convert value to string
    let val_str = val.coerce_str().unwrap_or_default();

    // step 2: bounce string up against regexes
    if BOOLEAN_RE.is_match(&val_str) {
        let bval = val_str
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
    } else if FLOAT_RE.is_match(&val_str) {
        let fval = val_str
            .parse::<f64>()
            .map_err(|_| ShellError::CantConvert {
                to_type: "string".to_string(),
                from_type: "float".to_string(),
                span,
                help: Some(format!(
                    r#""{val_str}" does not represent a valid floating point value"#
                )),
            })?;

        Ok(Value::float(fval, span))
    } else if INTEGER_RE.is_match(&val_str) {
        let ival = val_str
            .parse::<i64>()
            .map_err(|_| ShellError::CantConvert {
                to_type: "string".to_string(),
                from_type: "int".to_string(),
                span,
                help: Some(format!(
                    r#""{val_str}" does not represent a valid integer value"#
                )),
            })?;

        if display_as_filesizes {
            Ok(Value::filesize(ival, span))
        } else {
            Ok(Value::int(ival, span))
        }
    } else if INTEGER_WITH_DELIMS_RE.is_match(&val_str) {
        let mut val_str = val_str.into_owned();
        val_str.retain(|x| !['_', ','].contains(&x));

        let ival = val_str
            .parse::<i64>()
            .map_err(|_| ShellError::CantConvert {
                to_type: "string".to_string(),
                from_type: "int".to_string(),
                span,
                help: Some(format!(
                    r#""{val_str}" does not represent a valid integer value"#
                )),
            })?;

        if display_as_filesizes {
            Ok(Value::filesize(ival, span))
        } else {
            Ok(Value::int(ival, span))
        }
    } else if DATETIME_DMY_RE.is_match(&val_str) {
        let dt = parse_date_from_string(&val_str, span).map_err(|_| ShellError::CantConvert {
            to_type: "date".to_string(),
            from_type: "string".to_string(),
            span,
            help: Some(format!(
                r#""{val_str}" does not represent a valid DATETIME_MDY_RE value"#
            )),
        })?;

        Ok(Value::date(dt, span))
    } else if DATETIME_YMD_RE.is_match(&val_str) {
        let dt = parse_date_from_string(&val_str, span).map_err(|_| ShellError::CantConvert {
            to_type: "date".to_string(),
            from_type: "string".to_string(),
            span,
            help: Some(format!(
                r#""{val_str}" does not represent a valid DATETIME_YMD_RE value"#
            )),
        })?;

        Ok(Value::date(dt, span))
    } else if DATETIME_YMDZ_RE.is_match(&val_str) {
        let dt = parse_date_from_string(&val_str, span).map_err(|_| ShellError::CantConvert {
            to_type: "date".to_string(),
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

static DATETIME_DMY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(DATETIME_DMY_PATTERN).expect("datetime_dmy_pattern should be valid"));
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
static DATETIME_YMD_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(DATETIME_YMD_PATTERN).expect("datetime_ymd_pattern should be valid"));
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
static DATETIME_YMDZ_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(DATETIME_YMDZ_PATTERN).expect("datetime_ymdz_pattern should be valid"));

static FLOAT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*[-+]?((\d*\.\d+)([eE][-+]?\d+)?|inf|NaN|(\d+)[eE][-+]?\d+|\d+\.)$")
        .expect("float pattern should be valid")
});

static INTEGER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*-?(\d+)$").expect("integer pattern should be valid"));

static INTEGER_WITH_DELIMS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*-?(\d{1,3}([,_]\d{3})+)$")
        .expect("integer with delimiters pattern should be valid")
});

static BOOLEAN_RE: Lazy<Regex> = Lazy::new(|| {
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

        test_examples(IntoValue {})
    }

    #[test]
    fn test_float_parse() {
        // The regex should work on all these but nushell's float parser is more strict
        assert!(FLOAT_RE.is_match("0.1"));
        assert!(FLOAT_RE.is_match("3.0"));
        assert!(FLOAT_RE.is_match("3.00001"));
        assert!(FLOAT_RE.is_match("-9.9990e-003"));
        assert!(FLOAT_RE.is_match("9.9990e+003"));
        assert!(FLOAT_RE.is_match("9.9990E+003"));
        assert!(FLOAT_RE.is_match("9.9990E+003"));
        assert!(FLOAT_RE.is_match(".5"));
        assert!(FLOAT_RE.is_match("2.5E-10"));
        assert!(FLOAT_RE.is_match("2.5e10"));
        assert!(FLOAT_RE.is_match("NaN"));
        assert!(FLOAT_RE.is_match("-NaN"));
        assert!(FLOAT_RE.is_match("-inf"));
        assert!(FLOAT_RE.is_match("inf"));
        assert!(FLOAT_RE.is_match("-7e-05"));
        assert!(FLOAT_RE.is_match("7e-05"));
        assert!(FLOAT_RE.is_match("+7e+05"));
    }

    #[test]
    fn test_int_parse() {
        assert!(INTEGER_RE.is_match("0"));
        assert!(INTEGER_RE.is_match("1"));
        assert!(INTEGER_RE.is_match("10"));
        assert!(INTEGER_RE.is_match("100"));
        assert!(INTEGER_RE.is_match("1000"));
        assert!(INTEGER_RE.is_match("10000"));
        assert!(INTEGER_RE.is_match("100000"));
        assert!(INTEGER_RE.is_match("1000000"));
        assert!(INTEGER_RE.is_match("10000000"));
        assert!(INTEGER_RE.is_match("100000000"));
        assert!(INTEGER_RE.is_match("1000000000"));
        assert!(INTEGER_RE.is_match("10000000000"));
        assert!(INTEGER_RE.is_match("100000000000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("1_000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("10_000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("100_000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("1_000_000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("10_000_000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("100_000_000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("1_000_000_000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("10_000_000_000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("100_000_000_000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("1,000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("10,000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("100,000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("1,000,000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("10,000,000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("100,000,000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("1,000,000,000"));
        assert!(INTEGER_WITH_DELIMS_RE.is_match("10,000,000,000"));
    }

    #[test]
    fn test_bool_parse() {
        assert!(BOOLEAN_RE.is_match("true"));
        assert!(BOOLEAN_RE.is_match("false"));
        assert!(!BOOLEAN_RE.is_match("1"));
        assert!(!BOOLEAN_RE.is_match("0"));
    }

    #[test]
    fn test_datetime_ymdz_pattern() {
        assert!(DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00Z"));
        assert!(DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00.123456789Z"));
        assert!(DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00+01:00"));
        assert!(DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00.123456789+01:00"));
        assert!(DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00-01:00"));
        assert!(DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00.123456789-01:00"));
        assert!(DATETIME_YMDZ_RE.is_match("'2022-01-01T00:00:00Z'"));

        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00"));
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00."));
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00.123456789"));
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00+01"));
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00+01:0"));
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00+1:00"));
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00.123456789+01"));
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00.123456789+01:0"));
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00.123456789+1:00"));
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00-01"));
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00-01:0"));
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00-1:00"));
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00.123456789-01"));
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00.123456789-01:0"));
        assert!(!DATETIME_YMDZ_RE.is_match("2022-01-01T00:00:00.123456789-1:00"));
    }

    #[test]
    fn test_datetime_ymd_pattern() {
        assert!(DATETIME_YMD_RE.is_match("2022-01-01"));
        assert!(DATETIME_YMD_RE.is_match("2022/01/01"));
        assert!(DATETIME_YMD_RE.is_match("2022-01-01T00:00:00"));
        assert!(DATETIME_YMD_RE.is_match("2022-01-01T00:00:00.000000000"));
        assert!(DATETIME_YMD_RE.is_match("'2022-01-01'"));

        // The regex isn't this specific, but it would be nice if it were
        // assert!(!DATETIME_YMD_RE.is_match("2022-13-01"));
        // assert!(!DATETIME_YMD_RE.is_match("2022-01-32"));
        // assert!(!DATETIME_YMD_RE.is_match("2022-01-01T24:00:00"));
        // assert!(!DATETIME_YMD_RE.is_match("2022-01-01T00:60:00"));
        // assert!(!DATETIME_YMD_RE.is_match("2022-01-01T00:00:60"));
        assert!(!DATETIME_YMD_RE.is_match("2022-01-01T00:00:00.0000000000"));
    }

    #[test]
    fn test_datetime_dmy_pattern() {
        assert!(DATETIME_DMY_RE.is_match("31-12-2021"));
        assert!(DATETIME_DMY_RE.is_match("01/01/2022"));
        assert!(DATETIME_DMY_RE.is_match("15-06-2023 12:30"));
        assert!(!DATETIME_DMY_RE.is_match("2022-13-01"));
        assert!(!DATETIME_DMY_RE.is_match("2022-01-32"));
        assert!(!DATETIME_DMY_RE.is_match("2022-01-01 24:00"));
    }
}
