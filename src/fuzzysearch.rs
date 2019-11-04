use ansi_term::{ANSIString, ANSIStrings, Colour, Style};
#[cfg(feature = "crossterm")]
use crossterm::{cursor, terminal, ClearType, InputEvent, KeyEvent, RawScreen};
use std::io::Write;
use sublime_fuzzy::best_match;

pub enum SelectionResult {
    Selected(String),
    Edit(String),
    NoSelection,
}

pub fn interactive_fuzzy_search(lines: &Vec<&str>, max_results: usize) -> SelectionResult {
    #[derive(PartialEq)]
    enum State {
        Selecting,
        Quit,
        Selected(String),
        Edit(String),
    }
    let mut state = State::Selecting;
    #[cfg(feature = "crossterm")]
    {
        if let Ok(_raw) = RawScreen::into_raw_mode() {
            // User input for search
            let mut searchinput = String::new();
            let mut selected = 0;

            let mut cursor = cursor();
            let _ = cursor.hide();
            let input = crossterm::input();
            let mut sync_stdin = input.read_sync();

            while state == State::Selecting {
                let mut selected_lines = fuzzy_search(&searchinput, &lines, max_results);
                let num_lines = selected_lines.len();
                paint_selection_list(&selected_lines, selected);
                if let Some(ev) = sync_stdin.next() {
                    match ev {
                        InputEvent::Keyboard(k) => match k {
                            KeyEvent::Esc | KeyEvent::Ctrl('c') => {
                                state = State::Quit;
                            }
                            KeyEvent::Up => {
                                if selected > 0 {
                                    selected -= 1;
                                }
                            }
                            KeyEvent::Down => {
                                if selected + 1 < selected_lines.len() {
                                    selected += 1;
                                }
                            }
                            KeyEvent::Char('\n') => {
                                state = if selected_lines.len() > 0 {
                                    State::Selected(selected_lines.remove(selected).text)
                                } else {
                                    State::Edit("".to_string())
                                };
                            }
                            KeyEvent::Char('\t') | KeyEvent::Right => {
                                state = if selected_lines.len() > 0 {
                                    State::Edit(selected_lines.remove(selected).text)
                                } else {
                                    State::Edit("".to_string())
                                };
                            }
                            KeyEvent::Char(ch) => {
                                searchinput.push(ch);
                                selected = 0;
                            }
                            KeyEvent::Backspace => {
                                searchinput.pop();
                                selected = 0;
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
                if num_lines > 0 {
                    cursor.move_up(num_lines as u16);
                }
            }
            let (_x, y) = cursor.pos();
            let _ = cursor.goto(0, y - 1);
            let _ = cursor.show();
            let _ = RawScreen::disable_raw_mode();
        }
        terminal().clear(ClearType::FromCursorDown).unwrap();
    }
    match state {
        State::Selected(line) => SelectionResult::Selected(line),
        State::Edit(line) => SelectionResult::Edit(line),
        _ => SelectionResult::NoSelection,
    }
}

pub struct Match {
    text: String,
    char_matches: Vec<(usize, usize)>,
}

pub fn fuzzy_search(searchstr: &str, lines: &Vec<&str>, max_results: usize) -> Vec<Match> {
    if searchstr.is_empty() {
        return lines
            .iter()
            .take(max_results)
            .map(|line| Match {
                text: line.to_string(),
                char_matches: Vec::new(),
            })
            .collect();
    }

    let mut matches = lines
        .iter()
        .enumerate()
        .map(|(idx, line)| (idx, best_match(&searchstr, line)))
        .filter(|(_i, m)| m.is_some())
        .map(|(i, m)| (i, m.unwrap()))
        .collect::<Vec<_>>();
    matches.sort_by(|a, b| b.1.score().cmp(&a.1.score()));

    let results: Vec<Match> = matches
        .iter()
        .take(max_results)
        .map(|(i, m)| Match {
            text: lines[*i].to_string(),
            char_matches: m.continuous_matches(),
        })
        .collect();
    results
}

#[cfg(feature = "crossterm")]
fn highlight(textmatch: &Match, normal: Style, highlighted: Style) -> Vec<ANSIString> {
    let text = &textmatch.text;
    let mut ansi_strings = vec![];
    let mut idx = 0;
    for (match_idx, len) in &textmatch.char_matches {
        ansi_strings.push(normal.paint(&text[idx..*match_idx]));
        idx = match_idx + len;
        ansi_strings.push(highlighted.paint(&text[*match_idx..idx]));
    }
    if idx < text.len() {
        ansi_strings.push(normal.paint(&text[idx..text.len()]));
    }
    ansi_strings
}

#[cfg(feature = "crossterm")]
fn paint_selection_list(lines: &Vec<Match>, selected: usize) {
    let terminal = terminal();
    let size = terminal.terminal_size();
    let width = size.0 as usize;
    let cursor = cursor();
    let (_x, y) = cursor.pos();
    for (i, line) in lines.iter().enumerate() {
        let _ = cursor.goto(0, y + (i as u16));
        let (style, highlighted) = if selected == i {
            (Colour::White.normal(), Colour::Cyan.normal())
        } else {
            (Colour::White.dimmed(), Colour::Cyan.normal())
        };
        let mut ansi_strings = highlight(line, style, highlighted);
        for _ in line.text.len()..width {
            ansi_strings.push(style.paint(' '.to_string()));
        }
        outln!("{}", ANSIStrings(&ansi_strings));
    }
    let _ = cursor.goto(0, y + (lines.len() as u16));
    print!(
        "{}",
        Colour::Blue.paint("[ESC to quit, Enter to execute, Tab to edit]")
    );

    let _ = std::io::stdout().flush();
    // Clear additional lines from previous selection
    terminal.clear(ClearType::FromCursorDown).unwrap();
}

#[test]
fn fuzzy_match() {
    let matches = fuzzy_search("cb", &vec!["abc", "cargo build"], 1);
    assert_eq!(matches[0].text, "cargo build");
}
