use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    ast::Call, Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span,
    Spanned, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct SeqChar;

impl Command for SeqChar {
    fn name(&self) -> &str {
        "seq char"
    }

    fn usage(&self) -> &str {
        "Print sequence of chars"
    }

    fn signature(&self) -> Signature {
        Signature::build("seq char")
            .rest("rest", SyntaxShape::String, "sequence chars")
            .named(
                "separator",
                SyntaxShape::String,
                "separator character (defaults to \\n)",
                Some('s'),
            )
            .named(
                "terminator",
                SyntaxShape::String,
                "terminator character (defaults to \\n)",
                Some('t'),
            )
            .category(Category::Generators)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "sequence a to e with newline separator",
                example: "seq char a e",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string('a'),
                        Value::test_string('b'),
                        Value::test_string('c'),
                        Value::test_string('d'),
                        Value::test_string('e'),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "sequence a to e with pipe separator separator",
                example: "seq char -s '|' a e",
                result: Some(Value::test_string("a|b|c|d|e")),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        seq_char(engine_state, stack, call)
    }
}

fn is_single_character(ch: &str) -> bool {
    ch.is_ascii() && ch.len() == 1 && ch.chars().all(char::is_alphabetic)
}

fn seq_char(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    // input check.
    let separator: Option<Spanned<String>> = call.get_flag(engine_state, stack, "separator")?;
    let terminator: Option<Spanned<String>> = call.get_flag(engine_state, stack, "terminator")?;
    let rest_inputs: Vec<Spanned<String>> = call.rest(engine_state, stack, 0)?;

    let (start_ch, end_ch) = if rest_inputs.len() != 2
        || !is_single_character(&rest_inputs[0].item)
        || !is_single_character(&rest_inputs[1].item)
    {
        return Err(ShellError::GenericError(
            "seq char required two character parameters".into(),
            "needs parameter".into(),
            Some(call.head),
            None,
            Vec::new(),
        ));
    } else {
        // unwrap here is ok, because we just check the length of `rest_inputs`.
        (
            rest_inputs[0]
                .item
                .chars()
                .next()
                .expect("seq char input must contains 2 inputs"),
            rest_inputs[1]
                .item
                .chars()
                .next()
                .expect("seq char input must contains 2 inputs"),
        )
    };

    let sep: String = match separator {
        Some(s) => {
            if s.item == r"\t" {
                '\t'.to_string()
            } else if s.item == r"\n" {
                '\n'.to_string()
            } else if s.item == r"\r" {
                '\r'.to_string()
            } else {
                let vec_s: Vec<char> = s.item.chars().collect();
                if vec_s.is_empty() {
                    return Err(ShellError::GenericError(
                        "Expected a single separator char from --separator".into(),
                        "requires a single character string input".into(),
                        Some(s.span),
                        None,
                        Vec::new(),
                    ));
                };
                vec_s.iter().collect()
            }
        }
        _ => '\n'.to_string(),
    };

    let terminator: String = match terminator {
        Some(t) => {
            if t.item == r"\t" {
                '\t'.to_string()
            } else if t.item == r"\n" {
                '\n'.to_string()
            } else if t.item == r"\r" {
                '\r'.to_string()
            } else {
                let vec_t: Vec<char> = t.item.chars().collect();
                if vec_t.is_empty() {
                    return Err(ShellError::GenericError(
                        "Expected a single terminator char from --terminator".into(),
                        "requires a single character string input".into(),
                        Some(t.span),
                        None,
                        Vec::new(),
                    ));
                };
                vec_t.iter().collect()
            }
        }
        _ => '\n'.to_string(),
    };

    let span = call.head;
    run_seq_char(start_ch, end_ch, sep, terminator, span)
}

fn run_seq_char(
    start_ch: char,
    end_ch: char,
    sep: String,
    terminator: String,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let mut result_vec = vec![];
    for current_ch in start_ch as u8..end_ch as u8 + 1 {
        result_vec.push((current_ch as char).to_string())
    }
    let return_list = (sep == "\n" || sep == "\r") && (terminator == "\n" || terminator == "\r");
    if return_list {
        let result = result_vec
            .into_iter()
            .map(|x| Value::String { val: x, span })
            .collect::<Vec<Value>>();
        Ok(Value::List { vals: result, span }.into_pipeline_data())
    } else {
        let mut result = result_vec.join(&sep);
        result.push_str(&terminator);
        // doesn't output a list, if separator is '\n', it's better to eliminate them.
        // and it matches `seq` behavior.
        let result = result.lines().collect();
        Ok(Value::String { val: result, span }.into_pipeline_data())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SeqChar {})
    }
}
