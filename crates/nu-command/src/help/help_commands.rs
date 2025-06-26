use crate::filters::find_internal;
use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct HelpCommands;

impl Command for HelpCommands {
    fn name(&self) -> &str {
        "help commands"
    }

    fn description(&self) -> &str {
        "Show help on nushell commands."
    }

    fn signature(&self) -> Signature {
        Signature::build("help commands")
            .category(Category::Core)
            .rest(
                "rest",
                SyntaxShape::String,
                "The name of command to get help on.",
            )
            .named(
                "find",
                SyntaxShape::String,
                "string to find in command names, descriptions, and search terms",
                Some('f'),
            )
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .allow_variants_without_examples(true)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        help_commands(engine_state, stack, call)
    }
}

pub fn help_commands(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let find: Option<Spanned<String>> = call.get_flag(engine_state, stack, "find")?;
    let rest: Vec<Spanned<String>> = call.rest(engine_state, stack, 0)?;

    if let Some(f) = find {
        let all_cmds_vec = build_help_commands(engine_state, head);
        return find_internal(
            all_cmds_vec,
            engine_state,
            stack,
            &f.item,
            &["name", "description", "search_terms"],
            true,
        );
    }

    if rest.is_empty() {
        Ok(build_help_commands(engine_state, head))
    } else {
        let mut name = String::new();

        for r in &rest {
            if !name.is_empty() {
                name.push(' ');
            }
            name.push_str(&r.item);
        }

        if let Some(decl) = engine_state.find_decl(name.as_bytes(), &[]) {
            let cmd = engine_state.get_decl(decl);
            let help_text = get_full_help(cmd, engine_state, stack);
            Ok(Value::string(help_text, call.head).into_pipeline_data())
        } else {
            Err(ShellError::CommandNotFound {
                span: Span::merge_many(rest.iter().map(|s| s.span)),
            })
        }
    }
}

fn build_help_commands(engine_state: &EngineState, span: Span) -> PipelineData {
    let commands = engine_state.get_decls_sorted(false);
    let mut found_cmds_vec = Vec::new();

    for (_, decl_id) in commands {
        let decl = engine_state.get_decl(decl_id);
        let sig = decl.signature().update_from_command(decl);

        let key = sig.name;
        let description = sig.description;
        let search_terms = sig.search_terms;

        let command_type = decl.command_type().to_string();

        // Build table of parameters
        let param_table = {
            let mut vals = vec![];

            for required_param in &sig.required_positional {
                vals.push(Value::record(
                    record! {
                        "name" => Value::string(&required_param.name, span),
                        "type" => Value::string(required_param.shape.to_string(), span),
                        "required" => Value::bool(true, span),
                        "description" => Value::string(&required_param.desc, span),
                    },
                    span,
                ));
            }

            for optional_param in &sig.optional_positional {
                vals.push(Value::record(
                    record! {
                        "name" => Value::string(&optional_param.name, span),
                        "type" => Value::string(optional_param.shape.to_string(), span),
                        "required" => Value::bool(false, span),
                        "description" => Value::string(&optional_param.desc, span),
                    },
                    span,
                ));
            }

            if let Some(rest_positional) = &sig.rest_positional {
                vals.push(Value::record(
                    record! {
                        "name" => Value::string(format!("...{}", rest_positional.name), span),
                        "type" => Value::string(rest_positional.shape.to_string(), span),
                        "required" => Value::bool(false, span),
                        "description" => Value::string(&rest_positional.desc, span),
                    },
                    span,
                ));
            }

            for named_param in &sig.named {
                let name = if let Some(short) = named_param.short {
                    if named_param.long.is_empty() {
                        format!("-{short}")
                    } else {
                        format!("--{}(-{})", named_param.long, short)
                    }
                } else {
                    format!("--{}", named_param.long)
                };

                let typ = if let Some(arg) = &named_param.arg {
                    arg.to_string()
                } else {
                    "switch".to_string()
                };

                vals.push(Value::record(
                    record! {
                        "name" => Value::string(name, span),
                        "type" => Value::string(typ, span),
                        "required" => Value::bool(named_param.required, span),
                        "description" => Value::string(&named_param.desc, span),
                    },
                    span,
                ));
            }

            Value::list(vals, span)
        };

        // Build the signature input/output table
        let input_output_table = {
            let mut vals = vec![];

            for (input_type, output_type) in sig.input_output_types {
                vals.push(Value::record(
                    record! {
                        "input" => Value::string(input_type.to_string(), span),
                        "output" => Value::string(output_type.to_string(), span),
                    },
                    span,
                ));
            }

            Value::list(vals, span)
        };

        let record = record! {
            "name" => Value::string(key, span),
            "category" => Value::string(sig.category.to_string(), span),
            "command_type" => Value::string(command_type, span),
            "description" => Value::string(description, span),
            "params" => param_table,
            "input_output" => input_output_table,
            "search_terms" => Value::string(search_terms.join(", "), span),
            "is_const" => Value::bool(decl.is_const(), span),
        };

        found_cmds_vec.push(Value::record(record, span));
    }

    Value::list(found_cmds_vec, span).into_pipeline_data()
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::HelpCommands;
        use crate::test_examples;
        test_examples(HelpCommands {})
    }
}
