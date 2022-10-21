use crate::util::{eval_source, report_error};
use log::info;
use log::trace;
use miette::{IntoDiagnostic, Result};
use nu_engine::convert_env_values;
use nu_parser::parse;
use nu_protocol::Type;
use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack, StateWorkingSet},
    Config, PipelineData, Span, Value,
};
use nu_utils::stdout_write_all_and_flush;

/// Main function used when a file path is found as argument for nu
pub fn evaluate_file(
    path: String,
    args: &[String],
    engine_state: &mut EngineState,
    stack: &mut Stack,
    input: PipelineData,
) -> Result<()> {
    // Translate environment variables from Strings to Values
    if let Some(e) = convert_env_values(engine_state, stack) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &e);
        std::process::exit(1);
    }

    let file = std::fs::read(&path).into_diagnostic()?;

    let mut working_set = StateWorkingSet::new(engine_state);
    trace!("parsing file: {}", path);

    let _ = parse(&mut working_set, Some(&path), &file, false, &[]);

    if working_set.find_decl(b"main", &Type::Any).is_some() {
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

    info!("evaluate {}:{}:{}", file!(), line!(), column!());

    Ok(())
}

pub fn print_table_or_error(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    mut pipeline_data: PipelineData,
    config: &mut Config,
) -> Option<i64> {
    let exit_code = match &mut pipeline_data {
        PipelineData::ExternalStream { exit_code, .. } => exit_code.take(),
        _ => None,
    };

    // Change the engine_state config to use the passed in configuration
    engine_state.set_config(config);

    match engine_state.find_decl("table".as_bytes(), &[]) {
        Some(decl_id) => {
            let command = engine_state.get_decl(decl_id);
            if command.get_block_id().is_some() {
                print_or_exit(pipeline_data, engine_state, config);
            } else {
                let table = command.run(
                    engine_state,
                    stack,
                    &Call::new(Span::new(0, 0)),
                    pipeline_data,
                );

                match table {
                    Ok(table) => {
                        print_or_exit(table, engine_state, config);
                    }
                    Err(error) => {
                        let working_set = StateWorkingSet::new(engine_state);

                        report_error(&working_set, &error);

                        std::process::exit(1);
                    }
                }
            }
        }
        None => {
            print_or_exit(pipeline_data, engine_state, config);
        }
    };

    // Make sure everything has finished
    if let Some(exit_code) = exit_code {
        let mut exit_code: Vec<_> = exit_code.into_iter().collect();
        exit_code
            .pop()
            .and_then(|last_exit_code| match last_exit_code {
                Value::Int { val: code, .. } => Some(code),
                _ => None,
            })
    } else {
        None
    }
}

fn print_or_exit(pipeline_data: PipelineData, engine_state: &mut EngineState, config: &Config) {
    for item in pipeline_data {
        if let Value::Error { error } = item {
            let working_set = StateWorkingSet::new(engine_state);

            report_error(&working_set, &error);

            std::process::exit(1);
        }

        let mut out = item.into_string("\n", config);
        out.push('\n');

        let _ = stdout_write_all_and_flush(out).map_err(|err| eprintln!("{}", err));
    }
}
