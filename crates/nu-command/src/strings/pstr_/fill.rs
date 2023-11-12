use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use print_positions::print_positions;

#[derive(Clone)]
pub struct PstrFill;

struct Arguments {
    width: usize,
    alignment: FillAlignment,
    character: String,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

//todo: add decimal formatting, pstr fill --width 10 --precision 2 (--align right --character ' ')

#[derive(Clone, Copy)]
enum FillAlignment {
    Left,
    Right,
    Middle,
    MiddleRight,
}

impl Command for PstrFill {
    fn name(&self) -> &str {
        "pstr fill"
    }

    fn usage(&self) -> &str {
        "Fill and Align."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("pstr fill")
            .input_output_types(vec![
                (Type::Int, Type::String),
                (Type::Float, Type::String),
                (Type::String, Type::String),
                (Type::Filesize, Type::String),
                (Type::List(Box::new(Type::Int)), Type::List(Box::new(Type::String))),
                (Type::List(Box::new(Type::Float)), Type::List(Box::new(Type::String))),
                (Type::List(Box::new(Type::String)), Type::List(Box::new(Type::String))),
                (Type::List(Box::new(Type::Filesize)), Type::List(Box::new(Type::String))),
                // General case for heterogeneous lists
                (Type::List(Box::new(Type::Any)), Type::List(Box::new(Type::String))),
                ])
            .allow_variants_without_examples(true)
            .named(
                "width",
                SyntaxShape::Int,
                "The width of the output. Defaults to 1",
                Some('w'),
            )
            .named(
                "alignment",
                SyntaxShape::String,
                "The alignment of the output. Defaults to Left (Left(l), Right(r), Center(c/m), MiddleRight(cr/mr))",
                Some('a'),
            )
            .named(
                "character",
                SyntaxShape::String,
                "The character to fill with. Defaults to ' ' (space)",
                Some('c'),
            )
            .category(Category::Conversions)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["display", "render", "format", "pad", "align"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Pad 6 character string with '-' to fill 15 character field, align to left",
                example: "'nushell' | pstr fill --alignment l --character '─' --width 15",
                result: Some(Value::string("nushell────────", Span::test_data())),
            },
            Example {
                description: "Pad 6 character string with '-' to fill 15 character field, align to right",
                example: "'nushell' | pstr fill --alignment r --character '─' --width 15",
                result: Some(Value::string("────────nushell", Span::test_data())),
            },
            Example {
                description: "Pad 6 character string with '-' to fill 15 character field, centered",
                example: "'nushell' | pstr fill --alignment m --character '─' --width 15",
                result: Some(Value::string("────nushell────", Span::test_data())),
            },
            Example {
                description: "6 character string in 4 character field -- don't truncate",   //bugbug -- should truncate at left or right based on alignment
                example: "'nushell' | pstr fill --alignment m --character '─' --width 4",
                result: Some(Value::string("nushell", Span::test_data())),
            },
            Example {
                description: "Pad 7 UTF-8 grapheme clusters to 15 screen positions",
                example: "'こんにちは世界' | pstr fill --alignment m --character '─' --width 15",
                result: Some(Value::string("────こんにちは世界────", Span::test_data())),
            },
            Example {
                description: "Pad ANSI styled string of 4 screen positions to 8 screen positions",
                example: r#"$"(ansi cyan)cyan(ansi reset)" | pstr fill --alignment m --character '─' --width 8"#,
                result: Some(Value::string("──\u{1b}[36mcyan\u{1b}[0m──", Span::test_data())),
            },
            
            Example {
                description:
                    "Convert number to decimal string, right aligned, pad to 5 characters with leading zeros",
                example: "1 | pstr fill --alignment right --character '0' --width 5",
                result: Some(Value::string("00001", Span::test_data())),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        fill(engine_state, stack, call, input)
    }
}

fn fill(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let width_arg: Option<usize> = call.get_flag(engine_state, stack, "width")?;
    let alignment_arg: Option<String> = call.get_flag(engine_state, stack, "alignment")?;
    let character_arg: Option<String> = call.get_flag(engine_state, stack, "character")?;
    let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);

    let alignment = if let Some(arg) = alignment_arg {
        match arg.to_ascii_lowercase().as_str() {
            "l" | "left" => FillAlignment::Left,
            "r" | "right" => FillAlignment::Right,
            "c" | "center" | "m" | "middle" => FillAlignment::Middle,
            "cr" | "centerright" | "mr" | "middleright" => FillAlignment::MiddleRight,
            _ => FillAlignment::Left,
        }
    } else {
        FillAlignment::Left
    };

    let width = if let Some(arg) = width_arg { arg } else { 1 };

    let character = if let Some(arg) = character_arg {
        arg
    } else {
        " ".to_string()
    };

    let arg = Arguments {
        width,
        alignment,
        character,
        cell_paths,
    };

    operate(action, arg, input, call.head, engine_state.ctrlc.clone())
}

fn action(input: &Value, args: &Arguments, span: Span) -> Value {
    match input {
        Value::Int { val, .. } => fill_int(*val, args, span),
        Value::Filesize { val, .. } => fill_int(*val, args, span),
        Value::Float { val, .. } => fill_float(*val, args, span),
        Value::String { val, .. } => fill_string(val, args, span),
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "int, filesize, float, string".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.span(),
            },
            span,
        ),
    }
}

fn fill_float(num: f64, args: &Arguments, span: Span) -> Value {
    let s = num.to_string();
    let out_str = pad(&s, args.width, &args.character, args.alignment, false);

    Value::string(out_str, span)
}
fn fill_int(num: i64, args: &Arguments, span: Span) -> Value {
    let s = num.to_string();
    let out_str = pad(&s, args.width, &args.character, args.alignment, false);

    Value::string(out_str, span)
}
fn fill_string(s: &str, args: &Arguments, span: Span) -> Value {
    let out_str = pad(s, args.width, &args.character, args.alignment, false);

    Value::string(out_str, span)
}

fn pad(s: &str, width: usize, pad_char: &str, alignment: FillAlignment, truncate: bool) -> String {
    // Attribution: Most of this function was taken from https://github.com/ogham/rust-pad and tweaked. Thank you!
    // Use width instead of len for graphical display

    let cols = print_positions(s).count();

    if cols >= width {
        if truncate {
            return s[..width].to_string();
        } else {
            return s.to_string();
        }
    }

    let diff = width - cols;

    let (left_pad, right_pad) = match alignment {
        FillAlignment::Left => (0, diff),
        FillAlignment::Right => (diff, 0),
        FillAlignment::Middle => (diff / 2, diff - diff / 2),
        FillAlignment::MiddleRight => (diff - diff / 2, diff / 2),
    };

    let mut new_str = String::new();
    for _ in 0..left_pad {
        new_str.push_str(pad_char)
    }
    new_str.push_str(s);
    for _ in 0..right_pad {
        new_str.push_str(pad_char)
    }
    new_str
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(PstrFill {})
    }
}
