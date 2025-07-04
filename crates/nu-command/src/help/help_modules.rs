use crate::filters::find_internal;
use nu_engine::{command_prelude::*, scope::ScopeData};
use nu_protocol::DeclId;

#[derive(Clone)]
pub struct HelpModules;

impl Command for HelpModules {
    fn name(&self) -> &str {
        "help modules"
    }

    fn description(&self) -> &str {
        "Show help on nushell modules."
    }

    fn extra_description(&self) -> &str {
        r#"When requesting help for a single module, its commands and aliases will be highlighted if they
are also available in the current scope. Commands/aliases that were imported under a different name
(such as with a prefix after `use some-module`) will be highlighted in parentheses."#
    }

    fn signature(&self) -> Signature {
        Signature::build("help modules")
            .category(Category::Core)
            .rest(
                "rest",
                SyntaxShape::String,
                "The name of module to get help on.",
            )
            .named(
                "find",
                SyntaxShape::String,
                "string to find in module names and descriptions",
                Some('f'),
            )
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .allow_variants_without_examples(true)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "show all modules",
                example: "help modules",
                result: None,
            },
            Example {
                description: "show help for single module",
                example: "help modules my-module",
                result: None,
            },
            Example {
                description: "search for string in module names and descriptions",
                example: "help modules --find my-module",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        help_modules(engine_state, stack, call)
    }
}

pub fn help_modules(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let find: Option<Spanned<String>> = call.get_flag(engine_state, stack, "find")?;
    let rest: Vec<Spanned<String>> = call.rest(engine_state, stack, 0)?;

    if let Some(f) = find {
        let all_cmds_vec = build_help_modules(engine_state, stack, head);
        return find_internal(
            all_cmds_vec,
            engine_state,
            stack,
            &f.item,
            &["name", "description"],
            true,
        );
    }

    if rest.is_empty() {
        Ok(build_help_modules(engine_state, stack, head))
    } else {
        let mut name = String::new();

        for r in &rest {
            if !name.is_empty() {
                name.push(' ');
            }
            name.push_str(&r.item);
        }

        let Some(module_id) = engine_state.find_module(name.as_bytes(), &[]) else {
            return Err(ShellError::ModuleNotFoundAtRuntime {
                mod_name: name,
                span: Span::merge_many(rest.iter().map(|s| s.span)),
            });
        };

        let module = engine_state.get_module(module_id);

        let module_desc = engine_state.build_module_desc(module_id);

        // TODO: merge this into documentation.rs at some point
        const G: &str = "\x1b[32m"; // green
        const C: &str = "\x1b[36m"; // cyan
        const CB: &str = "\x1b[1;36m"; // cyan bold
        const RESET: &str = "\x1b[0m"; // reset

        let mut long_desc = String::new();

        if let Some((desc, extra_desc)) = module_desc {
            long_desc.push_str(&desc);
            long_desc.push_str("\n\n");

            if !extra_desc.is_empty() {
                long_desc.push_str(&extra_desc);
                long_desc.push_str("\n\n");
            }
        }

        long_desc.push_str(&format!("{G}Module{RESET}: {C}{name}{RESET}"));
        long_desc.push_str("\n\n");

        if !module.decls.is_empty() || module.main.is_some() {
            let commands: Vec<(Vec<u8>, DeclId)> = engine_state
                .get_decls_sorted(false)
                .into_iter()
                .filter(|(_, id)| !engine_state.get_decl(*id).is_alias())
                .collect();

            let mut module_commands: Vec<(Vec<u8>, DeclId)> = module
                .decls()
                .into_iter()
                .filter(|(_, id)| !engine_state.get_decl(*id).is_alias())
                .collect();
            module_commands.sort_by(|a, b| a.0.cmp(&b.0));

            let commands_str = module_commands
                .iter()
                .map(|(name_bytes, id)| {
                    let name = String::from_utf8_lossy(name_bytes);
                    if let Some((used_name_bytes, _)) =
                        commands.iter().find(|(_, decl_id)| id == decl_id)
                    {
                        if engine_state.find_decl(name.as_bytes(), &[]).is_some() {
                            format!("{CB}{name}{RESET}")
                        } else {
                            let command_name = String::from_utf8_lossy(used_name_bytes);
                            format!("{name} ({CB}{command_name}{RESET})")
                        }
                    } else {
                        format!("{name}")
                    }
                })
                .collect::<Vec<String>>()
                .join(", ");

            long_desc.push_str(&format!("{G}Exported commands{RESET}:\n  {commands_str}"));
            long_desc.push_str("\n\n");
        }

        if !module.decls.is_empty() {
            let aliases: Vec<(Vec<u8>, DeclId)> = engine_state
                .get_decls_sorted(false)
                .into_iter()
                .filter(|(_, id)| engine_state.get_decl(*id).is_alias())
                .collect();

            let mut module_aliases: Vec<(Vec<u8>, DeclId)> = module
                .decls()
                .into_iter()
                .filter(|(_, id)| engine_state.get_decl(*id).is_alias())
                .collect();
            module_aliases.sort_by(|a, b| a.0.cmp(&b.0));

            let aliases_str = module_aliases
                .iter()
                .map(|(name_bytes, id)| {
                    let name = String::from_utf8_lossy(name_bytes);
                    if let Some((used_name_bytes, _)) =
                        aliases.iter().find(|(_, alias_id)| id == alias_id)
                    {
                        if engine_state.find_decl(name.as_bytes(), &[]).is_some() {
                            format!("{CB}{name}{RESET}")
                        } else {
                            let alias_name = String::from_utf8_lossy(used_name_bytes);
                            format!("{name} ({CB}{alias_name}{RESET})")
                        }
                    } else {
                        format!("{name}")
                    }
                })
                .collect::<Vec<String>>()
                .join(", ");

            long_desc.push_str(&format!("{G}Exported aliases{RESET}:\n  {aliases_str}"));
            long_desc.push_str("\n\n");
        }

        if module.env_block.is_some() {
            long_desc.push_str(&format!("This module {C}exports{RESET} environment."));
        } else {
            long_desc.push_str(&format!(
                "This module {C}does not export{RESET} environment."
            ));
        }

        let config = stack.get_config(engine_state);
        if !config.use_ansi_coloring.get(engine_state) {
            long_desc = nu_utils::strip_ansi_string_likely(long_desc);
        }

        Ok(Value::string(long_desc, call.head).into_pipeline_data())
    }
}

fn build_help_modules(engine_state: &EngineState, stack: &Stack, span: Span) -> PipelineData {
    let mut scope_data = ScopeData::new(engine_state, stack);
    scope_data.populate_modules();

    Value::list(scope_data.collect_modules(span), span).into_pipeline_data()
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::HelpModules;
        use crate::test_examples;
        test_examples(HelpModules {})
    }
}
