#![feature(option_flattening)]

use crossterm::{cursor, terminal, InputEvent, KeyEvent, RawScreen};
use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, CommandConfig, Plugin, Primitive, ShellError, SourceMap, SpanSource,
    Spanned, Value,
};
use std::io::Write;

use std::path::Path;
use std::{thread, time::Duration};

struct TextView;

impl TextView {
    fn new() -> TextView {
        TextView
    }
}

impl Plugin for TextView {
    fn config(&mut self) -> Result<CommandConfig, ShellError> {
        Ok(CommandConfig {
            name: "textview".to_string(),
            positional: vec![],
            is_filter: false,
            is_sink: true,
            named: IndexMap::new(),
            rest_positional: false,
        })
    }

    fn sink(&mut self, call_info: CallInfo, input: Vec<Spanned<Value>>) {
        view_text_value(&input[0], &call_info.source_map);
    }
}

fn paint_textview(lines: &Vec<String>, starting_row: usize) -> (u16, u16) {
    let terminal = terminal();
    let cursor = cursor();

    let _ = terminal.clear(crossterm::ClearType::All);

    let size = terminal.terminal_size();
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
    print!("{}", crossterm::Attribute::Reset);

    let _ = std::io::stdout().flush();

    size
}

fn scroll_view_lines(lines: Vec<String>) {
    let mut starting_row = 0;

    if let Ok(_raw) = RawScreen::into_raw_mode() {
        let terminal = terminal();
        let input = crossterm::input();
        let cursor = cursor();

        let _ = cursor.hide();

        let mut async_stdin = input.read_async();
        let _ = terminal.clear(crossterm::ClearType::All);

        let mut size = paint_textview(&lines, starting_row);
        loop {
            if let Some(key_event) = async_stdin.next() {
                match key_event {
                    InputEvent::Keyboard(k) => match k {
                        KeyEvent::Esc => {
                            break;
                        }
                        KeyEvent::Up => {
                            if starting_row > 0 {
                                starting_row -= 1;
                                size = paint_textview(&lines, starting_row);
                            }
                        }
                        KeyEvent::Down => {
                            if starting_row
                                < (std::cmp::max(size.1 as usize, lines.len()) - size.1 as usize)
                            {
                                starting_row += 1;
                                size = paint_textview(&lines, starting_row);
                            }
                        }
                        KeyEvent::PageUp => {
                            starting_row -= std::cmp::min(starting_row, size.1 as usize);
                            size = paint_textview(&lines, starting_row);
                        }
                        KeyEvent::Char(c) if c == ' ' => {
                            if starting_row
                                < (std::cmp::max(size.1 as usize, lines.len()) - size.1 as usize)
                            {
                                starting_row += size.1 as usize;
                                size = paint_textview(&lines, starting_row);
                            }
                        }
                        KeyEvent::PageDown => {
                            if starting_row
                                < (std::cmp::max(size.1 as usize, lines.len()) - size.1 as usize)
                            {
                                starting_row += size.1 as usize;
                                size = paint_textview(&lines, starting_row);
                            }
                        }
                        _ => {}
                    },

                    _ => {}
                }
            }

            thread::sleep(Duration::from_millis(50));
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

fn main() {
    serve_plugin(&mut TextView::new());
}
