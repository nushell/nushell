use nu_parser::{parse, parse_module_block};
use nu_protocol::report_error;
use nu_protocol::{engine::StateWorkingSet, Module, ShellError, Span};

fn add_file(
    working_set: &mut StateWorkingSet,
    name: &String,
    content: &[u8],
) -> (Module, Vec<Span>) {
    let file_id = working_set.add_file(name.clone(), content);
    let new_span = working_set.get_span_for_file(file_id);

    let (_, module, comments) = parse_module_block(working_set, new_span, name.as_bytes());

    if let Some(err) = working_set.parse_errors.first() {
        report_error(working_set, err);
    }

    parse(working_set, Some(name), content, true);

    if let Some(err) = working_set.parse_errors.first() {
        report_error(working_set, err);
    }

    (module, comments)
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
        let content = include_str!("../lib/mod.nu");

        // these modules are loaded in the order they appear in this list
        #[rustfmt::skip]
        let submodules = vec![
            // helper modules that could be used in other parts of the library
            ("log", include_str!("../lib/log.nu")),

            // the rest of the library
            ("dirs", include_str!("../lib/dirs.nu")),
            ("iter", include_str!("../lib/iter.nu")),
            ("help", include_str!("../lib/help.nu")),
            ("testing", include_str!("../lib/testing.nu")),
            ("xml", include_str!("../lib/xml.nu")),
            ("dt", include_str!("../lib/dt.nu")),
        ];

        // Define commands to be preloaded into the default (top level, unprefixed) namespace.
        // User can invoke these without having to `use std` beforehand.
        // Entries are: (name to add to default namespace, path under std to find implementation)
        //
        // Conventionally, for a command implemented as `std foo`, the name added
        // is either `std foo` or bare `foo`, not some arbitrary rename.

        #[rustfmt::skip]
        let prelude = vec![
            ("std help", "help"),
            ("std help commands", "help commands"),
            ("std help aliases", "help aliases"),
            ("std help modules", "help modules"),
            ("std help externs", "help externs"),
            ("std help operators", "help operators"),

            ("enter", "enter"),
            ("shells", "shells"),
            ("g", "g"),
            ("n", "n"),
            ("p", "p"),
            ("dexit", "dexit"),
        ];

        let mut working_set = StateWorkingSet::new(engine_state);

        for (name, content) in submodules {
            let (module, comments) =
                add_file(&mut working_set, &name.to_string(), content.as_bytes());
            working_set.add_module(name, module, comments);
        }

        let (module, comments) = add_file(&mut working_set, &name, content.as_bytes());
        load_prelude(&mut working_set, prelude, &module);
        working_set.add_module(&name, module, comments);

        working_set.render()
    };

    engine_state.merge_delta(delta)?;

    Ok(())
}
