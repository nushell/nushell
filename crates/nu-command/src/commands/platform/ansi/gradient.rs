use crate::prelude::*;
use nu_ansi_term::{build_all_gradient_text, gradient::TargetGround, Gradient, Rgb};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tag;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
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
                "rest",
                SyntaxShape::ColumnPath,
                "optionally, draw gradients using text from column paths",
            )
    }

    fn usage(&self) -> &str {
        "draw text with a provided start and end code making a gradient"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args)
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

fn operate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let fgstart: Option<Value> = args.get_flag("fgstart")?;
    let fgend: Option<Value> = args.get_flag("fgend")?;
    let bgstart: Option<Value> = args.get_flag("bgstart")?;
    let bgend: Option<Value> = args.get_flag("bgend")?;
    let column_paths: Vec<_> = args.rest(0)?;

    let fgs_hex = fgstart.map(|color| Rgb::from_hex_string(color.convert_to_string()));
    let fge_hex = fgend.map(|color| Rgb::from_hex_string(color.convert_to_string()));
    let bgs_hex = bgstart.map(|color| Rgb::from_hex_string(color.convert_to_string()));
    let bge_hex = bgend.map(|color| Rgb::from_hex_string(color.convert_to_string()));

    let result: Vec<Value> = args
        .input
        .map(move |v| {
            if column_paths.is_empty() {
                action(&v, v.tag(), fgs_hex, fge_hex, bgs_hex, bge_hex)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| {
                            action(old, old.tag(), fgs_hex, fge_hex, bgs_hex, bge_hex)
                        }),
                    )?;
                }

                Ok(ret)
            }
        })
        .collect::<Result<Vec<Value>, _>>()?;

    Ok(OutputStream::from_stream(result.into_iter()))
}

fn action(
    input: &Value,
    tag: impl Into<Tag>,
    fg_start: Option<Rgb>,
    fg_end: Option<Rgb>,
    bg_start: Option<Rgb>,
    bg_end: Option<Rgb>,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    match &input.value {
        UntaggedValue::Primitive(Primitive::String(astring)) => {
            match (fg_start, fg_end, bg_start, bg_end) {
                (None, None, None, None) => {
                    // Error - no colors
                    Err(ShellError::labeled_error(
                        "please supply color parameters",
                        "please supply foreground and/or background color parameters",
                        Tag::unknown(),
                    ))
                }
                (None, None, None, Some(bg_end)) => {
                    // Error - missing bg_start, so assume black
                    let bg_start = Rgb::new(0, 0, 0);
                    let gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string = gradient.build(astring, TargetGround::Background);
                    Ok(UntaggedValue::string(gradient_string).into_value(tag))
                }
                (None, None, Some(bg_start), None) => {
                    // Error - missing bg_end, so assume black
                    let bg_end = Rgb::new(0, 0, 0);
                    let gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string = gradient.build(astring, TargetGround::Background);
                    Ok(UntaggedValue::string(gradient_string).into_value(tag))
                }
                (None, None, Some(bg_start), Some(bg_end)) => {
                    // Background Only
                    let gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string = gradient.build(astring, TargetGround::Background);
                    Ok(UntaggedValue::string(gradient_string).into_value(tag))
                }
                (None, Some(fg_end), None, None) => {
                    // Error - missing fg_start, so assume black
                    let fg_start = Rgb::new(0, 0, 0);
                    let gradient = Gradient::new(fg_start, fg_end);
                    let gradient_string = gradient.build(astring, TargetGround::Foreground);
                    Ok(UntaggedValue::string(gradient_string).into_value(tag))
                }
                (None, Some(fg_end), None, Some(bg_end)) => {
                    // missin fg_start and bg_start, so assume black
                    let fg_start = Rgb::new(0, 0, 0);
                    let bg_start = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string =
                        build_all_gradient_text(astring, fg_gradient, bg_gradient);
                    Ok(UntaggedValue::string(gradient_string).into_value(tag))
                }
                (None, Some(fg_end), Some(bg_start), None) => {
                    // Error - missing fg_start and bg_end
                    let fg_start = Rgb::new(0, 0, 0);
                    let bg_end = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string =
                        build_all_gradient_text(astring, fg_gradient, bg_gradient);
                    Ok(UntaggedValue::string(gradient_string).into_value(tag))
                }
                (None, Some(fg_end), Some(bg_start), Some(bg_end)) => {
                    // Error - missing fg_start, so assume black
                    let fg_start = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string =
                        build_all_gradient_text(astring, fg_gradient, bg_gradient);
                    Ok(UntaggedValue::string(gradient_string).into_value(tag))
                }
                (Some(fg_start), None, None, None) => {
                    // Error - missing fg_end, so assume black
                    let fg_end = Rgb::new(0, 0, 0);
                    let gradient = Gradient::new(fg_start, fg_end);
                    let gradient_string = gradient.build(astring, TargetGround::Foreground);
                    Ok(UntaggedValue::string(gradient_string).into_value(tag))
                }
                (Some(fg_start), None, None, Some(bg_end)) => {
                    // Error - missing fg_end, bg_start, so assume black
                    let fg_end = Rgb::new(0, 0, 0);
                    let bg_start = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string =
                        build_all_gradient_text(astring, fg_gradient, bg_gradient);
                    Ok(UntaggedValue::string(gradient_string).into_value(tag))
                }
                (Some(fg_start), None, Some(bg_start), None) => {
                    // Error - missing fg_end, bg_end, so assume black
                    let fg_end = Rgb::new(0, 0, 0);
                    let bg_end = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string =
                        build_all_gradient_text(astring, fg_gradient, bg_gradient);
                    Ok(UntaggedValue::string(gradient_string).into_value(tag))
                }
                (Some(fg_start), None, Some(bg_start), Some(bg_end)) => {
                    // Error - missing fg_end, so assume black
                    let fg_end = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string =
                        build_all_gradient_text(astring, fg_gradient, bg_gradient);
                    Ok(UntaggedValue::string(gradient_string).into_value(tag))
                }
                (Some(fg_start), Some(fg_end), None, None) => {
                    // Foreground Only
                    let gradient = Gradient::new(fg_start, fg_end);
                    let gradient_string = gradient.build(astring, TargetGround::Foreground);
                    Ok(UntaggedValue::string(gradient_string).into_value(tag))
                }
                (Some(fg_start), Some(fg_end), None, Some(bg_end)) => {
                    // Error - missing bg_start, so assume black
                    let bg_start = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string =
                        build_all_gradient_text(astring, fg_gradient, bg_gradient);
                    Ok(UntaggedValue::string(gradient_string).into_value(tag))
                }
                (Some(fg_start), Some(fg_end), Some(bg_start), None) => {
                    // Error - missing bg_end, so assume black
                    let bg_end = Rgb::new(0, 0, 0);
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string =
                        build_all_gradient_text(astring, fg_gradient, bg_gradient);
                    Ok(UntaggedValue::string(gradient_string).into_value(tag))
                }
                (Some(fg_start), Some(fg_end), Some(bg_start), Some(bg_end)) => {
                    // Foreground and Background Gradient
                    let fg_gradient = Gradient::new(fg_start, fg_end);
                    let bg_gradient = Gradient::new(bg_start, bg_end);
                    let gradient_string =
                        build_all_gradient_text(astring, fg_gradient, bg_gradient);
                    Ok(UntaggedValue::string(gradient_string).into_value(tag))
                }
            }
        }
        other => {
            let got = format!("got {}", other.type_name());

            Err(ShellError::labeled_error(
                "value is not string",
                got,
                tag.span,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::{action, SubCommand};
    use nu_ansi_term::Rgb;
    use nu_protocol::UntaggedValue;
    use nu_source::Tag;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    // #[test]
    // fn test_stripping() {
    //     let input_string =
    //         UntaggedValue::string("\u{1b}[3;93;41mHello\u{1b}[0m \u{1b}[1;32mNu \u{1b}[1;35mWorld")
    //             .into_untagged_value();
    //     let expected = UntaggedValue::string("Hello Nu World").into_untagged_value();

    //     let actual = action(&input_string, Tag::unknown()).unwrap();
    //     assert_eq!(actual, expected);
    // }

    #[test]
    fn test_fg_gradient() {
        let input_string = UntaggedValue::string("Hello, World!").into_untagged_value();
        let expected = UntaggedValue::string("\u{1b}[38;2;64;201;255mH\u{1b}[38;2;76;187;254me\u{1b}[38;2;89;174;254ml\u{1b}[38;2;102;160;254ml\u{1b}[38;2;115;147;254mo\u{1b}[38;2;128;133;254m,\u{1b}[38;2;141;120;254m \u{1b}[38;2;153;107;254mW\u{1b}[38;2;166;94;254mo\u{1b}[38;2;179;80;254mr\u{1b}[38;2;192;67;254ml\u{1b}[38;2;205;53;254md\u{1b}[38;2;218;40;254m!\u{1b}[0m").into_untagged_value();
        let fg_start = Rgb::from_hex_string("0x40c9ff".to_string());
        let fg_end = Rgb::from_hex_string("0xe81cff".to_string());
        let actual = action(
            &input_string,
            Tag::unknown(),
            Some(fg_start),
            Some(fg_end),
            None,
            None,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }
}
