use crate::parse_date_from_string;
use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::{Block, Call};
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    PipelineIterator, Record, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use std::collections::HashSet;
use std::iter::FromIterator;

#[derive(Clone)]
pub struct IntoValue;

impl Command for IntoValue {
    fn name(&self) -> &str {
        "into value"
    }

    fn signature(&self) -> Signature {
        Signature::build("into value")
            .input_output_types(vec![(Type::Table(vec![]), Type::Table(vec![]))])
            // .required(
            //     "closure",
            //     SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
            //     "the closure to run an update for each cell",
            // )
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
                description: "Update the zero value cells to empty strings.",
                example: "",
                result: None,
            },
            Example {
                description: "Update the zero value cells to empty strings in 2 last columns.",
                example: "",
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
        // the block to run on each cell
        let engine_state = engine_state.clone();
        // let block: Closure = call.req(&engine_state, stack, 0)?;
        // let mut stack = stack.captures_to_stack(&block.captures);
        // let orig_env_vars = stack.env_vars.clone();
        // let orig_env_hidden = stack.env_hidden.clone();

        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        // let block: Block = engine_state.get_block(block.block_id).clone();

        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        let span = call.head;

        // stack.with_env(&orig_env_vars, &orig_env_hidden);

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
            engine_state,
            // stack,
            // block,
            columns,
            redirect_stdout,
            redirect_stderr,
            span,
        }
        .into_pipeline_data(ctrlc)
        .set_metadata(metadata))
    }
}

struct UpdateCellIterator {
    input: PipelineIterator,
    columns: Option<HashSet<String>>,
    engine_state: EngineState,
    // stack: Stack,
    // block: Block,
    redirect_stdout: bool,
    redirect_stderr: bool,
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

                match val {
                    Value::Record { val, span } => Some(Value::record(
                        val.into_iter()
                            .map(|(col, val)| match &self.columns {
                                Some(cols) if !cols.contains(&col) => (col, val),
                                _ => (
                                    col,
                                    process_cell(
                                        val,
                                        &self.engine_state,
                                        // &mut self.stack,
                                        // &self.block,
                                        self.redirect_stdout,
                                        self.redirect_stderr,
                                        span,
                                    ),
                                ),
                            })
                            .collect(),
                        span,
                    )),
                    val => Some(process_cell(
                        val,
                        &self.engine_state,
                        // &mut self.stack,
                        // &self.block,
                        self.redirect_stdout,
                        self.redirect_stderr,
                        self.span,
                    )),
                }
            }
            None => None,
        }
    }
}

fn process_cell(
    val: Value,
    engine_state: &EngineState,
    // stack: &mut Stack,
    // block: &Block,
    redirect_stdout: bool,
    redirect_stderr: bool,
    span: Span,
) -> Value {
    // if let Some(var) = block.signature.get_positional(0) {
    //     if let Some(var_id) = &var.var_id {
    //         stack.add_var(*var_id, val.clone());
    //     }
    // }
    // match eval_block(
    //     engine_state,
    //     stack,
    //     block,
    //     val.into_pipeline_data(),
    //     redirect_stdout,
    //     redirect_stderr,
    // ) {
    //     Ok(pd) => pd.into_value(span),
    //     Err(e) => Value::Error {
    //         error: Box::new(e),
    //         span,
    //     },
    // }

    // step 1: convert value to string
    let val_str = val.as_string().unwrap_or_default();
    // step 2: bounce string up against regexes
    if BOOLEAN_RE.is_match(&val_str) {
        Value::bool(val_str.parse::<bool>().unwrap(), span)
    } else if FLOAT_RE.is_match(&val_str) {
        Value::float(val_str.parse::<f64>().unwrap(), span)
    } else if INTEGER_RE.is_match(&val_str) {
        Value::int(val_str.parse::<i64>().unwrap(), span)
    } else if DATETIME_DMY_RE.is_match(&val_str) {
        let dt = parse_date_from_string(&val_str, span).unwrap();
        Value::date(dt, span)
    } else if DATETIME_YMD_RE.is_match(&val_str) {
        let dt = parse_date_from_string(&val_str, span).unwrap();
        Value::date(dt, span)
    } else if DATETIME_YMDZ_RE.is_match(&val_str) {
        let dt = parse_date_from_string(&val_str, span).unwrap();
        Value::date(dt, span)
    } else {
        // If we don't know what it is, just return whatever it was passed in as
        val
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

// region: Patterns
// // Patterns are grouped together by order of month, day, year. This is to prevent
// // parsing different orders of dates in a single column.

// pub(super) static DATE_D_M_Y: &[&str] = &[
//     "%d-%m-%Y", // 31-12-2021
//     "%d/%m/%Y", // 31/12/2021
// ];

// pub(super) static DATE_Y_M_D: &[&str] = &[
//     "%Y/%m/%d", // 2021/12/31
//     "%Y-%m-%d", // 2021-12-31
// ];

// /// NOTE: don't use single letter dates like %F
// /// polars parsers does not support them, so it will be slower
// pub(super) static DATETIME_D_M_Y: &[&str] = &[
//     // --
//     // supported by polars' parser
//     // ---
//     // 31/12/2021 24:58:01
//     "%d/%m/%Y %H:%M:%S",
//     // 31-12-2021 24:58
//     "%d-%m-%Y %H:%M",
//     // 31-12-2021 24:58:01
//     "%d-%m-%Y %H:%M:%S",
//     // 31-04-2021T02:45:55.555000000
//     // milliseconds
//     "%d-%m-%YT%H:%M:%S.%3f",
//     // microseconds
//     "%d-%m-%YT%H:%M:%S.%6f",
//     // nanoseconds
//     "%d-%m-%YT%H:%M:%S.%9f",
//     "%d/%m/%Y 00:00:00",
//     "%d-%m-%Y 00:00:00",
//     // no times
//     "%d-%m-%Y",
// ];

// /// NOTE: don't use single letter dates like %F
// /// polars parsers does not support them, so it will be slower
// pub(super) static DATETIME_Y_M_D: &[&str] = &[
//     // ---
//     // ISO8601-like, generated via the `iso8601_format_datetime` test fixture
//     // ---
//     "%Y/%m/%dT%H:%M:%S",
//     "%Y-%m-%dT%H:%M:%S",
//     "%Y/%m/%dT%H%M%S",
//     "%Y-%m-%dT%H%M%S",
//     "%Y/%m/%dT%H:%M",
//     "%Y-%m-%dT%H:%M",
//     "%Y/%m/%dT%H%M",
//     "%Y-%m-%dT%H%M",
//     "%Y/%m/%dT%H:%M:%S.%9f",
//     "%Y-%m-%dT%H:%M:%S.%9f",
//     "%Y/%m/%dT%H:%M:%S.%6f",
//     "%Y-%m-%dT%H:%M:%S.%6f",
//     "%Y/%m/%dT%H:%M:%S.%3f",
//     "%Y-%m-%dT%H:%M:%S.%3f",
//     "%Y/%m/%dT%H%M%S.%9f",
//     "%Y-%m-%dT%H%M%S.%9f",
//     "%Y/%m/%dT%H%M%S.%6f",
//     "%Y-%m-%dT%H%M%S.%6f",
//     "%Y/%m/%dT%H%M%S.%3f",
//     "%Y-%m-%dT%H%M%S.%3f",
//     "%Y/%m/%d",
//     "%Y-%m-%d",
//     "%Y/%m/%d %H:%M:%S",
//     "%Y-%m-%d %H:%M:%S",
//     "%Y/%m/%d %H%M%S",
//     "%Y-%m-%d %H%M%S",
//     "%Y/%m/%d %H:%M",
//     "%Y-%m-%d %H:%M",
//     "%Y/%m/%d %H%M",
//     "%Y-%m-%d %H%M",
//     "%Y/%m/%d %H:%M:%S.%9f",
//     "%Y-%m-%d %H:%M:%S.%9f",
//     "%Y/%m/%d %H:%M:%S.%6f",
//     "%Y-%m-%d %H:%M:%S.%6f",
//     "%Y/%m/%d %H:%M:%S.%3f",
//     "%Y-%m-%d %H:%M:%S.%3f",
//     "%Y/%m/%d %H%M%S.%9f",
//     "%Y-%m-%d %H%M%S.%9f",
//     "%Y/%m/%d %H%M%S.%6f",
//     "%Y-%m-%d %H%M%S.%6f",
//     "%Y/%m/%d %H%M%S.%3f",
//     "%Y-%m-%d %H%M%S.%3f",
//     // ---
//     // other
//     // ---
//     // we cannot know this one, because polars needs to know
//     // the length of the parsed fmt
//     // ---
//     "%FT%H:%M:%S%.f",
// ];

// pub(super) static DATETIME_Y_M_D_Z: &[&str] = &[
//     // ---
//     // ISO8601-like, generated via the `iso8601_tz_aware_format_datetime` test fixture
//     // ---
//     "%Y/%m/%dT%H:%M:%S%#z",
//     "%Y-%m-%dT%H:%M:%S%#z",
//     "%Y/%m/%dT%H%M%S%#z",
//     "%Y-%m-%dT%H%M%S%#z",
//     "%Y/%m/%dT%H:%M%#z",
//     "%Y-%m-%dT%H:%M%#z",
//     "%Y/%m/%dT%H%M%#z",
//     "%Y-%m-%dT%H%M%#z",
//     "%Y/%m/%dT%H:%M:%S.%9f%#z",
//     "%Y-%m-%dT%H:%M:%S.%9f%#z",
//     "%Y/%m/%dT%H:%M:%S.%6f%#z",
//     "%Y-%m-%dT%H:%M:%S.%6f%#z",
//     "%Y/%m/%dT%H:%M:%S.%3f%#z",
//     "%Y-%m-%dT%H:%M:%S.%3f%#z",
//     "%Y/%m/%dT%H%M%S.%9f%#z",
//     "%Y-%m-%dT%H%M%S.%9f%#z",
//     "%Y/%m/%dT%H%M%S.%6f%#z",
//     "%Y-%m-%dT%H%M%S.%6f%#z",
//     "%Y/%m/%dT%H%M%S.%3f%#z",
//     "%Y-%m-%dT%H%M%S.%3f%#z",
//     "%Y/%m/%d %H:%M:%S%#z",
//     "%Y-%m-%d %H:%M:%S%#z",
//     "%Y/%m/%d %H%M%S%#z",
//     "%Y-%m-%d %H%M%S%#z",
//     "%Y/%m/%d %H:%M%#z",
//     "%Y-%m-%d %H:%M%#z",
//     "%Y/%m/%d %H%M%#z",
//     "%Y-%m-%d %H%M%#z",
//     "%Y/%m/%d %H:%M:%S.%9f%#z",
//     "%Y-%m-%d %H:%M:%S.%9f%#z",
//     "%Y/%m/%d %H:%M:%S.%6f%#z",
//     "%Y-%m-%d %H:%M:%S.%6f%#z",
//     "%Y/%m/%d %H:%M:%S.%3f%#z",
//     "%Y-%m-%d %H:%M:%S.%3f%#z",
//     "%Y/%m/%d %H%M%S.%9f%#z",
//     "%Y-%m-%d %H%M%S.%9f%#z",
//     "%Y/%m/%d %H%M%S.%6f%#z",
//     "%Y-%m-%d %H%M%S.%6f%#z",
//     "%Y/%m/%d %H%M%S.%3f%#z",
//     "%Y-%m-%d %H%M%S.%3f%#z",
//     // other
//     "%+",
// ];

// #[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
// pub enum Pattern {
//     DateDMY,
//     DateYMD,
//     DatetimeYMD,
//     DatetimeDMY,
//     DatetimeYMDZ,
// }
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
