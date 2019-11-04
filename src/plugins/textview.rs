use crossterm::{cursor, terminal, RawScreen};
use crossterm::{InputEvent, KeyEvent};
use nu::{
    outln, serve_plugin, AnchorLocation, CallInfo, Plugin, Primitive, ShellError, Signature,
    Tagged, Value,
};

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;

use std::io::Write;
use std::path::Path;

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
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("textview").desc("Autoview of text data."))
    }

    fn sink(&mut self, _call_info: CallInfo, input: Vec<Tagged<Value>>) {
        view_text_value(&input[0]);
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
    let width = size.0 as usize;
    let height = size.1 as usize - 1;
    let mut frame_buffer = vec![];

    for command in draw_commands {
        match command {
            DrawCommand::DrawString(style, string) => {
                for chr in string.chars() {
                    if chr == '\t' {
                        for _ in 0..8 {
                            frame_buffer.push((
                                ' ',
                                style.foreground.r,
                                style.foreground.g,
                                style.foreground.b,
                            ));
                        }
                        pos += 8;
                    } else {
                        frame_buffer.push((
                            chr,
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                        ));
                        pos += 1;
                    }
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

    let num_frame_buffer_rows = frame_buffer.len() / width;
    let buffer_needs_scrolling = num_frame_buffer_rows > height;

    // display
    let mut ansi_strings = vec![];
    let mut normal_chars = vec![];

    for c in
        &frame_buffer[starting_row * width..std::cmp::min(pos, (starting_row + height) * width)]
    {
        if use_color_buffer {
            ansi_strings.push(ansi_term::Colour::RGB(c.1, c.2, c.3).paint(format!("{}", c.0)));
        } else {
            normal_chars.push(c.0);
        }
    }

    if buffer_needs_scrolling {
        let _ = cursor.goto(0, 0);
    }

    if use_color_buffer {
        print!("{}", ansi_term::ANSIStrings(&ansi_strings));
    } else {
        let s: String = normal_chars.into_iter().collect();
        print!("{}", s);
    }

    if buffer_needs_scrolling {
        let _ = cursor.goto(0, size.1);
        print!(
            "{}",
            ansi_term::Colour::Blue.paint("[ESC to quit, arrow keys to move]")
        );
    }

    let _ = std::io::stdout().flush();

    num_frame_buffer_rows
}

fn scroll_view_lines_if_needed(draw_commands: Vec<DrawCommand>, use_color_buffer: bool) {
    let mut starting_row = 0;

    if let Ok(_raw) = RawScreen::into_raw_mode() {
        let terminal = terminal();
        let mut size = terminal.terminal_size();
        let height = size.1 as usize - 1;

        let mut max_bottom_line = paint_textview(&draw_commands, starting_row, use_color_buffer);

        // Only scroll if needed
        if max_bottom_line > height as usize {
            let cursor = cursor();
            let _ = cursor.hide();

            let input = crossterm::input();
            let mut sync_stdin = input.read_sync();

            loop {
                if let Some(ev) = sync_stdin.next() {
                    match ev {
                        InputEvent::Keyboard(k) => match k {
                            KeyEvent::Esc => {
                                break;
                            }
                            KeyEvent::Up | KeyEvent::Char('k') => {
                                if starting_row > 0 {
                                    starting_row -= 1;
                                    max_bottom_line = paint_textview(
                                        &draw_commands,
                                        starting_row,
                                        use_color_buffer,
                                    );
                                }
                            }
                            KeyEvent::Down | KeyEvent::Char('j') => {
                                if starting_row < (max_bottom_line - height) {
                                    starting_row += 1;
                                }
                                max_bottom_line =
                                    paint_textview(&draw_commands, starting_row, use_color_buffer);
                            }
                            KeyEvent::PageUp | KeyEvent::Ctrl('b') => {
                                starting_row -= std::cmp::min(height, starting_row);
                                max_bottom_line =
                                    paint_textview(&draw_commands, starting_row, use_color_buffer);
                            }
                            KeyEvent::PageDown | KeyEvent::Ctrl('f') | KeyEvent::Char(' ') => {
                                if starting_row < (max_bottom_line - height) {
                                    starting_row += height;

                                    if starting_row > (max_bottom_line - height) {
                                        starting_row = max_bottom_line - height;
                                    }
                                }
                                max_bottom_line =
                                    paint_textview(&draw_commands, starting_row, use_color_buffer);
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }

                let new_size = terminal.terminal_size();
                if size != new_size {
                    size = new_size;
                    let _ = terminal.clear(crossterm::ClearType::All);
                    max_bottom_line =
                        paint_textview(&draw_commands, starting_row, use_color_buffer);
                }
            }
            let _ = cursor.show();

            let _ = RawScreen::disable_raw_mode();
        }
    }

    outln!("");
}

fn scroll_view(s: &str) {
    let mut v = vec![];
    for line in s.lines() {
        v.push(DrawCommand::DrawString(Style::default(), line.to_string()));
        v.push(DrawCommand::NextLine);
    }
    scroll_view_lines_if_needed(v, false);
}

fn view_text_value(value: &Tagged<Value>) {
    let value_anchor = value.anchor();
    match value.item {
        Value::Primitive(Primitive::String(ref s)) => {
            if let Some(source) = value_anchor {
                let extension: Option<String> = match source {
                    AnchorLocation::File(file) => {
                        let path = Path::new(&file);
                        path.extension().map(|x| x.to_string_lossy().to_string())
                    }
                    AnchorLocation::Url(url) => {
                        let url = url::Url::parse(&url);
                        if let Ok(url) = url {
                            let url = url.clone();
                            if let Some(mut segments) = url.path_segments() {
                                if let Some(file) = segments.next_back() {
                                    let path = Path::new(file);
                                    path.extension().map(|x| x.to_string_lossy().to_string())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    //FIXME: this probably isn't correct
                    AnchorLocation::Source(_source) => None,
                };

                match extension {
                    Some(extension) => {
                        // Load these once at the start of your program
                        let ps: SyntaxSet = syntect::dumps::from_binary(include_bytes!(
                            "../../assets/syntaxes.bin"
                        ));

                        if let Some(syntax) = ps.find_syntax_by_extension(&extension) {
                            let ts: ThemeSet = syntect::dumps::from_binary(include_bytes!(
                                "../../assets/themes.bin"
                            ));
                            let mut h = HighlightLines::new(syntax, &ts.themes["OneHalfDark"]);

                            let mut v = vec![];
                            for line in s.lines() {
                                let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);

                                for range in ranges {
                                    v.push(DrawCommand::DrawString(range.0, range.1.to_string()));
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
