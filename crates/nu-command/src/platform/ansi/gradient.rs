use nu_ansi_term::{build_all_gradient_text, gradient::TargetGround, Gradient, Rgb};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call, ast::CellPath, engine::Command, engine::EngineState, engine::Stack, Category,
    Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "ansi gradient"
    }

    fn signature(&self) -> Signature {
        Signature::build("ansi gradient")
            .named(
                "fgstart",
                SyntaxShape::String,
                "foreground gradient start color in hex (0x123456)",
                Some('a'),
            )
            .named(
                "fgend",
                SyntaxShape::String,
                "foreground gradient end color in hex",
                Some('b'),
            )
            .named(
                "bgstart",
                SyntaxShape::String,
                "background gradient start color in hex",
                Some('c'),
            )
            .named(
                "bgend",
                SyntaxShape::String,
                "background gradient end color in hex",
                Some('d'),
            )
            .rest(
                "cell path",
                SyntaxShape::CellPath,
                "for a data structure input, add a gradient to strings at the given cell paths",
            )
            .category(Category::Platform)
    }

    fn usage(&self) -> &str {
        "Add a color gradient (using ANSI color codes) to the given string"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        operate(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
            description: "draw text in a gradient with foreground start and end colors",
            example:
                "echo 'Hello, Nushell! This is a gradient.' | ansi gradient --fgstart 0x40c9ff --fgend 0xe81cff",
            result: None,
        },
        Example {
            description: "draw text in a gradient with foreground start and end colors and background start and end colors",
            example:
                "echo 'Hello, Nushell! This is a gradient.' | ansi gradient --fgstart 0x40c9ff --fgend 0xe81cff --bgstart 0xe81cff --bgend 0x40c9ff",
            result: None,
        },
        Example {
            description: "draw text in a gradient by specifying foreground start color - end color is assumed to be black",
            example:
                "echo 'Hello, Nushell! This is a gradient.' | ansi gradient --fgstart 0x40c9ff",
            result: None,
        },
        Example {
            description: "draw text in a gradient by specifying foreground end color - start color is assumed to be black",
            example:
                "echo 'Hello, Nushell! This is a gradient.' | ansi gradient --fgend 0xe81cff",
            result: None,
        },
        ]
    }
}

fn value_to_color(v: Option<Value>) -> Result<Option<Rgb>, ShellError> {
    let s = match v {
        None => return Ok(None),
        Some(x) => x.as_string()?,
    };
    Ok(Some(Rgb::from_hex_string(s)))
}

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let fgstart: Option<Value> = call.get_flag(engine_state, stack, "fgstart")?;
    let fgend: Option<Value> = call.get_flag(engine_state, stack, "fgend")?;
    let bgstart: Option<Value> = call.get_flag(engine_state, stack, "bgstart")?;
    let bgend: Option<Value> = call.get_flag(engine_state, stack, "bgend")?;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    let fgs_hex = value_to_color(fgstart)?;
    let fge_hex = value_to_color(fgend)?;
    let bgs_hex = value_to_color(bgstart)?;
    let bge_hex = value_to_color(bgend)?;
    let head = call.head;
    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, fgs_hex, fge_hex, bgs_hex, bge_hex, &head)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, fgs_hex, fge_hex, bgs_hex, bge_hex, &head)),
                    );
                    if let Err(error) = r {
                        return Value::Error { error };
                    }
                }
                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}

fn action(
    input: &Value,
    fg_start: Option<Rgb>,
    fg_end: Option<Rgb>,
    bg_start: Option<Rgb>,
    bg_end: Option<Rgb>,
    command_span: &Span,
) -> Value {
    match input {
        Value::String { val, span } => {
            match (fg_start, fg_end, bg_start, bg_end) {
                (None, None, None, None) => {
                    // Error - no colors
                    Value::Error {
                        error: ShellError::MissingParameter(
                            "please supply foreground and/or background color parameters".into(),
                            *command_span,
                        ),
                    }
                }
                (None, None, None, Some(bg_end)) => {
                    // Error - missing bg_start, so assume black
                    let bg_start = Rgb::new(0, 0, 0);
                    let gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string = gradient.build(val, TargetGround::Background);
                    Value::string(gradient_string, *span)
                }
                (None, None, Some(bg_start), None) => {
                    // Error - missing bg_end, so assume black
                    let bg_end = Rgb::new(0, 0, 0);
                    let gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string = gradient.build(val, TargetGround::Background);
                    Value::string(gradient_string, *span)
                }
                (None, None, Some(bg_start), Some(bg_end)) => {
                    // Background Only
                    let gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string = gradient.build(val, TargetGround::Background);
                    Value::string(gradient_string, *span)
                }
                (None, Some(fg_end), None, None) => {
                    // Error - missing fg_start, so assume black
                    let fg_start = Rgb::new(0, 0, 0);
                    let gradient = Gradient::new(fg_start, fg_end);
                    let gradient_string = gradient.build(val, TargetGround::Foreground);
                    Value::string(gradient_string, *span)
                }
                (None, Some(fg_end), None, Some(bg_end)) => {
                    // missin fg_start and bg_start, so assume black
                    let fg_start = Rgb::new(0, 0, 0);
                    let bg_start = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string = build_all_gradient_text(val, fg_gradient, bg_gradient);
                    Value::string(gradient_string, *span)
                }
                (None, Some(fg_end), Some(bg_start), None) => {
                    // Error - missing fg_start and bg_end
                    let fg_start = Rgb::new(0, 0, 0);
                    let bg_end = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string = build_all_gradient_text(val, fg_gradient, bg_gradient);
                    Value::string(gradient_string, *span)
                }
                (None, Some(fg_end), Some(bg_start), Some(bg_end)) => {
                    // Error - missing fg_start, so assume black
                    let fg_start = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string = build_all_gradient_text(val, fg_gradient, bg_gradient);
                    Value::string(gradient_string, *span)
                }
                (Some(fg_start), None, None, None) => {
                    // Error - missing fg_end, so assume black
                    let fg_end = Rgb::new(0, 0, 0);
                    let gradient = Gradient::new(fg_start, fg_end);
                    let gradient_string = gradient.build(val, TargetGround::Foreground);
                    Value::string(gradient_string, *span)
                }
                (Some(fg_start), None, None, Some(bg_end)) => {
                    // Error - missing fg_end, bg_start, so assume black
                    let fg_end = Rgb::new(0, 0, 0);
                    let bg_start = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string = build_all_gradient_text(val, fg_gradient, bg_gradient);
                    Value::string(gradient_string, *span)
                }
                (Some(fg_start), None, Some(bg_start), None) => {
                    // Error - missing fg_end, bg_end, so assume black
                    let fg_end = Rgb::new(0, 0, 0);
                    let bg_end = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string = build_all_gradient_text(val, fg_gradient, bg_gradient);
                    Value::string(gradient_string, *span)
                }
                (Some(fg_start), None, Some(bg_start), Some(bg_end)) => {
                    // Error - missing fg_end, so assume black
                    let fg_end = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string = build_all_gradient_text(val, fg_gradient, bg_gradient);
                    Value::string(gradient_string, *span)
                }
                (Some(fg_start), Some(fg_end), None, None) => {
                    // Foreground Only
                    let gradient = Gradient::new(fg_start, fg_end);
                    let gradient_string = gradient.build(val, TargetGround::Foreground);
                    Value::string(gradient_string, *span)
                }
                (Some(fg_start), Some(fg_end), None, Some(bg_end)) => {
                    // Error - missing bg_start, so assume black
                    let bg_start = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string = build_all_gradient_text(val, fg_gradient, bg_gradient);
                    Value::string(gradient_string, *span)
                }
                (Some(fg_start), Some(fg_end), Some(bg_start), None) => {
                    // Error - missing bg_end, so assume black
                    let bg_end = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string = build_all_gradient_text(val, fg_gradient, bg_gradient);
                    Value::string(gradient_string, *span)
                }
                (Some(fg_start), Some(fg_end), Some(bg_start), Some(bg_end)) => {
                    // Foreground and Background Gradient
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string = build_all_gradient_text(val, fg_gradient, bg_gradient);
                    Value::string(gradient_string, *span)
                }
            }
        }
        other => {
            let got = format!("value is {}, not string", other.get_type());

            Value::Error {
                error: ShellError::TypeMismatch(got, other.span().unwrap_or(*command_span)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{action, SubCommand};
    use nu_ansi_term::Rgb;
    use nu_protocol::{Span, Value};

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn test_fg_gradient() {
        let input_string = Value::test_string("Hello, World!");
        let expected = Value::test_string("\u{1b}[38;2;64;201;255mH\u{1b}[38;2;76;187;254me\u{1b}[38;2;89;174;254ml\u{1b}[38;2;102;160;254ml\u{1b}[38;2;115;147;254mo\u{1b}[38;2;128;133;254m,\u{1b}[38;2;141;120;254m \u{1b}[38;2;153;107;254mW\u{1b}[38;2;166;94;254mo\u{1b}[38;2;179;80;254mr\u{1b}[38;2;192;67;254ml\u{1b}[38;2;205;53;254md\u{1b}[38;2;218;40;254m!\u{1b}[0m");
        let fg_start = Rgb::from_hex_string("0x40c9ff".to_string());
        let fg_end = Rgb::from_hex_string("0xe81cff".to_string());
        let actual = action(
            &input_string,
            Some(fg_start),
            Some(fg_end),
            None,
            None,
            &Span::test_data(),
        );
        assert_eq!(actual, expected);
    }
}
