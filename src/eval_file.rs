use crate::is_perf_true;
use crate::utils::{eval_source, gather_parent_env_vars, report_error};
use log::info;
use log::trace;
use miette::{IntoDiagnostic, Result};
use nu_engine::convert_env_values;
use nu_parser::parse;
use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack, StateWorkingSet},
    Config, PipelineData, Span, Value, CONFIG_VARIABLE_ID,
};
use std::io::Write;

/// Main function used when a file path is found as argument for nu
pub(crate) fn evaluate(
    path: String,
    args: &[String],
    engine_state: &mut EngineState,
    input: PipelineData,
) -> Result<()> {
    // First, set up env vars as strings only
    gather_parent_env_vars(engine_state);

    let mut stack = nu_protocol::engine::Stack::new();

    // Set up our initial config to start from
    stack.vars.insert(
        CONFIG_VARIABLE_ID,
        Value::Record {
            cols: vec![],
            vals: vec![],
            span: Span { start: 0, end: 0 },
        },
    );

    let config = match stack.get_config() {
        Ok(config) => config,
        Err(e) => {
            let working_set = StateWorkingSet::new(engine_state);

            report_error(&working_set, &e);
            Config::default()
        }
    };

    // Translate environment variables from Strings to Values
    if let Some(e) = convert_env_values(engine_state, &stack, &config) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &e);
        std::process::exit(1);
    }

    let file = std::fs::read(&path).into_diagnostic()?;

    let mut working_set = StateWorkingSet::new(engine_state);
    trace!("parsing file: {}", path);

    let _ = parse(&mut working_set, Some(&path), &file, false);

    if working_set.find_decl(b"main").is_some() {
        let args = format!("main {}", args.join(" "));

        eval_source(
            engine_state,
            &mut stack,
            &file,
            &path,
            PipelineData::new(Span::new(0, 0)),
        );
        eval_source(
            engine_state,
            &mut stack,
            args.as_bytes(),
            "<commandline>",
            input,
        );
    } else {
        eval_source(engine_state, &mut stack, &file, &path, input);
    }

    // let (block, delta) = {
    //     let mut working_set = StateWorkingSet::new(engine_state);
    //     trace!("parsing file: {}", path);

    //     let (output, err) = parse(&mut working_set, Some(&path), &file, false);
    //     if let Some(err) = err {
    //         report_error(&working_set, &err);

    //         std::process::exit(1);
    //     }
    //     (output, working_set.render())
    // };

    // if let Err(err) = engine_state.merge_delta(delta, None, &init_cwd) {
    //     let working_set = StateWorkingSet::new(engine_state);
    //     report_error(&working_set, &err);
    // }

    // let mut stack = nu_protocol::engine::Stack::new();

    // // Set up our initial config to start from
    // stack.vars.insert(
    //     CONFIG_VARIABLE_ID,
    //     Value::Record {
    //         cols: vec![],
    //         vals: vec![],
    //         span: Span { start: 0, end: 0 },
    //     },
    // );

    // let config = match stack.get_config() {
    //     Ok(config) => config,
    //     Err(e) => {
    //         let working_set = StateWorkingSet::new(engine_state);

    //         report_error(&working_set, &e);
    //         Config::default()
    //     }
    // };

    // // Merge the delta in case env vars changed in the config
    // match nu_engine::env::current_dir(engine_state, &stack) {
    //     Ok(cwd) => {
    //         if let Err(e) = engine_state.merge_delta(StateDelta::new(), Some(&mut stack), cwd) {
    //             let working_set = StateWorkingSet::new(engine_state);
    //             report_error(&working_set, &e);
    //         }
    //     }
    //     Err(e) => {
    //         let working_set = StateWorkingSet::new(engine_state);
    //         report_error(&working_set, &e);
    //     }
    // }

    // // Translate environment variables from Strings to Values
    // if let Some(e) = convert_env_values(engine_state, &stack, &config) {
    //     let working_set = StateWorkingSet::new(engine_state);
    //     report_error(&working_set, &e);
    //     std::process::exit(1);
    // }

    // // Next, let's check if there are any flags we want to pass to the main function
    // if args.is_empty() && engine_state.find_decl(b"main").is_none() {
    //     // We don't have a main, so evaluate the whole file
    //     match eval_block(engine_state, &mut stack, &block, input) {
    //         Ok(pipeline_data) => {
    //             print_table_or_error(engine_state, &mut stack, pipeline_data, &config)
    //         }
    //         Err(err) => {
    //             let working_set = StateWorkingSet::new(engine_state);

    //             report_error(&working_set, &err);

    //             std::process::exit(1);
    //         }
    //     }
    // } else {
    //     let args = format!("main {}", args.join(" "));

    //     let (block, delta) = {
    //         let mut working_set = StateWorkingSet::new(engine_state);
    //         let (output, err) = parse(&mut working_set, Some("<cmdline>"), &args, false);
    //         if let Some(err) = err {
    //             report_error(&working_set, &err);

    //             std::process::exit(1);
    //         }
    //         (output, working_set.render())
    //     };

    //     let cwd = nu_engine::env::current_dir_str(engine_state, &stack)?;

    //     if let Err(err) = engine_state.merge_delta(delta, Some(&mut stack), &cwd) {
    //         let working_set = StateWorkingSet::new(engine_state);
    //         report_error(&working_set, &err);
    //     }

    //     match eval_block(engine_state, &mut stack, &block, input) {
    //         Ok(pipeline_data) => {
    //             print_table_or_error(engine_state, &mut stack, pipeline_data, &config)
    //         }
    //         Err(err) => {
    //             let working_set = StateWorkingSet::new(engine_state);

    //             report_error(&working_set, &err);

    //             std::process::exit(1);
    //         }
    //     }
    // }

    if is_perf_true() {
        info!("evaluate {}:{}:{}", file!(), line!(), column!());
    }

    Ok(())
}

pub fn print_table_or_error(
    engine_state: &EngineState,
    stack: &mut Stack,
    pipeline_data: PipelineData,
    config: &Config,
) {
    match engine_state.find_decl("table".as_bytes()) {
        Some(decl_id) => {
            let table = engine_state.get_decl(decl_id).run(
                engine_state,
                stack,
                &Call::new(Span::new(0, 0)),
                pipeline_data,
            );

            match table {
                Ok(table) => {
                    for item in table {
                        let stdout = std::io::stdout();

                        if let Value::Error { error } = item {
                            let working_set = StateWorkingSet::new(engine_state);

                            report_error(&working_set, &error);

                            std::process::exit(1);
                        }

                        let mut out = item.into_string("\n", config);
                        out.push('\n');

                        match stdout.lock().write_all(out.as_bytes()) {
                            Ok(_) => (),
                            Err(err) => eprintln!("{}", err),
                        };
                    }
                }
                Err(error) => {
                    let working_set = StateWorkingSet::new(engine_state);

                    report_error(&working_set, &error);

                    std::process::exit(1);
                }
            }
        }
        None => {
            for item in pipeline_data {
                let stdout = std::io::stdout();

                if let Value::Error { error } = item {
                    let working_set = StateWorkingSet::new(engine_state);

                    report_error(&working_set, &error);

                    std::process::exit(1);
                }

                let mut out = item.into_string("\n", config);
                out.push('\n');

                match stdout.lock().write_all(out.as_bytes()) {
                    Ok(_) => (),
                    Err(err) => eprintln!("{}", err),
                };
            }
        }
    };
}
