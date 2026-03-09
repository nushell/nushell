use nu_protocol::engine::EngineState;

pub(crate) fn add_command_context(engine_state: EngineState) -> EngineState {
    let engine_state = nu_cmd_lang::add_default_context(engine_state);
    #[cfg(feature = "plugin")]
    let engine_state = nu_cmd_plugin::add_plugin_command_context(engine_state);
    let engine_state = nu_command::add_shell_command_context(engine_state);
    let engine_state = nu_cmd_extra::add_extra_command_context(engine_state);
    let engine_state = nu_cli::add_cli_context(engine_state);
    nu_explore::add_explore_context(engine_state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::{Category, PositionalArg};

    #[test]
    fn arguments_end_period() {
        fn ends_period(cmd_name: &str, ty: &str, arg: PositionalArg, failures: &mut Vec<String>) {
            let arg_name = arg.name;
            let desc = arg.desc;
            if !desc.ends_with('.') {
                failures.push(format!(
                    "{cmd_name} {ty} argument \"{arg_name}\": \"{desc}\""
                ));
            }
        }

        let ctx = add_command_context(EngineState::new());
        let decls = ctx.get_decls_sorted(true);
        let mut failures = Vec::new();

        for (name_bytes, decl_id) in decls {
            let cmd = ctx.get_decl(decl_id);
            let cmd_name = String::from_utf8_lossy(&name_bytes);
            let signature = cmd.signature();

            for arg in signature.required_positional {
                ends_period(&cmd_name, "required", arg, &mut failures);
            }

            for arg in signature.optional_positional {
                ends_period(&cmd_name, "optional", arg, &mut failures);
            }

            if let Some(arg) = signature.rest_positional {
                ends_period(&cmd_name, "rest", arg, &mut failures);
            }
        }

        assert!(
            failures.is_empty(),
            "Command argument description does not end with a period:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn arguments_start_uppercase() {
        fn starts_uppercase(
            cmd_name: &str,
            ty: &str,
            arg: PositionalArg,
            failures: &mut Vec<String>,
        ) {
            let arg_name = arg.name;
            let desc = arg.desc;

            // Check lowercase to allow usage to contain syntax like:
            //
            // "`as` keyword …"
            if desc.starts_with(|u: char| u.is_lowercase()) {
                failures.push(format!(
                    "{cmd_name} {ty} argument \"{arg_name}\": \"{desc}\""
                ));
            }
        }

        let ctx = add_command_context(EngineState::new());
        let decls = ctx.get_decls_sorted(true);
        let mut failures = Vec::new();

        for (name_bytes, decl_id) in decls {
            let cmd = ctx.get_decl(decl_id);
            let cmd_name = String::from_utf8_lossy(&name_bytes);
            let signature = cmd.signature();

            for arg in signature.required_positional {
                starts_uppercase(&cmd_name, "required", arg, &mut failures);
            }

            for arg in signature.optional_positional {
                starts_uppercase(&cmd_name, "optional", arg, &mut failures);
            }

            if let Some(arg) = signature.rest_positional {
                starts_uppercase(&cmd_name, "rest", arg, &mut failures);
            }
        }

        assert!(
            failures.is_empty(),
            "Command argument description does not end with a period:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn signature_name_matches_command_name() {
        let ctx = add_command_context(EngineState::new());
        let decls = ctx.get_decls_sorted(true);
        let mut failures = Vec::new();

        for (name_bytes, decl_id) in decls {
            let cmd = ctx.get_decl(decl_id);
            let cmd_name = String::from_utf8_lossy(&name_bytes);
            let sig_name = cmd.signature().name;
            let category = cmd.signature().category;

            if cmd_name != sig_name {
                failures.push(format!(
                "{cmd_name} ({category:?}): Signature name \"{sig_name}\" is not equal to the command name \"{cmd_name}\""
            ));
            }
        }

        assert!(
            failures.is_empty(),
            "Name mismatch:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn commands_declare_input_output_types() {
        let ctx = add_command_context(EngineState::new());
        let decls = ctx.get_decls_sorted(true);
        let mut failures = Vec::new();

        for (_, decl_id) in decls {
            let cmd = ctx.get_decl(decl_id);
            let sig_name = cmd.signature().name;
            let category = cmd.signature().category;

            if let Category::Removed = category {
                // Deprecated/Removed commands don't have to conform
                continue;
            }

            if cmd.signature().input_output_types.is_empty() {
                failures.push(format!(
                    "{sig_name} ({category:?}): No pipeline input/output type signatures found"
                ));
            }
        }

        assert!(
            failures.is_empty(),
            "Command missing type annotations:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn no_search_term_duplicates() {
        let ctx = add_command_context(EngineState::new());
        let decls = ctx.get_decls_sorted(true);
        let mut failures = Vec::new();

        for (name_bytes, decl_id) in decls {
            let cmd = ctx.get_decl(decl_id);
            let cmd_name = String::from_utf8_lossy(&name_bytes);
            let search_terms = cmd.search_terms();
            let category = cmd.signature().category;

            for search_term in search_terms {
                if cmd_name.contains(search_term) {
                    failures.push(format!("{cmd_name} ({category:?}): Search term \"{search_term}\" is substring of command name \"{cmd_name}\""));
                }
            }
        }

        assert!(
            failures.is_empty(),
            "Duplication in search terms:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn description_end_period() {
        let ctx = add_command_context(EngineState::new());
        let decls = ctx.get_decls_sorted(true);
        let mut failures = Vec::new();

        for (name_bytes, decl_id) in decls {
            let cmd = ctx.get_decl(decl_id);
            let cmd_name = String::from_utf8_lossy(&name_bytes);
            let description = cmd.description();

            if !description.ends_with('.') {
                failures.push(format!("{cmd_name}: \"{description}\""));
            }
        }

        assert!(
            failures.is_empty(),
            "Command description does not end with a period:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn description_start_uppercase() {
        let ctx = add_command_context(EngineState::new());
        let decls = ctx.get_decls_sorted(true);
        let mut failures = Vec::new();

        for (name_bytes, decl_id) in decls {
            let cmd = ctx.get_decl(decl_id);
            let cmd_name = String::from_utf8_lossy(&name_bytes);
            let description = cmd.description();

            // Check lowercase to allow description to contain syntax like:
            //
            // "`$env.FOO = ...`"
            if description.starts_with(|u: char| u.is_lowercase()) {
                failures.push(format!("{cmd_name}: \"{description}\""));
            }
        }

        assert!(
            failures.is_empty(),
            "Command description does not start with an uppercase letter:\n{}",
            failures.join("\n")
        );
    }
}
