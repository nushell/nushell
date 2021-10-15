use std::{cell::RefCell, io::Write, rc::Rc};

use dialoguer::{console::Term, theme::ColorfulTheme, Select};
use miette::{IntoDiagnostic, Result};
use nu_cli::{report_error, NuCompleter, NuHighlighter, NuValidator, NushellPrompt};
use nu_command::create_default_context;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    ast::Call,
    engine::{EngineState, EvaluationContext, Stack, StateWorkingSet},
    ShellError, Value,
};
use reedline::{Completer, CompletionActionHandler, DefaultPrompt, LineBuffer, Prompt};

#[cfg(test)]
mod tests;

// Name of environment variable where the prompt could be stored
const PROMPT_COMMAND: &str = "PROMPT_COMMAND";

struct FuzzyCompletion {
    completer: Box<dyn Completer>,
}

impl CompletionActionHandler for FuzzyCompletion {
    fn handle(&mut self, present_buffer: &mut LineBuffer) {
        let completions = self
            .completer
            .complete(present_buffer.get_buffer(), present_buffer.offset());

        if completions.is_empty() {
            // do nothing
        } else if completions.len() == 1 {
            let span = completions[0].0;

            let mut offset = present_buffer.offset();
            offset += completions[0].1.len() - (span.end - span.start);

            // TODO improve the support for multiline replace
            present_buffer.replace(span.start..span.end, &completions[0].1);
            present_buffer.set_insertion_point(offset);
        } else {
            let selections: Vec<_> = completions.iter().map(|(_, string)| string).collect();

            let _ = crossterm::terminal::disable_raw_mode();
            println!();
            let result = Select::with_theme(&ColorfulTheme::default())
                .default(0)
                .items(&selections[..])
                .interact_on_opt(&Term::stdout())
                .unwrap();
            let _ = crossterm::terminal::enable_raw_mode();

            match result {
                Some(result) => {
                    let span = completions[result].0;

                    let mut offset = present_buffer.offset();
                    offset += completions[result].1.len() - (span.end - span.start);

                    // TODO improve the support for multiline replace
                    present_buffer.replace(span.start..span.end, &completions[result].1);
                    present_buffer.set_insertion_point(offset);
                }
                None => {}
            }
        }
    }
}

fn main() -> Result<()> {
    miette::set_panic_hook();
    let miette_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |x| {
        crossterm::terminal::disable_raw_mode().unwrap();
        miette_hook(x);
    }));

    let engine_state = create_default_context();

    if let Some(path) = std::env::args().nth(1) {
        let file = std::fs::read(&path).into_diagnostic()?;

        let (block, delta) = {
            let engine_state = engine_state.borrow();
            let mut working_set = StateWorkingSet::new(&*engine_state);
            let (output, err) = parse(&mut working_set, Some(&path), &file, false);
            if let Some(err) = err {
                report_error(&working_set, &err);

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

                report_error(&working_set, &err);

                std::process::exit(1);
            }
        }

        Ok(())
    } else {
        use reedline::{FileBackedHistory, Reedline, Signal};

        let completer = NuCompleter::new(engine_state.clone());
        let mut entry_num = 0;

        let mut line_editor = Reedline::create()
            .into_diagnostic()?
            .with_history(Box::new(
                FileBackedHistory::with_file(1000, "history.txt".into()).into_diagnostic()?,
            ))
            .into_diagnostic()?
            .with_highlighter(Box::new(NuHighlighter {
                engine_state: engine_state.clone(),
            }))
            .with_completion_action_handler(Box::new(FuzzyCompletion {
                completer: Box::new(completer),
            }))
            // .with_completion_action_handler(Box::new(
            //     ListCompletionHandler::default().with_completer(Box::new(completer)),
            // ))
            .with_validator(Box::new(NuValidator {
                engine_state: engine_state.clone(),
            }));

        let default_prompt = DefaultPrompt::new(1);
        let mut nu_prompt = NushellPrompt::new();
        let stack = nu_protocol::engine::Stack::new();

        // Load config startup file
        if let Some(mut config_path) = nu_path::config_dir() {
            config_path.push("nushell");
            config_path.push("config.nu");

            // FIXME: remove this message when we're ready
            println!("Loading config from: {:?}", config_path);

            if config_path.exists() {
                let config_filename = config_path.to_string_lossy().to_owned();

                if let Ok(contents) = std::fs::read_to_string(&config_path) {
                    eval_source(engine_state.clone(), &stack, &contents, &config_filename);
                }
            }
        }

        loop {
            let prompt = update_prompt(
                PROMPT_COMMAND,
                engine_state.clone(),
                &stack,
                &mut nu_prompt,
                &default_prompt,
            );

            entry_num += 1;

            let input = line_editor.read_line(prompt);
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
                    } else if s.trim() == "contents" {
                        engine_state.borrow().print_contents();
                        continue;
                    }

                    eval_source(
                        engine_state.clone(),
                        &stack,
                        &s,
                        &format!("entry #{}", entry_num),
                    );
                }
                Ok(Signal::CtrlC) => {
                    println!("Ctrl-c");
                }
                Ok(Signal::CtrlD) => {
                    break;
                }
                Ok(Signal::CtrlL) => {
                    line_editor.clear_screen().into_diagnostic()?;
                }
                Err(err) => {
                    let message = err.to_string();
                    if !message.contains("duration") {
                        println!("Error: {:?}", err);
                    }
                }
            }
        }

        Ok(())
    }
}

fn print_value(value: Value, state: &EvaluationContext) -> Result<(), ShellError> {
    // If the table function is in the declarations, then we can use it
    // to create the table value that will be printed in the terminal
    let engine_state = state.engine_state.borrow();
    let output = match engine_state.find_decl("table".as_bytes()) {
        Some(decl_id) => {
            let table = engine_state
                .get_decl(decl_id)
                .run(state, &Call::new(), value)?;
            table.into_string()
        }
        None => value.into_string(),
    };
    let stdout = std::io::stdout();

    match stdout.lock().write_all(output.as_bytes()) {
        Ok(_) => (),
        Err(err) => eprintln!("{}", err),
    };

    Ok(())
}

fn update_prompt<'prompt>(
    env_variable: &str,
    engine_state: Rc<RefCell<EngineState>>,
    stack: &Stack,
    nu_prompt: &'prompt mut NushellPrompt,
    default_prompt: &'prompt DefaultPrompt,
) -> &'prompt dyn Prompt {
    let prompt_command = match stack.get_env_var(env_variable) {
        Some(prompt) => prompt,
        None => return default_prompt as &dyn Prompt,
    };

    // Checking if the PROMPT_COMMAND is the same to avoid evaluating constantly
    // the same command, thus saturating the contents in the EngineState
    if !nu_prompt.is_new_prompt(prompt_command.as_str()) {
        return nu_prompt as &dyn Prompt;
    }

    let (block, delta) = {
        let ref_engine_state = engine_state.borrow();
        let mut working_set = StateWorkingSet::new(&ref_engine_state);
        let (output, err) = parse(&mut working_set, None, prompt_command.as_bytes(), false);
        if let Some(err) = err {
            report_error(&working_set, &err);
            return default_prompt as &dyn Prompt;
        }
        (output, working_set.render())
    };

    EngineState::merge_delta(&mut *engine_state.borrow_mut(), delta);

    let state = nu_protocol::engine::EvaluationContext {
        engine_state: engine_state.clone(),
        stack: stack.clone(),
    };

    let evaluated_prompt = match eval_block(&state, &block, Value::nothing()) {
        Ok(value) => match value.as_string() {
            Ok(prompt) => prompt,
            Err(err) => {
                let engine_state = engine_state.borrow();
                let working_set = StateWorkingSet::new(&*engine_state);
                report_error(&working_set, &err);
                return default_prompt as &dyn Prompt;
            }
        },
        Err(err) => {
            let engine_state = engine_state.borrow();
            let working_set = StateWorkingSet::new(&*engine_state);
            report_error(&working_set, &err);
            return default_prompt as &dyn Prompt;
        }
    };

    nu_prompt.update_prompt(prompt_command, evaluated_prompt);

    nu_prompt as &dyn Prompt
}

fn eval_source(
    engine_state: Rc<RefCell<EngineState>>,
    stack: &Stack,
    source: &str,
    fname: &str,
) -> bool {
    let (block, delta) = {
        let engine_state = engine_state.borrow();
        let mut working_set = StateWorkingSet::new(&*engine_state);
        let (output, err) = parse(
            &mut working_set,
            Some(fname), // format!("entry #{}", entry_num)
            source.as_bytes(),
            false,
        );
        if let Some(err) = err {
            report_error(&working_set, &err);
            return false;
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
            if let Err(err) = print_value(value, &state) {
                let engine_state = engine_state.borrow();
                let working_set = StateWorkingSet::new(&*engine_state);

                report_error(&working_set, &err);
                return false;
            }
        }
        Err(err) => {
            let engine_state = engine_state.borrow();
            let working_set = StateWorkingSet::new(&*engine_state);

            report_error(&working_set, &err);
            return false;
        }
    }

    true
}
