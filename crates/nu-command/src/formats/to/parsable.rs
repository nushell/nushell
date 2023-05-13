use fancy_regex::Regex;
use nu_engine::get_columns;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Config, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type,
    Value,
};
use once_cell::sync::Lazy;

struct Options<'a> {
    sep: &'a str,        // separator string
    _allow_ragged: bool, // Allow ragged tables -- long way to go to fix: missing cells; out-of-order cells;
    config: &'a Config,  // EngineState.config for formatting
}

#[derive(Clone)]
pub struct ToParsable;

// credit: code gleefully adapted from `to nuon`

impl Command for ToParsable {
    fn name(&self) -> &str {
        "to parsable"
    }

    fn signature(&self) -> Signature {
        Signature::build("to parsable")
            .input_output_types(vec![(Type::Any, Type::String)])
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Converts objects into strings that are parsable by Nushell and colorable by nu-highlight."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let options = Options {
            sep: ", ",
            _allow_ragged: false,
            config: &engine_state.config,
        };
        let span = call.head;
        let value = input.into_value(span);

        match value_to_string(&value, span, &options) {
            Ok(parsable_string) => Ok(Value::String {
                val: parsable_string,
                span,
            }
            .into_pipeline_data()),
            _ => Ok(Value::Error {
                error: Box::new(ShellError::CantConvert {
                    to_type: "parsable string".into(),
                    from_type: value.get_type().to_string(),
                    span,
                    help: None,
                }),
            }
            .into_pipeline_data()),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Ensures simple types do not print with unquoted embedded blanks or other NU delimiters",
                example: r#"{date: 2001-05-11 string:'embedded: delimiter' duration:6.35day filesize: 7b} | to parsable | nu-highlight"#,
                result: Some(Value::test_string(r#"{date: 2001-05-11T00:00:00+00:00, string: 'embedded: delimiter', duration: 548640000000000ns, filesize: 7b}"#)),
            },
            Example {
                description: "Without this help, simple types could produce unparsable text and cause mis-highlighting.",
                example: r#"> $"({date: 2001-05-11 string:'embedded: delimiter' duration:6.35day filesize: 7b})" | nu-highlight"#,
                result: Some(Value::test_string(r#"{date: Fri, 11 May 2001 00:00:00 +0000 (22 years ago), string: embedded: delimiter, duration: 6day 8hr 24min, filesize: 7 B}"#)),
            },

//            Example {
//                description: "Reassembles table from raw list of records, \nbut only if all rows have same columns, and in same order"
//            }
        ]
    }
}

fn value_to_string(v: &Value, span: Span, options: &Options) -> Result<String, ShellError> {
    match v {
        // Propagate existing errors
        Value::Error { error } => Err(*error.clone()),

        // Simple values and compound values range and record can be handled by Value::to_string_parsable()

        Value::Binary {..}      |
        Value::Bool {..}         |
        Value::Date {..}         |
        Value::Duration {..}     |
        Value::Filesize {..}     |
        Value::Float {..}        |
        Value::Int {..}          |
        Value::Nothing  {..}     |
        Value::Range {..}        |
        Value::Record {..}       |
        Value::LazyRecord {..}   |
        // All strings outside data structures are quoted because they are in 'command position'
        // (could be mistaken for commands by the Nu parser)
        Value::String {..}       => {
            Ok(v.into_string_parsable(options.sep, options.config))
        }

        // But if outer value is a list, check for list of records (i.e, a table)

        Value::List { vals, .. } => {
            let headers = get_columns(vals);
            if !headers.is_empty() && vals.iter().all(|x| x.columns() == headers) {
                // Table output

                let headers: Vec<String> = headers
                    .iter()
                    .map(|string| {
                        if needs_quotes(string) {
                            format!(r#"'{string}'"#)
                        } else {
                            string.to_string()
                        }
                    })
                    .collect();
                let headers_output = headers.join(options.sep);

                let mut table_output = vec![];
                for val in vals {
                    let mut row = vec![];

                    if let Value::Record { vals, .. } = val {
                        //FIXME - what if elements are not in same order in each row?
                        for rval in vals {
                            // hopeful optimization -- if element of record is not List, just stringify it.
                            // only if element is list could it possibly be a table and we have to recurse to format it correctly.
                            if let Value::List{ ..}= rval  {
                            row.push(value_to_string(rval, span, options)?);
                            } else {
                                row.push(rval.into_string_parsable(options.sep, options.config))
                            }
                        }
                    }

                    table_output.push(format!("[{}]", row.join(options.sep)));
                }

                Ok(format!(r#"[[{headers_output}]; {}]"#,
                    table_output.join(options.sep)))
            } else {
                // a list, but not a table
                Ok(v.into_string_parsable(options.sep, options.config))
            }
        }

        _ => Err(ShellError::UnsupportedInput(
            format!("{} not parsable-compatible", v.get_type()),
            "value originates from here".into(),
            span,
            v.expect_span(),
        )),
    }
}

// This hits, in order:
// • Any character of []:`{}#'";()|$,
// • Any digit (\d)
// • Any whitespace (\s)
// • Case-insensitive sign-insensitive float "keywords" inf, infinity and nan.
static NEEDS_QUOTES_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"[\[\]:`\{\}#'";\(\)\|\$,\d\s]|(?i)^[+\-]?(inf(inity)?|nan)$"#)
        .expect("internal error: NEEDS_QUOTES_REGEX didn't compile")
});

fn needs_quotes(string: &str) -> bool {
    if string.is_empty() {
        return true;
    }
    // These are case-sensitive keywords
    match string {
        // `true`/`false`/`null` are active keywords in JSON and NUON
        // `&&` is denied by the nu parser for diagnostics reasons
        // (https://github.com/nushell/nushell/pull/7241)
        // TODO: remove the extra check in the nuon codepath
        "true" | "false" | "null" | "&&" => return true,
        _ => (),
    };
    // All other cases are handled here
    NEEDS_QUOTES_REGEX.is_match(string).unwrap_or(false)
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::ToParsable;
        use crate::test_examples;
        test_examples(ToParsable {})
    }
}
