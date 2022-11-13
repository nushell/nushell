use fancy_regex::Regex;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, ListStream, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape,
    Type, Value,
};

#[derive(Clone)]
pub struct Parse;

impl Command for Parse {
    fn name(&self) -> &str {
        "parse"
    }

    fn usage(&self) -> &str {
        "Parse columns from string data using a simple pattern."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pattern", "match"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("parse")
            .required(
                "pattern",
                SyntaxShape::String,
                "the pattern to match. Eg) \"{foo}: {bar}\"",
            )
            .input_output_types(vec![(Type::String, Type::Table(vec![]))])
            .switch("regex", "use full regex syntax for patterns", Some('r'))
            .category(Category::Strings)
    }

    fn examples(&self) -> Vec<Example> {
        let result = Value::List {
            vals: vec![Value::Record {
                cols: vec!["foo".to_string(), "bar".to_string()],
                vals: vec![Value::test_string("hi"), Value::test_string("there")],
                span: Span::test_data(),
            }],
            span: Span::test_data(),
        };

        vec![
            Example {
                description: "Parse a string into two named columns",
                example: "echo \"hi there\" | parse \"{foo} {bar}\"",
                result: Some(result.clone()),
            },
            Example {
                description: "Parse a string using regex pattern",
                example: "echo \"hi there\" | parse -r '(?P<foo>\\w+) (?P<bar>\\w+)'",
                result: Some(result),
            },
            Example {
                description: "Parse a string using fancy-regex named capture group pattern",
                example: "echo \"foo bar.\" | parse -r '\\s*(?<name>\\w+)(?=\\.)'",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["name".to_string()],
                        vals: vec![Value::test_string("bar")],
                        span: Span::test_data()
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Parse a string using fancy-regex capture group pattern",
                example: "echo \"foo! bar.\" | parse -r '(\\w+)(?=\\.)|(\\w+)(?=!)'",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["Capture1".to_string(), "Capture2".to_string()],
                            vals: vec![Value::test_string(""), Value::test_string("foo")],
                            span: Span::test_data()
                        },
                        Value::Record {
                            cols: vec!["Capture1".to_string(), "Capture2".to_string()],
                            vals: vec![Value::test_string("bar"), Value::test_string("")],
                            span: Span::test_data(),
                        }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Parse a string using fancy-regex look behind pattern",
                example: "echo \" @another(foo bar)   \" | parse -r '\\s*(?<=[() ])(@\\w+)(\\([^)]*\\))?\\s*'",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["Capture1".to_string(), "Capture2".to_string()],
                        vals: vec![Value::test_string("@another"), Value::test_string("(foo bar)")],
                        span: Span::test_data()
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Parse a string using fancy-regex look ahead atomic group pattern",
                example: "echo \"abcd\" | parse -r '^a(bc(?=d)|b)cd$'",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["Capture1".to_string()],
                        vals: vec![Value::test_string("b")],
                        span: Span::test_data()
                    }],
                    span: Span::test_data(),
                }),
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
        operate(engine_state, stack, call, input)
    }
}

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let pattern: Spanned<String> = call.req(engine_state, stack, 0)?;
    let regex: bool = call.has_flag("regex");
    let ctrlc = engine_state.ctrlc.clone();

    let pattern_item = pattern.item;
    let pattern_span = pattern.span;

    let item_to_parse = if regex {
        pattern_item
    } else {
        build_regex(&pattern_item, pattern_span)?
    };

    let regex_pattern = Regex::new(&item_to_parse).map_err(|err| {
        ShellError::GenericError(
            "Error with regular expression".into(),
            err.to_string(),
            Some(pattern_span),
            None,
            Vec::new(),
        )
    })?;

    let columns = column_names(&regex_pattern);
    let mut parsed: Vec<Value> = Vec::new();

    for v in input {
        match v.as_string() {
            Ok(s) => {
                let results = regex_pattern.captures_iter(&s);

                for c in results {
                    let mut cols = Vec::with_capacity(columns.len());
                    let captures = match c {
                        Ok(c) => c,
                        Err(e) => {
                            return Err(ShellError::GenericError(
                                "Error with regular expression captures".into(),
                                e.to_string(),
                                None,
                                None,
                                Vec::new(),
                            ))
                        }
                    };
                    let mut vals = Vec::with_capacity(captures.len());

                    for (column_name, cap) in columns.iter().zip(captures.iter().skip(1)) {
                        let cap_string = cap.map(|v| v.as_str()).unwrap_or("").to_string();
                        cols.push(column_name.clone());
                        vals.push(Value::String {
                            val: cap_string,
                            span: v.span()?,
                        });
                    }

                    parsed.push(Value::Record {
                        cols,
                        vals,
                        span: head,
                    });
                }
            }
            Err(_) => {
                return Err(ShellError::PipelineMismatch(
                    "string".into(),
                    head,
                    v.span()?,
                ))
            }
        }
    }

    Ok(PipelineData::ListStream(
        ListStream::from_stream(parsed.into_iter(), ctrlc),
        None,
    ))
}

fn build_regex(input: &str, span: Span) -> Result<String, ShellError> {
    let mut output = "(?s)\\A".to_string();

    //let mut loop_input = input;
    let mut loop_input = input.chars().peekable();
    loop {
        let mut before = String::new();
        while let Some(c) = loop_input.next() {
            if c == '{' {
                // If '{{', still creating a plaintext parse command, but just for a single '{' char
                if loop_input.peek() == Some(&'{') {
                    let _ = loop_input.next();
                } else {
                    break;
                }
            }
            before.push(c);
        }

        if !before.is_empty() {
            output.push_str(&fancy_regex::escape(&before));
        }

        // Look for column as we're now at one
        let mut column = String::new();
        while let Some(c) = loop_input.next() {
            if c == '}' {
                break;
            }
            column.push(c);

            if loop_input.peek().is_none() {
                return Err(ShellError::DelimiterError(
                    "Found opening `{` without an associated closing `}`".to_owned(),
                    span,
                ));
            }
        }

        if !column.is_empty() {
            output.push_str("(?P<");
            output.push_str(&column);
            output.push_str(">.*?)");
        }

        if before.is_empty() && column.is_empty() {
            break;
        }
    }

    output.push_str("\\z");
    Ok(output)
}

fn column_names(regex: &Regex) -> Vec<String> {
    regex
        .capture_names()
        .enumerate()
        .skip(1)
        .map(|(i, name)| {
            name.map(String::from)
                .unwrap_or_else(|| format!("Capture{}", i))
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(Parse)
    }
}
