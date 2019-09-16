use ansi_term::Colour;
use crossterm::{cursor, terminal, ClearType, InputEvent, KeyEvent, RawScreen};
use std::io::Write;
use sublime_fuzzy::best_match;

pub fn select_from_list(lines: &Vec<&str>) {
    const MAX_RESULTS: usize = 5;
    #[derive(PartialEq)]
    enum State {
        Selecting,
        Quit,
        Selected(usize),
        Edit(usize),
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
            let search_result = search(&searchinput, &lines, MAX_RESULTS);
            let selected_lines: Vec<&str> = search_result
                .iter()
                .map(|item| &item.highlighted_text as &str)
                .collect();
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
                            state = State::Selected(search_result[selected].text_idx);
                        }
                        KeyEvent::Char('\t') | KeyEvent::Right => {
                            state = State::Edit(search_result[selected].text_idx);
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
                            // println!("{}", format!("OTHER InputEvent: {:?}\n\n", k));
                        }
                    },
                    _ => {}
                }
            }
            cursor.move_up(selected_lines.len() as u16);
        }
        let (_x, y) = cursor.pos();
        let _ = cursor.goto(0, y);
        let _ = cursor.show();

        let _ = RawScreen::disable_raw_mode();
    }
    let terminal = terminal();
    terminal.clear(ClearType::FromCursorDown).unwrap();

    match state {
        State::Selected(idx) => {
            print!("{}", lines[idx]);
        }
        State::Edit(idx) => {
            print!("{}", lines[idx]);
        }
        _ => {}
    }
}

struct Match {
    highlighted_text: String,
    text_idx: usize,
}

fn search(input: &String, lines: &Vec<&str>, max_results: usize) -> Vec<Match> {
    if input.is_empty() {
        return lines
            .iter()
            .take(max_results)
            .enumerate()
            .map(|(i, line)| Match {
                highlighted_text: line.to_string(),
                text_idx: i,
            })
            .collect();
    }

    let mut matches = lines
        .iter()
        .enumerate()
        .map(|(idx, line)| (idx, best_match(&input, line)))
        .filter(|(_i, m)| m.is_some())
        .map(|(i, m)| (i, m.unwrap()))
        .collect::<Vec<_>>();
    matches.sort_by(|a, b| b.1.score().cmp(&a.1.score()));

    let highlight = Colour::Cyan;
    let results: Vec<Match> = matches
        .iter()
        .take(max_results)
        .map(|(i, m)| {
            let r = &lines[*i];
            let mut outstr = String::with_capacity(r.len());
            let mut idx = 0;
            for (match_idx, len) in m.continuous_matches() {
                outstr.push_str(&r[idx..match_idx]);
                idx = match_idx + len;
                outstr.push_str(&format!("{}", highlight.paint(&r[match_idx..idx])));
            }
            if idx < r.len() {
                outstr.push_str(&r[idx..r.len()]);
            }
            Match {
                highlighted_text: outstr,
                text_idx: *i,
            }
        })
        .collect();
    results
}

fn paint_selection_list(lines: &Vec<&str>, selected: usize) {
    let dimmed = Colour::White.dimmed();
    let cursor = cursor();
    let (_x, y) = cursor.pos();
    for (i, line) in lines.iter().enumerate() {
        let _ = cursor.goto(0, y + (i as u16));
        if selected == i {
            println!("{}", *line);
        } else {
            println!("{}", dimmed.paint(*line));
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
