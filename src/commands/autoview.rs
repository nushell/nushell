use crate::commands::{RawCommandArgs, StaticCommand};
use crate::context::{SourceMap, SpanSource};
use crate::errors::ShellError;
use crate::prelude::*;
use std::path::Path;

pub struct Autoview;

#[derive(Deserialize)]
pub struct AutoviewArgs {}

impl StaticCommand for Autoview {
    fn name(&self) -> &str {
        "autoview"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process_raw(registry, autoview)?.run()
    }

    fn signature(&self) -> Signature {
        Signature::build("autoview")
    }
}

pub fn autoview(
    AutoviewArgs {}: AutoviewArgs,
    mut context: RunnableContext,
    raw: RawCommandArgs,
) -> Result<OutputStream, ShellError> {
    Ok(OutputStream::new(async_stream_block! {
        let input = context.input.drain_vec().await;

        if input.len() > 0 {
            if let Spanned {
                item: Value::Binary(_),
                ..
            } = input[0]
            {
                let binary = context.expect_command("binaryview");
                binary.run(raw.with_input(input), &context.commands).await;
            } else if is_single_text_value(&input) {
                view_text_value(&input[0], &raw.call_info.source_map);
            } else if equal_shapes(&input) {
                let table = context.expect_command("table");
                let result = table.run(raw.with_input(input), &context.commands).await.unwrap();
                result.collect::<Vec<_>>().await;
            } else {
                println!("TODO!")
                // TODO
                // let mut host = context.host.lock().unwrap();
                // for i in input.iter() {
                //     let view = GenericView::new(&i);
                //     handle_unexpected(&mut *host, |host| crate::format::print_view(&view, host));
                //     host.stdout("");
                // }
            }
        }
    }))
}

fn equal_shapes(input: &Vec<Spanned<Value>>) -> bool {
    let mut items = input.iter();

    let item = match items.next() {
        Some(item) => item,
        None => return false,
    };

    let desc = item.data_descriptors();

    for item in items {
        if desc != item.data_descriptors() {
            return false;
        }
    }

    true
}

fn is_single_text_value(input: &Vec<Spanned<Value>>) -> bool {
    if input.len() != 1 {
        return false;
    }
    if let Spanned {
        item: Value::Primitive(Primitive::String(_)),
        ..
    } = input[0]
    {
        true
    } else {
        false
    }
}

fn view_text_value(value: &Spanned<Value>, source_map: &SourceMap) {
    match value {
        Spanned {
            item: Value::Primitive(Primitive::String(s)),
            span,
        } => {
            let source = span.source.map(|x| source_map.get(&x)).flatten();

            if let Some(source) = source {
                match source {
                    SpanSource::File(file) => {
                        let path = Path::new(file);
                        match path.extension() {
                            Some(extension) => {
                                use syntect::easy::HighlightLines;
                                use syntect::highlighting::{Style, ThemeSet};
                                use syntect::parsing::SyntaxSet;
                                use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

                                // Load these once at the start of your program
                                let ps: SyntaxSet = syntect::dumps::from_binary(include_bytes!(
                                    "../../assets/syntaxes.bin"
                                ));

                                if let Some(syntax) =
                                    ps.find_syntax_by_extension(extension.to_str().unwrap())
                                {
                                    let ts: ThemeSet = syntect::dumps::from_binary(include_bytes!(
                                        "../../assets/themes.bin"
                                    ));
                                    let mut h =
                                        HighlightLines::new(syntax, &ts.themes["OneHalfDark"]);

                                    for line in LinesWithEndings::from(s) {
                                        let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);
                                        let escaped =
                                            as_24_bit_terminal_escaped(&ranges[..], false);
                                        print!("{}", escaped);
                                    }
                                } else {
                                    println!("{}", s);
                                }
                            }
                            _ => {
                                println!("{}", s);
                            }
                        }
                    }
                    _ => {
                        println!("{}", s);
                    }
                }
            } else {
                println!("{}", s);
            }
        }
        _ => {}
    }
}
