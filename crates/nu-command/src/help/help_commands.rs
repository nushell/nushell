use crate::help::highlight_search_in_table;
use nu_color_config::StyleComputer;
use nu_engine::{get_full_help, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    span, Category, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, Spanned, SyntaxShape, Type, Value,
};
use std::borrow::Borrow;

#[derive(Clone)]
pub struct HelpCommands;

impl Command for HelpCommands {
    fn name(&self) -> &str {
        "help commands"
    }

    fn usage(&self) -> &str {
        "Show help on nushell commands."
    }

    fn signature(&self) -> Signature {
        Signature::build("help commands")
            .category(Category::Core)
            .rest(
                "rest",
                SyntaxShape::String,
                "the name of command to get help on",
            )
            .named(
                "find",
                SyntaxShape::String,
                "string to find in command names, usage, and search terms",
                Some('f'),
            )
            .input_output_types(vec![(Type::Nothing, Type::Table(vec![]))])
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

    // 🚩The following two-lines are copied from filters/find.rs:
    let style_computer = StyleComputer::from_config(engine_state, stack);
    // Currently, search results all use the same style.
    // Also note that this sample string is passed into user-written code (the closure that may or may not be
    // defined for "string").
    let string_style = style_computer.compute("string", &Value::string("search result", head));
    let highlight_style =
        style_computer.compute("search_result", &Value::string("search result", head));

    if let Some(f) = find {
        let all_cmds_vec = build_help_commands(engine_state, head);
        let found_cmds_vec = highlight_search_in_table(
            all_cmds_vec,
            &f.item,
            &["name", "usage", "search_terms"],
            &string_style,
            &highlight_style,
        )?;

        return Ok(found_cmds_vec
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone()));
    }

    if rest.is_empty() {
        let found_cmds_vec = build_help_commands(engine_state, head);

        Ok(found_cmds_vec
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone()))
    } else {
        let mut name = String::new();

        for r in &rest {
            if !name.is_empty() {
                name.push(' ');
            }
            name.push_str(&r.item);
        }

        let output = engine_state
            .get_signatures_with_examples(false)
            .iter()
            .filter(|(signature, _, _, _, _)| signature.name == name)
            .map(|(signature, examples, _, _, is_parser_keyword)| {
                get_full_help(signature, examples, engine_state, stack, *is_parser_keyword)
            })
            .collect::<Vec<String>>();

        if !output.is_empty() {
            Ok(Value::String {
                val: output.join("======================\n\n"),
                span: call.head,
            }
            .into_pipeline_data())
        } else {
            Err(ShellError::CommandNotFound(span(&[
                rest[0].span,
                rest[rest.len() - 1].span,
            ])))
        }
    }
}

fn build_help_commands(engine_state: &EngineState, span: Span) -> Vec<Value> {
    let commands = engine_state.get_decls_sorted(false);
    let mut found_cmds_vec = Vec::new();

    for (name_bytes, decl_id) in commands {
        let mut cols = vec![];
        let mut vals = vec![];

        let name = String::from_utf8_lossy(&name_bytes).to_string();
        let decl = engine_state.get_decl(decl_id);
        let sig = decl.signature().update_from_command(name, decl.borrow());

        let key = sig.name;
        let usage = sig.usage;
        let search_terms = sig.search_terms;

        cols.push("name".into());
        vals.push(Value::String { val: key, span });

        cols.push("category".into());
        vals.push(Value::string(sig.category.to_string(), span));

        cols.push("command_type".into());
        vals.push(Value::String {
            val: format!("{:?}", decl.command_type()).to_lowercase(),
            span,
        });

        cols.push("usage".into());
        vals.push(Value::String { val: usage, span });

        cols.push("params".into());
        // Build table of parameters
        let param_table = {
            let mut vals = vec![];
        };

        cols.push("input_output".into());

        // Build the signature input/output table
        let input_output_table = {
            let mut vals = vec![];

            for (input_type, output_type) in sig.input_output_types {
                vals.push(Value::Record {
                    cols: vec!["input".to_string(), "output".to_string()],
                    vals: vec![
                        Value::String {
                            val: input_type.to_string(),
                            span,
                        },
                        Value::String {
                            val: output_type.to_string(),
                            span,
                        },
                    ],
                    span,
                });
            }

            Value::List { vals, span }
        };
        vals.push(input_output_table);

        cols.push("search_terms".into());
        vals.push(Value::String {
            val: search_terms.join(", "),
            span,
        });

        found_cmds_vec.push(Value::Record { cols, vals, span });
    }

    found_cmds_vec
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
