use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

use print_positions::print_positions;

#[derive(Clone)]
pub struct Fill;

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

#[derive(Clone, Copy)]
enum FillAlignment {
    Left,
    Right,
    Middle,
    MiddleRight,
}

impl Command for Fill {
    fn name(&self) -> &str {
        "fill"
    }

    fn description(&self) -> &str {
        "Fill and Align."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("fill")
            .input_output_types(vec![
                (Type::Int, Type::String),
                (Type::Float, Type::String),
                (Type::String, Type::String),
                (Type::Filesize, Type::String),
                (
                    Type::List(Box::new(Type::Int)),
                    Type::List(Box::new(Type::String)),
                ),
                (
                    Type::List(Box::new(Type::Float)),
                    Type::List(Box::new(Type::String)),
                ),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
                (
                    Type::List(Box::new(Type::Filesize)),
                    Type::List(Box::new(Type::String)),
                ),
                // General case for heterogeneous lists
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::String)),
                ),
            ])
            .allow_variants_without_examples(true)
            .named(
                "width",
                SyntaxShape::Int,
                "The width of the output. Defaults to 1",
                Some('w'),
            )
            .param(
                Flag::new("alignment")
                    .short('a')
                    .arg(SyntaxShape::String)
                    .desc(
                        "The alignment of the output. Defaults to Left (Left(l), Right(r), \
                         Center(c/m), MiddleRight(cr/mr))",
                    )
                    .completion(Completion::new_list(&[
                        "left",
                        "right",
                        "middle",
                        "middleright",
                    ])),
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
        vec!["display", "render", "format", "pad", "align", "repeat"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Fill a string on the left side to a width of 15 with the character '─'",
                example: "'nushell' | fill --alignment l --character '─' --width 15",
                result: Some(Value::string("nushell────────", Span::test_data())),
            },
            Example {
                description: "Fill a string on the right side to a width of 15 with the character '─'",
                example: "'nushell' | fill --alignment r --character '─' --width 15",
                result: Some(Value::string("────────nushell", Span::test_data())),
            },
            Example {
                description: "Fill an empty string with 10 '─' characters",
                example: "'' | fill --character '─' --width 10",
                result: Some(Value::string("──────────", Span::test_data())),
            },
            Example {
                description: "Fill a number on the left side to a width of 5 with the character '0'",
                example: "1 | fill --alignment right --character '0' --width 5",
                result: Some(Value::string("00001", Span::test_data())),
            },
            Example {
                description: "Fill a number on both sides to a width of 5 with the character '0'",
                example: "1.1 | fill --alignment center --character '0' --width 5",
                result: Some(Value::string("01.10", Span::test_data())),
            },
            Example {
                description: "Fill a filesize on both sides to a width of 10 with the character '0'",
                example: "1kib | fill --alignment middle --character '0' --width 10",
                result: Some(Value::string("0001024000", Span::test_data())),
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

    let width = width_arg.unwrap_or(1);

    let character = character_arg.unwrap_or_else(|| " ".to_string());

    let arg = Arguments {
        width,
        alignment,
        character,
        cell_paths,
    };

    operate(action, arg, input, call.head, engine_state.signals())
}

fn action(input: &Value, args: &Arguments, span: Span) -> Value {
    match input {
        Value::Int { val, .. } => fill_int(*val, args, span),
        Value::Filesize { val, .. } => fill_int(val.get(), args, span),
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

        test_examples(Fill {})
    }
}
