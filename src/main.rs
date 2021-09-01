use nu_cli::{create_default_context, report_parsing_error, report_shell_error, NuHighlighter};
use nu_engine::eval_block;
use nu_parser::{ParserState, ParserWorkingSet};

#[cfg(test)]
mod tests;

fn main() -> std::io::Result<()> {
    let parser_state = create_default_context();

    if let Some(path) = std::env::args().nth(1) {
        let file = std::fs::read(&path)?;

        let (block, delta) = {
            let parser_state = parser_state.borrow();
            let mut working_set = ParserWorkingSet::new(&*parser_state);
            let (output, err) = working_set.parse_file(&path, &file, false);
            if let Some(err) = err {
                let _ = report_parsing_error(&working_set, &err);

                std::process::exit(1);
            }
            (output, working_set.render())
        };

        ParserState::merge_delta(&mut *parser_state.borrow_mut(), delta);

        let state = nu_engine::State {
            parser_state: parser_state.clone(),
            stack: nu_engine::Stack::new(),
        };

        match eval_block(&state, &block) {
            Ok(value) => {
                println!("{}", value.into_string());
            }
            Err(err) => {
                let parser_state = parser_state.borrow();
                let working_set = ParserWorkingSet::new(&*parser_state);

                let _ = report_shell_error(&working_set, &err);

                std::process::exit(1);
            }
        }

        Ok(())
    } else {
        use reedline::{DefaultPrompt, FileBackedHistory, Reedline, Signal};

        let mut line_editor = Reedline::new()
            .with_history(Box::new(FileBackedHistory::with_file(
                1000,
                "history.txt".into(),
            )?))?
            .with_highlighter(Box::new(NuHighlighter {
                parser_state: parser_state.clone(),
            }));

        let prompt = DefaultPrompt::new(1);
        let mut current_line = 1;
        let stack = nu_engine::Stack::new();

        loop {
            let input = line_editor.read_line(&prompt);
            match input {
                Ok(Signal::Success(s)) => {
                    if s.trim() == "exit" {
                        break;
                    }
                    // println!("input: '{}'", s);

                    let (block, delta) = {
                        let parser_state = parser_state.borrow();
                        let mut working_set = ParserWorkingSet::new(&*parser_state);
                        let (output, err) = working_set.parse_file(
                            &format!("line_{}", current_line),
                            s.as_bytes(),
                            false,
                        );
                        if let Some(err) = err {
                            let _ = report_parsing_error(&working_set, &err);
                            continue;
                        }
                        (output, working_set.render())
                    };

                    ParserState::merge_delta(&mut *parser_state.borrow_mut(), delta);

                    let state = nu_engine::State {
                        parser_state: parser_state.clone(),
                        stack: stack.clone(),
                    };

                    match eval_block(&state, &block) {
                        Ok(value) => {
                            println!("{}", value.into_string());
                        }
                        Err(err) => {
                            let parser_state = parser_state.borrow();
                            let working_set = ParserWorkingSet::new(&*parser_state);

                            let _ = report_shell_error(&working_set, &err);
                        }
                    }
                }
                Ok(Signal::CtrlC) => {
                    println!("Ctrl-c");
                }
                Ok(Signal::CtrlD) => {
                    break;
                }
                Ok(Signal::CtrlL) => {
                    line_editor.clear_screen()?;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                }
            }
            current_line += 1;
        }

        Ok(())
    }
}
