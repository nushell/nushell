use crate::commands::command::SinkCommandArgs;
use crate::context::{SourceMap, SpanSource};
use crate::errors::ShellError;
use crate::format::GenericView;
use crate::prelude::*;
use std::path::Path;

pub fn autoview(args: SinkCommandArgs) -> Result<(), ShellError> {
    if args.input.len() > 0 {
        if let Spanned {
            item: Value::Binary(_),
            ..
        } = args.input[0]
        {
            args.ctx.get_sink("binaryview").run(args)?;
        } else if is_single_text_value(&args.input) {
            view_text_value(&args.input[0], &args.call_info.source_map);
        } else if equal_shapes(&args.input) {
            args.ctx.get_sink("table").run(args)?;
        } else {
            let mut host = args.ctx.host.lock().unwrap();
            for i in args.input.iter() {
                let view = GenericView::new(&i);
                handle_unexpected(&mut *host, |host| crate::format::print_view(&view, host));
                host.stdout("");
            }
        }
    }

    Ok(())
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

fn scroll_view_lines(lines: Vec<String>) {
    use crossterm::{cursor, input, terminal, InputEvent, KeyEvent, RawScreen};
    use std::io::Write;

    let mut starting_row = 0;

    let terminal = terminal();

    if let Ok(_raw) = RawScreen::into_raw_mode() {
        let input = input();
        let cursor = cursor();

        let _ = cursor.hide();

        let mut sync_stdin = input.read_sync();

        loop {
            let size = terminal.terminal_size();
            let _ = terminal.clear(crossterm::ClearType::All);
            let _ = cursor.goto(0, 0);

            let mut total_max_num_lines = 0;
            for line in lines.iter().skip(starting_row).take(size.1 as usize) {
                //let pos = cursor.pos();
                let stripped_line = strip_ansi_escapes::strip(&line.as_bytes()).unwrap();
                let line_length = stripped_line.len();

                let max_num_lines = line_length as u16 / size.0
                    + if (line_length as u16 % size.0) > 0 {
                        1
                    } else {
                        0
                    };
                total_max_num_lines += max_num_lines;

                if total_max_num_lines < size.1 {
                    print!("{}\r\n", line);
                } else {
                    break;
                }
            }

            let _ = cursor.goto(0, size.1);
            print!(
                "{}",
                ansi_term::Colour::Blue.paint("[ESC to quit, arrow keys to move]")
            );
            let _ = std::io::stdout().flush();

            let event = sync_stdin.next();

            if let Some(key_event) = event {
                match key_event {
                    InputEvent::Keyboard(k) => match k {
                        KeyEvent::Esc => {
                            break;
                        }
                        KeyEvent::Up => {
                            if starting_row > 0 {
                                starting_row -= 1;
                            }
                        }
                        KeyEvent::Down => {
                            if starting_row
                                < (std::cmp::max(size.1 as usize, lines.len()) - size.1 as usize)
                            {
                                starting_row += 1;
                            }
                        }
                        KeyEvent::PageUp => {
                            starting_row -= std::cmp::min(starting_row, size.1 as usize);
                        }
                        KeyEvent::Char(c) if c == ' ' => {
                            if starting_row
                                < (std::cmp::max(size.1 as usize, lines.len()) - size.1 as usize)
                            {
                                starting_row += size.1 as usize;
                            }
                        }
                        KeyEvent::PageDown => {
                            if starting_row
                                < (std::cmp::max(size.1 as usize, lines.len()) - size.1 as usize)
                            {
                                starting_row += size.1 as usize;
                            }
                        }
                        _ => {}
                    },

                    _ => {}
                }
            }
        }

        let _ = cursor.show();
    }
}

fn scroll_view(s: &str) {
    let lines: Vec<_> = s.lines().map(|x| x.to_string()).collect();

    scroll_view_lines(lines);
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
                                use syntect::util::as_24_bit_terminal_escaped;

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

                                    let mut v = vec![];
                                    for line in s.lines() {
                                        let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);
                                        let escaped =
                                            as_24_bit_terminal_escaped(&ranges[..], false);
                                        v.push(format!("{}", escaped));
                                    }
                                    scroll_view_lines(v);
                                } else {
                                    scroll_view(s);
                                }
                            }
                            _ => {
                                scroll_view(s);
                            }
                        }
                    }
                    _ => {
                        scroll_view(s);
                    }
                }
            } else {
                scroll_view(s);
            }
        }
        _ => {}
    }
}
