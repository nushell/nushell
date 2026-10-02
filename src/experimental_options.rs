use std::{
    borrow::Borrow,
    fs::File,
    io::{Read, Seek},
};

use nu_parser::pickle::{self, PickleHeader};
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    report_error::report_experimental_option_warning,
};

use crate::command::NushellCliArgs;

// 1. Parse experimental options from env
// 2. See if we should have any and disable all of them if not
// 3. Parse CLI arguments, if explicitly mentioned, let's enable them
// 4. If the script is a pickle, use the options it was made with instead
pub fn load(engine_state: &EngineState, cli_args: &NushellCliArgs, script_name: &str) {
    let working_set = StateWorkingSet::new(engine_state);
    let has_script = !script_name.is_empty();

    if !should_disable_experimental_options(has_script, cli_args) {
        let env_content = std::env::var(nu_experimental::ENV).unwrap_or_default();
        let env_offset = format!("{}=", nu_experimental::ENV).len();

        for (env_warning, span) in nu_experimental::parse_env() {
            let span_offset = (span.start + env_offset)..(span.end + env_offset);
            let mut diagnostic = miette::diagnostic!(
                severity = miette::Severity::Warning,
                code = env_warning.code(),
                labels = vec![miette::LabeledSpan::new_with_span(None, span_offset)],
                "{}",
                env_warning,
            );
            if let Some(help) = env_warning.help() {
                diagnostic = diagnostic.with_help(help);
            }

            let error = miette::Error::from(diagnostic).with_source_code(format!(
                "{}={}",
                nu_experimental::ENV,
                env_content
            ));
            report_experimental_option_warning(None, &working_set, error.borrow());
        }
    }

    for (cli_arg_warning, ctx) in
        nu_experimental::parse_iter(cli_args.experimental_options.iter().flatten().map(|entry| {
            let cleaned = entry
                .item
                .trim()
                .trim_matches(['[', ']'])
                .trim_end_matches(',');
            cleaned
                .split_once("=")
                .map(|(key, val)| (key.into(), Some(val.into()), entry))
                .unwrap_or((cleaned.into(), None, entry))
        }))
    {
        // Skip labels when span is unknown (CLI args parsed before engine setup)
        let labels = if ctx.span == nu_protocol::Span::unknown() {
            Vec::new()
        } else {
            vec![miette::LabeledSpan::new_with_span(None, ctx.span)]
        };
        let diagnostic = miette::diagnostic!(
            severity = miette::Severity::Warning,
            code = cli_arg_warning.code(),
            labels = labels,
            "{}",
            cli_arg_warning,
        );
        match cli_arg_warning.help() {
            Some(help) => {
                report_experimental_option_warning(None, &working_set, &diagnostic.with_help(help))
            }
            None => report_experimental_option_warning(None, &working_set, &diagnostic),
        }
    }

    // The options can change what the parser and compiler emit, so a pickle runs with the ones it
    // was compiled with. The pickle records every option, so none of the ones set above is left
    // over.
    if let Some(header) = pickle_header(script_name) {
        if cli_args.experimental_options.is_some() {
            let diagnostic = miette::diagnostic!(
                severity = miette::Severity::Warning,
                code = "nu::experimental_option::pickle",
                help = "pickle the script again with the options it should run with",
                "`--experimental-options` doesn't apply to a pickle, which runs with the \
experimental options it was made with",
            );
            report_experimental_option_warning(None, &working_set, &diagnostic);
        }
        for (name, enabled) in header.experimental_options {
            if let Some(option) = nu_experimental::ALL
                .iter()
                .find(|option| option.identifier() == name)
            {
                // SAFETY: This runs at initialization, before anything reads the options.
                unsafe { option.set(enabled) };
            }
        }
    }
}

/// The header of the pickle `script_name` names, found the way `evaluate_file` finds the script.
/// `None` if it isn't a pickle or can't be read, which running it reports properly later.
fn pickle_header(script_name: &str) -> Option<PickleHeader> {
    let path = nu_path::absolute_with(script_name, std::env::current_dir().ok()?).ok()?;
    // Only a regular file can be a pickle. Reading a pipe or FIFO here would take its bytes from
    // the script `evaluate_file` reads.
    if !path.metadata().is_ok_and(|metadata| metadata.is_file()) {
        return None;
    }
    let mut file = File::open(path).ok()?;
    // Source code is told apart by its first bytes, only a pickle is read whole.
    let mut contents = vec![];
    (&mut file)
        .take(pickle::MAGIC.len() as u64)
        .read_to_end(&mut contents)
        .ok()?;
    if pickle::is_pickle(&contents) {
        file.read_to_end(&mut contents).ok()?;
    }
    // `/dev/stdin` redirected from a file shares its offset with the one `evaluate_file` opens.
    file.rewind().ok()?;
    pickle::info(&contents).ok()?.header
}

// Disable experimental options when not loading config files (for NU_EXPERIMENTAL_OPTIONS env).
fn should_disable_experimental_options(has_script: bool, cli_args: &NushellCliArgs) -> bool {
    let no_config_flag = cli_args.no_config_file.is_some();
    let running_script_without_config =
        has_script && cli_args.config_file.is_none() && cli_args.env_file.is_none();
    let running_command_without_config = cli_args.commands.is_some()
        && cli_args.login_shell.is_none()
        && cli_args.config_file.is_none()
        && cli_args.env_file.is_none();

    no_config_flag || running_script_without_config || running_command_without_config
}
