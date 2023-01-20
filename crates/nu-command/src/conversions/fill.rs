use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use unicode_width::UnicodeWidthStr;

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

    fn usage(&self) -> &str {
        "Fill and Align"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("fill")
            .input_output_types(vec![(Type::Number, Type::Number)])
            .input_output_types(vec![(Type::String, Type::String)])
            .named(
                "width",
                SyntaxShape::Int,
                "the width of the output",
                Some('w'),
            )
            .named(
                "alignment",
                SyntaxShape::String,
                "the alignment of the output",
                Some('a'),
            )
            .named(
                "character",
                SyntaxShape::String,
                "the fill character",
                Some('c'),
            )
            .category(Category::Conversions)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["display", "render", "format"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get a record containing multiple formats for the number 42",
            example: "42 | fmt",
            result: Some(Value::Record {
                cols: vec![
                    "binary".into(),
                    "debug".into(),
                    "display".into(),
                    "lowerexp".into(),
                    "lowerhex".into(),
                    "octal".into(),
                    "upperexp".into(),
                    "upperhex".into(),
                ],
                vals: vec![
                    Value::test_string("0b101010"),
                    Value::test_string("42"),
                    Value::test_string("42"),
                    Value::test_string("4.2e1"),
                    Value::test_string("0x2a"),
                    Value::test_string("0o52"),
                    Value::test_string("4.2E1"),
                    Value::test_string("0x2A"),
                ],
                span: Span::test_data(),
            }),
        }]
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
        match arg.to_lowercase().as_str() {
            "l" | "left" => FillAlignment::Left,
            "r" | "right" => FillAlignment::Right,
            "m" | "middle" => FillAlignment::Middle,
            "mr" | "middleright" => FillAlignment::MiddleRight,
            _ => FillAlignment::Left,
        }
    } else {
        FillAlignment::Left
    };

    let width = if let Some(arg) = width_arg {
        arg as usize
    } else {
        1
    };

    let character = if let Some(arg) = character_arg {
        arg.to_string()
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
        Value::String { val, .. } => fill_string(&*val, args, span),
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::Error {
            error: ShellError::OnlySupportsThisInputType(
                "int, filesize, float, string".into(),
                other.get_type().to_string(),
                span,
                // This line requires the Value::Error match above.
                other.expect_span(),
            ),
        },
    }
}

fn fill_float(num: f64, args: &Arguments, span: Span) -> Value {
    Value::Nothing { span }
}
fn fill_int(num: i64, args: &Arguments, span: Span) -> Value {
    Value::Nothing { span }
}
fn fill_string(s: &str, args: &Arguments, span: Span) -> Value {
    let out_str = pad(&s, args.width, &args.character, args.alignment, false);
    // let mut s = s.clone();
    // let width = args.width as usize;
    // let character = args.character.clone();
    // let alignment = args.alignment.clone();

    // if s.len() < width {
    //     let diff = width - s.len();
    //     let mut padding = String::new();
    //     for _ in 0..diff {
    //         padding.push_str(&character);
    //     }

    //     match alignment {
    //         FillAlignment::Left => s.push_str(&padding),
    //         FillAlignment::Right => s.insert_str(0, &padding),
    //         FillAlignment::Center => {
    //             let left = diff / 2;
    //             let right = diff - left;
    //             let mut left_padding = String::new();
    //             let mut right_padding = String::new();
    //             for _ in 0..left {
    //                 left_padding.push_str(&character);
    //             }
    //             for _ in 0..right {
    //                 right_padding.push_str(&character);
    //             }
    //             s.insert_str(0, &left_padding);
    //             s.push_str(&right_padding);
    //         }
    //     }
    // }

    Value::String { val: out_str, span }
}

fn pad(s: &str, width: usize, pad_char: &str, alignment: FillAlignment, truncate: bool) -> String {
    // eprintln!("str start: {}", &s);
    // Use width instead of len for graphical display
    let cols = UnicodeWidthStr::width(s);

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
    // eprintln!("left_pad: {}, right_pad: {}", left_pad, right_pad);
    let mut new_str = String::new();
    for _ in 0..left_pad {
        new_str.push_str(pad_char)
    }
    new_str.push_str(&s);
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
