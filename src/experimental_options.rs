use std::borrow::Borrow;

use itertools::Itertools;

use nu_experimental::ParseWarning;
use nu_protocol::{
    cli_error::report_experimental_option_warning,
    engine::{EngineState, StateWorkingSet},
};

use crate::command::NushellCliArgs;

// 1. Parse experimental options from env
// 2. See if we should have any and disable all of them if not
// 3. Parse CLI arguments, if explicitly mentioned, let's enable them
pub fn load(engine_state: &EngineState, cli_args: &NushellCliArgs, has_script: bool) {
    let working_set = StateWorkingSet::new(&engine_state);

    let env_content = std::env::var(nu_experimental::ENV).unwrap_or_default();
    let env_offset = format!("{}=", nu_experimental::ENV).len();
    for (env_warning, span) in nu_experimental::parse_env() {
        let span_offset = (span.start + env_offset)..(span.end + env_offset);
        let mut diagnostic = miette::diagnostic!(
            severity = miette::Severity::Warning,
            code = code(&env_warning),
            labels = vec![miette::LabeledSpan::new_with_span(None, span_offset)],
            "{}",
            env_warning,
        );
        if let Some(help) = help(&env_warning) {
            diagnostic = diagnostic.with_help(help);
        }

        let error = miette::Error::from(diagnostic).with_source_code(format!(
            "{}={}",
            nu_experimental::ENV,
            env_content
        ));
        report_experimental_option_warning(&working_set, error.borrow());
    }

    if should_disable_experimental_options(has_script, cli_args) {
        for option in nu_experimental::ALL {
            // SAFETY:
            // The `set` method for experimental option is marked unsafe to warn consumers that
            // changing these values at runtime it risky.
            // At this point we're still in the initialization phase, so this should be fine.
            unsafe {
                option.set(false);
            }
        }
    }

    for (cli_arg_warning, ctx) in
        nu_experimental::parse_iter(cli_args.experimental_options.iter().flatten().map(|entry| {
            entry
                .item
                .split_once("=")
                .map(|(key, val)| (key.into(), Some(val.into()), entry))
                .unwrap_or((entry.item.clone().into(), None, entry))
        }))
    {
        let diagnostic = miette::diagnostic!(
            severity = miette::Severity::Warning,
            code = code(&cli_arg_warning),
            labels = vec![miette::LabeledSpan::new_with_span(None, ctx.span)],
            "{}",
            cli_arg_warning,
        );
        match help(&cli_arg_warning) {
            Some(help) => {
                report_experimental_option_warning(&working_set, &diagnostic.with_help(help))
            }
            None => report_experimental_option_warning(&working_set, &diagnostic),
        }
    }
}

fn should_disable_experimental_options(has_script: bool, cli_args: &NushellCliArgs) -> bool {
    has_script
        || cli_args.commands.is_some()
        || cli_args.execute.is_some()
        || cli_args.no_config_file.is_some()
        || cli_args.login_shell.is_some()
}

fn code(warning: &ParseWarning) -> &'static str {
    match warning {
        nu_experimental::ParseWarning::Unknown(_) => "nu::experimental_option::unknown",
        nu_experimental::ParseWarning::InvalidAssignment(_, _) => {
            "nu::experimental_option::invalid_assignment"
        }
        nu_experimental::ParseWarning::Deprecated(_) => "nu::experimental_option::deprecated",
    }
}

fn help(warning: &ParseWarning) -> Option<String> {
    match warning {
        ParseWarning::Unknown(_) => Some(format!(
            "known experimental options are: {}",
            nu_experimental::ALL
                .iter()
                .map(|option| option.identifier())
                .join(", ")
        )),
        ParseWarning::InvalidAssignment(_, _) => None,
        ParseWarning::Deprecated(_) => None,
    }
}
