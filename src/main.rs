use nu_cli::{report_parsing_error, report_shell_error, NuHighlighter};
use nu_command::create_default_context;
use nu_engine::eval_block;
use nu_parser::parse_file;
use nu_protocol::{
    engine::{EngineState, EvaluationContext, StateWorkingSet},
    Value,
};

#[cfg(test)]
mod tests;

fn main() -> std::io::Result<()> {
    let engine_state = create_default_context();

    if let Some(path) = std::env::args().nth(1) {
        let file = std::fs::read(&path)?;

        let (block, delta) = {
            let engine_state = engine_state.borrow();
            let mut working_set = StateWorkingSet::new(&*engine_state);
            let (output, err) = parse_file(&mut working_set, &path, &file, false);
            if let Some(err) = err {
                let _ = report_parsing_error(&working_set, &err);

                std::process::exit(1);
            }
            (output, working_set.render())
        };

        EngineState::merge_delta(&mut *engine_state.borrow_mut(), delta);

        let state = EvaluationContext {
            engine_state: engine_state.clone(),
            stack: nu_protocol::engine::Stack::new(),
        };

        match eval_block(&state, &block, Value::nothing()) {
            Ok(value) => {
                println!("{}", value.into_string());
            }
            Err(err) => {
                let engine_state = engine_state.borrow();
                let working_set = StateWorkingSet::new(&*engine_state);

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
                engine_state: engine_state.clone(),
            }));

        let prompt = DefaultPrompt::new(1);
        let mut current_line = 1;
        let stack = nu_protocol::engine::Stack::new();

        loop {
            let input = line_editor.read_line(&prompt);
            match input {
                Ok(Signal::Success(s)) => {
                    if s.trim() == "exit" {
                        break;
                    } else if s.trim() == "vars" {
                        engine_state.borrow().print_vars();
                        continue;
                    } else if s.trim() == "decls" {
                        engine_state.borrow().print_decls();
                        continue;
                    } else if s.trim() == "blocks" {
                        engine_state.borrow().print_blocks();
                        continue;
                    } else if s.trim() == "stack" {
                        stack.print_stack();
                        continue;
                    }

                    let (block, delta) = {
                        let engine_state = engine_state.borrow();
                        let mut working_set = StateWorkingSet::new(&*engine_state);
                        let (output, err) = parse_file(
                            &mut working_set,
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

                    EngineState::merge_delta(&mut *engine_state.borrow_mut(), delta);

                    let state = nu_protocol::engine::EvaluationContext {
                        engine_state: engine_state.clone(),
                        stack: stack.clone(),
                    };

                    match eval_block(&state, &block, Value::nothing()) {
                        Ok(value) => {
                            println!("{}", value.into_string());
                        }
                        Err(err) => {
                            let engine_state = engine_state.borrow();
                            let working_set = StateWorkingSet::new(&*engine_state);

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
