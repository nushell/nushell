use crate::parse_date_from_string;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, PipelineData, PipelineIterator, ShellError,
    Signature, Span, SyntaxShape, Type, Value,
};
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use std::{collections::HashSet, iter::FromIterator};

#[derive(Clone)]
pub struct IntoValue;

impl Command for IntoValue {
    fn name(&self) -> &str {
        "into value"
    }

    fn signature(&self) -> Signature {
        Signature::build("into value")
            .input_output_types(vec![(Type::Table(vec![]), Type::Table(vec![]))])
            .named(
                "columns",
                SyntaxShape::Table(vec![]),
                "list of columns to update",
                Some('c'),
            )
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

        // the columns to update
        let columns: Option<Value> = call.get_flag(&engine_state, stack, "columns")?;
        let columns: Option<HashSet<String>> = match columns {
            Some(val) => {
                let cols = val
                    .as_list()?
                    .iter()
                    .map(|val| val.as_string())
                    .collect::<Result<Vec<String>, ShellError>>()?;
                Some(HashSet::from_iter(cols))
            }
            None => None,
        };

        Ok(UpdateCellIterator {
            input: input.into_iter(),
            columns,
            span,
        }
        .into_pipeline_data(ctrlc)
        .set_metadata(metadata))
    }
}

struct UpdateCellIterator {
    input: PipelineIterator,
    columns: Option<HashSet<String>>,
    span: Span,
}

impl Iterator for UpdateCellIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.input.next() {
            Some(val) => {
                if let Some(ref cols) = self.columns {
                    if !val.columns().iter().any(|c| cols.contains(c)) {
                        return Some(val);
                    }
                }

                let span = val.span();
                match val {
                    Value::Record { val, .. } => Some(Value::record(
                        val.into_iter()
                            .map(|(col, val)| match &self.columns {
                                Some(cols) if !cols.contains(&col) => (col, val),
                                _ => (
                                    col,
                                    match process_cell(val, span) {
                                        Ok(val) => val,
                                        Err(err) => Value::error(err, span),
                                    },
                                ),
                            })
                            .collect(),
                        span,
                    )),
                    val => match process_cell(val, self.span) {
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
fn process_cell(val: Value, span: Span) -> Result<Value, ShellError> {
    // step 1: convert value to string
    let val_str = val.as_string().unwrap_or_default();

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

        Ok(Value::int(ival, span))
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

    // val
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

static DATETIME_DMY_RE: Lazy<Regex> = Lazy::new(|| Regex::new(DATETIME_DMY_PATTERN).unwrap());
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
static DATETIME_YMD_RE: Lazy<Regex> = Lazy::new(|| Regex::new(DATETIME_YMD_PATTERN).unwrap());
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
static DATETIME_YMDZ_RE: Lazy<Regex> = Lazy::new(|| Regex::new(DATETIME_YMDZ_PATTERN).unwrap());

static FLOAT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*[-+]?((\d*\.\d+)([eE][-+]?\d+)?|inf|NaN|(\d+)[eE][-+]?\d+|\d+\.)$").unwrap()
});

static INTEGER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*-?(\d+)$").unwrap());

static BOOLEAN_RE: Lazy<Regex> = Lazy::new(|| {
    RegexBuilder::new(r"^\s*(true)$|^(false)$")
        .case_insensitive(true)
        .build()
        .unwrap()
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
}
