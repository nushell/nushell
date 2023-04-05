use nu_protocol::{
    ast::RangeInclusion,
    engine::{EngineState, Stack, StateWorkingSet},
    CliError, Range, ShellError, Span, Value,
};
use std::path::PathBuf;

pub fn report_error(
    working_set: &StateWorkingSet,
    error: &(dyn miette::Diagnostic + Send + Sync + 'static),
) {
    eprintln!("Error: {:?}", CliError(error, working_set));
    // reset vt processing, aka ansi because illbehaved externals can break it
    #[cfg(windows)]
    {
        let _ = nu_utils::enable_vt_processing();
    }
}

pub fn report_error_new(
    engine_state: &EngineState,
    error: &(dyn miette::Diagnostic + Send + Sync + 'static),
) {
    let working_set = StateWorkingSet::new(engine_state);

    report_error(&working_set, error);
}

pub fn get_init_cwd() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| {
        std::env::var("PWD")
            .map(Into::into)
            .unwrap_or_else(|_| nu_path::home_dir().unwrap_or_default())
    })
}

pub fn get_guaranteed_cwd(engine_state: &EngineState, stack: &Stack) -> PathBuf {
    nu_engine::env::current_dir(engine_state, stack).unwrap_or_else(|e| {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &e);
        crate::util::get_init_cwd()
    })
}

type MakeRangeError = fn(&str, Span) -> ShellError;

pub fn process_range(range: &Range) -> Result<(isize, isize), MakeRangeError> {
    let start = match &range.from {
        Value::Int { val, .. } => *val as isize,
        Value::Nothing { .. } => 0,
        _ => {
            return Err(|msg, span| ShellError::TypeMismatch {
                err_message: msg.to_string(),
                span,
            })
        }
    };

    let end = match &range.to {
        Value::Int { val, .. } => {
            if matches!(range.inclusion, RangeInclusion::Inclusive) {
                *val as isize
            } else {
                *val as isize - 1
            }
        }
        Value::Nothing { .. } => isize::max_value(),
        _ => {
            return Err(|msg, span| ShellError::TypeMismatch {
                err_message: msg.to_string(),
                span,
            })
        }
    };

    Ok((start, end))
}
