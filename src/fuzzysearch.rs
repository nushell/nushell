use ansi_term::Colour;
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
    if let Ok(_raw) = RawScreen::into_raw_mode() {
        // User input for search
        let mut searchinput = String::new();
        let mut selected = 0;

        let mut cursor = cursor();
        let _ = cursor.hide();
        let input = crossterm::input();
        let mut sync_stdin = input.read_sync();

        while state == State::Selecting {
            let mut search_result = fuzzy_search(&searchinput, &lines, max_results);
            let selected_lines: Vec<String> =
                search_result.iter().map(|item| highlight(&item)).collect();
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
                            state = State::Selected(search_result.remove(selected).text);
                        }
                        KeyEvent::Char('\t') | KeyEvent::Right => {
                            state = State::Edit(search_result.remove(selected).text);
                        }
                        KeyEvent::Char(ch) => {
                            searchinput.push(ch);
                            selected = 0;
                        }
                        KeyEvent::Backspace => {
                            searchinput.pop();
                            selected = 0;
                        }
                        _ => {
                            // println!("OTHER InputEvent: {:?}", k);
                        }
                    },
                    _ => {}
                }
            }
            cursor.move_up(selected_lines.len() as u16);
        }
        let (_x, y) = cursor.pos();
        let _ = cursor.goto(0, y - 1);
        let _ = cursor.show();

        let _ = RawScreen::disable_raw_mode();
    }
    let terminal = terminal();
    terminal.clear(ClearType::FromCursorDown).unwrap();

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

fn highlight(textmatch: &Match) -> String {
    let hlcol = Colour::Cyan;
    let text = &textmatch.text;
    let mut outstr = String::with_capacity(text.len());
    let mut idx = 0;
    for (match_idx, len) in &textmatch.char_matches {
        outstr.push_str(&text[idx..*match_idx]);
        idx = match_idx + len;
        outstr.push_str(&format!("{}", hlcol.paint(&text[*match_idx..idx])));
    }
    if idx < text.len() {
        outstr.push_str(&text[idx..text.len()]);
    }
    outstr
}

fn paint_selection_list(lines: &Vec<String>, selected: usize) {
    let dimmed = Colour::White.dimmed();
    let cursor = cursor();
    let (_x, y) = cursor.pos();
    for (i, line) in lines.iter().enumerate() {
        let _ = cursor.goto(0, y + (i as u16));
        if selected == i {
            println!("{}", line);
        } else {
            println!("{}", dimmed.paint(line));
        }
    }
    let _ = cursor.goto(0, y + (lines.len() as u16));
    print!(
        "{}",
        Colour::Blue.paint("[ESC to quit, Enter to execute, Tab to edit]")
    );

    let _ = std::io::stdout().flush();
    // Clear additional lines from previous selection
    terminal().clear(ClearType::FromCursorDown).unwrap();
}

#[test]
fn fuzzy_match() {
    let matches = fuzzy_search("cb", &vec!["abc", "cargo build"], 1);
    assert_eq!(matches[0].text, "cargo build");
}
