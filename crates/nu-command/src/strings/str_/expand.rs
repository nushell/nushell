use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str expand"
    }

    fn usage(&self) -> &str {
        "Expands brace patterns in a string, generating all possible combinations."
    }

    fn signature(&self) -> Signature {
        Signature::build("str expand")
            .input_output_types(vec![(Type::String, Type::List(Box::new(Type::String)))])
            .vectorizes_over_list(true)
            .category(Category::Strings)
    }

    fn examples(&self) -> Vec<nu_protocol::Example> {
        vec![
            Example {
                description: "the brace expansion pattern {apple,banana,cherry} is expanded by str expand, resulting in the output apple banana cherry.",
                example: "echo (\"{apple,banana,cherry}\" | str expand)",
                result: Some(Value::List{
                    vals: vec![
                        Value::test_string("apple"),
                        Value::test_string("banana"),
                        Value::test_string("cherry")
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "the brace expansion pattern ~/Desktop/{file1,file2,file3}.txt represents multiple file paths with a common directory. str expand expands the pattern, resulting in the output of the expanded file paths.",
                example: "echo (\"~/Desktop/{file1,file2,file3}.txt\" | str expand)",
                result: Some(Value::List{
                    vals: vec![
                        Value::test_string("~/Desktop/file1.txt"),
                        Value::test_string("~/Desktop/file2.txt"),
                        Value::test_string("~/Desktop/file3.txt")
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Here, the brace expansion pattern {A{1,2},B{3,4}} contains nested brace expansions. str expand expands the pattern, resulting in the output A1 A2 B3 B4.",
                example: "echo (\"{A{1,2},B{3,4}}\" | str expand)",
                result: Some(Value::List{
                    vals: vec![
                        Value::test_string("A1"),
                        Value::test_string("A2"),
                        Value::test_string("B3"),
                        Value::test_string("B4")
                    ],
                    span: Span::test_data()
                },)
            }
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: span });
        }
        input.map(
            move |v| {
                let value_span = match v.span() {
                    Err(v) => return Value::Error { error: Box::new(v) },
                    Ok(v) => v,
                };
                match v.as_string() {
                    Ok(s) => str_expand(&s, span),
                    Err(_) => Value::Error {
                        error: Box::new(ShellError::PipelineMismatch {
                            exp_input_type: "string".into(),
                            dst_span: span,
                            src_span: value_span,
                        }),
                    },
                }
            },
            engine_state.ctrlc.clone(),
        )
    }
}

fn str_expand(contents: &str, span: Span) -> Value {
    if let Some(expansions) = expand(contents) {
        let vals = expansions
            .iter()
            .map(|e| Value::string(e, span))
            .collect::<Vec<Value>>();
        Value::List { vals, span }
    } else {
        Value::Error {
            error: Box::new(ShellError::DelimiterError {
                msg: "Please check the piped data.".into(),
                span,
            }),
        }
    }
}

// This would fit best in a seperate crate.
// But I'm a bit lazy. Perhaps, another day...
// Below code, is 1 day of work. 2 days of thinking.
// A. Taha Baki <atahabaki@pm.me>
fn expand(input: &str) -> Option<Vec<String>> {
    if input.is_empty() {
        return None;
    }
    let mut expansions = Vec::<String>::new();
    let mut count = (0, 0); // right, left / open, close
    let mut fixes = (String::new(), String::new()); // prefix, postfix
    let mut inside = String::new();
    for c in input.chars() {
        match c {
            '{' => {
                if count.0 != 0 {
                    inside.push(c);
                }
                count.0 += 1;
            }
            '}' => {
                count.1 += 1;
                if count.0 != count.1 {
                    inside.push(c);
                }
            }
            _ if count.0 == 0 => fixes.0.push(c),
            _ if count.0 == count.1 => fixes.1.push(c),
            _ => inside.push(c),
        }
    }
    let parts = split(inside);
    if let Some(pieces) = parts {
        for piece in pieces {
            let (prefix, postfix) = fixes.clone();
            if piece.contains('{') || piece.contains('}') {
                if let Some(recursive_parts) = expand(&piece) {
                    for recursive_part in recursive_parts {
                        let combination = combine(&prefix, &recursive_part, &postfix);
                        expansions.push(combination);
                    }
                }
            } else {
                let combination = combine(&prefix, &piece, &postfix);
                expansions.push(combination);
            }
        }
    } else {
        return None;
    }
    if expansions.is_empty() {
        None
    } else {
        Some(expansions)
    }
}

fn combine(prefix: &str, content: &str, postfix: &str) -> String {
    format!("{}{}{}", prefix, content, postfix)
}

fn split(content: impl ToString) -> Option<Vec<String>> {
    let content = content.to_string();
    if content.is_empty() {
        return None;
    }
    let mut pieces: Vec<String> = Vec::new();
    let mut count = (0, 0); // right, left / open, close
    let mut piece = String::new();
    for c in content.chars() {
        match c {
            '{' | '}' => {
                piece.push(c);
                if c == '{' {
                    count.0 += 1;
                } else {
                    count.1 += 1;
                }
            }
            ',' if count.0 == count.1 => {
                pieces.push(piece.clone());
                piece.clear();
            }
            _ => piece.push(c),
        }
    }
    if !piece.is_empty() {
        pieces.push(piece);
    }
    if pieces.is_empty() {
        None
    } else {
        Some(pieces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_simple() {
        let input = "c{d,e}f";
        let expected_output: Vec<String> = vec!["cdf".into(), "cef".into()];
        assert_eq!(expand(input), Some(expected_output));
    }

    #[test]
    fn test_expand_recursive1() {
        let input = "a{b,c{d,e}f,g}h";
        let output: Vec<String> = vec!["abh".into(), "acdfh".into(), "acefh".into(), "agh".into()];
        assert_eq!(expand(input), Some(output));
    }

    #[test]
    fn test_expand_recursive2() {
        let input = "a{b,c{d{1,2},e}f,g}h";
        let output: Vec<String> = vec![
            "abh".into(),
            "acd1fh".into(),
            "acd2fh".into(),
            "acefh".into(),
            "agh".into(),
        ];
        assert_eq!(expand(input), Some(output));
    }

    #[test]
    fn test_split_complex1() {
        let input = "b,c{d,e}f,g";
        let output: Vec<String> = vec!["b".into(), "c{d,e}f".into(), "g".into()];
        assert_eq!(split(input), Some(output));
    }

    #[test]
    fn test_split_complex2() {
        let input = "a,b,c,d{e,f},g{h,i,j},k";
        let output: Vec<String> = vec![
            "a".into(),
            "b".into(),
            "c".into(),
            "d{e,f}".into(),
            "g{h,i,j}".into(),
            "k".into(),
        ];
        assert_eq!(split(input), Some(output));
    }

    #[test]
    fn test_basic_brace_expansion() {
        let input = "{apple,banana,cherry}";
        let expected_output: Vec<String> = vec!["apple".into(), "banana".into(), "cherry".into()];
        assert_eq!(expand(&input), Some(expected_output))
    }
}
