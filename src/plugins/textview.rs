#![feature(option_flattening)]

use crossterm::{cursor, terminal, RawScreen};
use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, CommandConfig, Plugin, Primitive, ShellError, SourceMap, SpanSource,
    Spanned, Value,
};
use rawkey::RawKey;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;

use std::io::Write;
use std::path::Path;
use std::{thread, time::Duration};

enum DrawCommand {
    DrawString(Style, String),
    NextLine,
}
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

fn paint_textview(
    draw_commands: &Vec<DrawCommand>,
    starting_row: usize,
    use_color_buffer: bool,
) -> usize {
    let terminal = terminal();
    let cursor = cursor();

    let size = terminal.terminal_size();

    // render
    let mut pos = 0;
    let width = size.0 as usize + 1;
    let height = size.1 as usize;
    let mut frame_buffer = vec![]; //(' ', 0, 0, 0); max_pos];

    for command in draw_commands {
        match command {
            DrawCommand::DrawString(style, string) => {
                for chr in string.chars() {
                    frame_buffer.push((
                        chr,
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    ));
                    pos += 1;
                }
            }
            DrawCommand::NextLine => {
                for _ in 0..(width - pos % width) {
                    frame_buffer.push((' ', 0, 0, 0));
                }
                pos += width - pos % width;
            }
        }
    }

    // if it's a short buffer, be sure to fill it out
    while pos < (width * height) {
        frame_buffer.push((' ', 0, 0, 0));
        pos += 1;
    }

    // display
    let mut ansi_strings = vec![];
    let mut normal_chars = vec![];

    for c in &frame_buffer[starting_row * width..(starting_row + height) * width] {
        if use_color_buffer {
            ansi_strings.push(ansi_term::Colour::RGB(c.1, c.2, c.3).paint(format!("{}", c.0)));
        } else {
            normal_chars.push(c.0);
        }
    }

    let _ = cursor.goto(0, 0);
    if use_color_buffer {
        print!("{}", ansi_term::ANSIStrings(&ansi_strings));
    } else {
        let s: String = normal_chars.into_iter().collect();
        print!("{}", s);
    }

    if (frame_buffer.len() / width) > height {
        let _ = cursor.goto(0, size.1);
        print!(
            "{}",
            ansi_term::Colour::Blue.paint("[ESC to quit, arrow keys to move]")
        );
    }

    let _ = std::io::stdout().flush();

    frame_buffer.len() / width
}

fn scroll_view_lines_if_needed(draw_commands: Vec<DrawCommand>, use_color_buffer: bool) {
    let mut starting_row = 0;
    let rawkey = RawKey::new();

    if let Ok(_raw) = RawScreen::into_raw_mode() {
        let cursor = cursor();
        let _ = cursor.hide();

        let input = crossterm::input();
        let _ = input.read_async();

        let terminal = terminal();
        let mut size = terminal.terminal_size();
        let mut max_bottom_line = paint_textview(&draw_commands, starting_row, use_color_buffer);

        // Only scroll if needed
        if max_bottom_line > size.1 as usize {
            loop {
                if rawkey.is_pressed(rawkey::KeyCode::Escape) {
                    break;
                }
                if rawkey.is_pressed(rawkey::KeyCode::UpArrow) {
                    if starting_row > 0 {
                        starting_row -= 1;
                        max_bottom_line =
                            paint_textview(&draw_commands, starting_row, use_color_buffer);
                    }
                }
                if rawkey.is_pressed(rawkey::KeyCode::DownArrow) {
                    if starting_row < (max_bottom_line - size.1 as usize) {
                        starting_row += 1;
                    }
                    max_bottom_line =
                        paint_textview(&draw_commands, starting_row, use_color_buffer);
                }
                if rawkey.is_pressed(rawkey::KeyCode::PageUp) {
                    starting_row -= std::cmp::min(size.1 as usize, starting_row);
                    max_bottom_line =
                        paint_textview(&draw_commands, starting_row, use_color_buffer);
                }
                if rawkey.is_pressed(rawkey::KeyCode::PageDown) {
                    if starting_row < (max_bottom_line - size.1 as usize) {
                        starting_row += size.1 as usize;

                        if starting_row > (max_bottom_line - size.1 as usize) {
                            starting_row = max_bottom_line - size.1 as usize;
                        }
                    }
                    max_bottom_line =
                        paint_textview(&draw_commands, starting_row, use_color_buffer);
                }

                thread::sleep(Duration::from_millis(50));

                let new_size = terminal.terminal_size();
                if size != new_size {
                    size = new_size;
                    let _ = terminal.clear(crossterm::ClearType::All);
                    max_bottom_line =
                        paint_textview(&draw_commands, starting_row, use_color_buffer);
                }
            }
        }

        let _ = cursor.show();
    }

    let cursor = cursor();
    let _ = cursor.show();

    #[allow(unused)]
    let screen = RawScreen::disable_raw_mode();

    println!("");
    thread::sleep(Duration::from_millis(50));
}

fn scroll_view(s: &str) {
    let mut v = vec![];
    for line in s.lines() {
        v.push(DrawCommand::DrawString(Style::default(), line.to_string()));
        v.push(DrawCommand::NextLine);
    }
    scroll_view_lines_if_needed(v, false);
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

                                        for range in ranges {
                                            v.push(DrawCommand::DrawString(
                                                range.0,
                                                range.1.to_string(),
                                            ));
                                        }

                                        v.push(DrawCommand::NextLine);
                                    }
                                    scroll_view_lines_if_needed(v, true);
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
