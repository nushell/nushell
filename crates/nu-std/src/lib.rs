use nu_cli::report_error;
use nu_parser::{parse, parse_module_block};
use nu_protocol::{engine::StateWorkingSet, Module, ShellError, Span};

fn get_standard_library() -> &'static str {
    include_str!("../std.nu")
}

fn load_prelude(working_set: &mut StateWorkingSet, prelude: Vec<(&str, &str)>, module: &Module) {
    let mut decls = Vec::new();
    let mut errs = Vec::new();
    for (name, search_name) in prelude {
        if let Some(id) = module.decls.get(&search_name.as_bytes().to_vec()) {
            let decl = (name.as_bytes().to_vec(), id.to_owned());
            decls.push(decl);
        } else {
            errs.push(ShellError::GenericError(
                format!("could not load `{}` from `std`.", search_name),
                String::new(),
                None,
                None,
                Vec::new(),
            ));
        }
    }

    if !errs.is_empty() {
        report_error(
            working_set,
            &ShellError::GenericError(
                "Unable to load the prelude of the standard library.".into(),
                String::new(),
                None,
                Some("this is a bug: please file an issue in the [issue tracker](https://github.com/nushell/nushell/issues/new/choose)".to_string()),
                errs,
            ),
        );
    }

    working_set.use_decls(decls);
}

pub fn load_standard_library(
    engine_state: &mut nu_protocol::engine::EngineState,
) -> Result<(), miette::ErrReport> {
    let delta = {
        let name = "std".to_string();
        let content = get_standard_library().as_bytes();

        let mut working_set = StateWorkingSet::new(engine_state);

        let start = working_set.next_span_start();
        working_set.add_file(name.clone(), content);
        let end = working_set.next_span_start();

        let (_, module, comments, parse_error) = parse_module_block(
            &mut working_set,
            Span::new(start, end),
            name.as_bytes(),
            &[],
        );

        if let Some(err) = parse_error {
            report_error(&working_set, &err);
        }

        let (_, parse_error) = parse(&mut working_set, Some(&name), content, true, &[]);

        if let Some(err) = parse_error {
            report_error(&working_set, &err);
        }

        // TODO: change this when #8505 is merged
        // NOTE: remove the assert and uncomment the `help`s
        let prelude = vec![
            ("assert", "assert"),
            // ("help", "help"),
            // ("help commands", "help commands"),
            // ("help aliases", "help aliases"),
            // ("help modules", "help modules"),
            // ("help externs", "help externs"),
            // ("help operators", "help operators"),
        ];

        load_prelude(&mut working_set, prelude, &module);

        working_set.add_module(&name, module, comments);

        working_set.render()
    };

    engine_state.merge_delta(delta)?;

    Ok(())
}
