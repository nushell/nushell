use crate::util::{eval_source, report_error};
use log::info;
use log::trace;
use miette::{IntoDiagnostic, Result};
use nu_engine::convert_env_values;
use nu_parser::parse;
use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack, StateWorkingSet},
    Config, PipelineData, Span, Value,
};
use std::io::Write;

/// Main function used when a file path is found as argument for nu
pub fn evaluate_file(
    path: String,
    args: &[String],
    engine_state: &mut EngineState,
    stack: &mut Stack,
    input: PipelineData,
    is_perf_true: bool,
) -> Result<()> {
    // Translate environment variables from Strings to Values
    if let Some(e) = convert_env_values(engine_state, stack) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &e);
        std::process::exit(1);
    }

    // Make a note of the exceptions we see for externals that look like math expressions
    let exceptions = crate::util::external_exceptions(engine_state, stack);
    engine_state.external_exceptions = exceptions;

    let file = std::fs::read(&path).into_diagnostic()?;

    let mut working_set = StateWorkingSet::new(engine_state);
    trace!("parsing file: {}", path);

    let _ = parse(&mut working_set, Some(&path), &file, false);

    if working_set.find_decl(b"main").is_some() {
        let args = format!("main {}", args.join(" "));

        if !eval_source(
            engine_state,
            stack,
            &file,
            &path,
            PipelineData::new(Span::new(0, 0)),
        ) {
            std::process::exit(1);
        }
        if !eval_source(engine_state, stack, args.as_bytes(), "<commandline>", input) {
            std::process::exit(1);
        }
    } else if !eval_source(engine_state, stack, &file, &path, input) {
        std::process::exit(1);
    }

    if is_perf_true {
        info!("evaluate {}:{}:{}", file!(), line!(), column!());
    }

    Ok(())
}

pub fn print_table_or_error(
    engine_state: &EngineState,
    stack: &mut Stack,
    mut pipeline_data: PipelineData,
    config: &Config,
) {
    let exit_code = match &mut pipeline_data {
        PipelineData::ExternalStream { exit_code, .. } => exit_code.take(),
        _ => None,
    };

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

    // Make sure everything has finished
    if let Some(exit_code) = exit_code {
        let _: Vec<_> = exit_code.into_iter().collect();
    }
}
