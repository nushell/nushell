use crate::help::highlight_search_in_table;
use nu_color_config::StyleComputer;
use nu_engine::{get_full_help, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    span, Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct HelpExternals;

impl Command for HelpExternals {
    fn name(&self) -> &str {
        "help externals"
    }

    fn usage(&self) -> &str {
        "Show help on nushell externals."
    }

    fn signature(&self) -> Signature {
        Signature::build("help externals")
            .category(Category::Core)
            .rest(
                "rest",
                SyntaxShape::String,
                "the name of external to get help on",
            )
            .named(
                "find",
                SyntaxShape::String,
                "string to find in external names and usage",
                Some('f'),
            )
            .input_output_types(vec![(Type::Nothing, Type::Table(vec![]))])
            .allow_variants_without_examples(true)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "show all externals",
                example: "help aliases",
                result: None,
            },
            Example {
                description: "show help for single external",
                example: "help aliases my-alias",
                result: None,
            },
            Example {
                description: "search for string in external names and usages",
                example: "help aliases --find my-alias",
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
        help_externals(engine_state, stack, call)
    }
}

pub fn help_externals(
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

    if let Some(f) = find {
        let all_cmds_vec = build_help_externals(engine_state, head);
        let found_cmds_vec =
            highlight_search_in_table(all_cmds_vec, &f.item, &["name", "usage"], &string_style)?;

        return Ok(found_cmds_vec
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone()));
    }

    if rest.is_empty() {
        let found_cmds_vec = build_help_externals(engine_state, head);

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

fn build_help_externals(engine_state: &EngineState, span: Span) -> Vec<Value> {
    let mut externals = vec![];
    for (name, decl_id) in engine_state.get_decls_sorted(false) {
        let decl = engine_state.get_decl(decl_id);

        if decl.is_known_external() {
            let mut cols = vec![];
            let mut vals = vec![];

            cols.push("name".into());
            vals.push(Value::String {
                val: String::from_utf8_lossy(&name).to_string(),
                span,
            });

            let sig = decl.signature();
            let signatures = sig.to_string().trim_start().replace("\n  ", "\n");

            cols.push("category".to_string());
            vals.push(Value::String {
                val: sig.category.to_string(),
                span,
            });

            cols.push("usage".to_string());
            vals.push(Value::String {
                val: decl.usage().into(),
                span,
            });

            cols.push("signatures".into());
            vals.push(Value::String {
                val: if decl.is_parser_keyword() {
                    "".to_string()
                } else {
                    signatures
                },
                span,
            });

            let search_terms = decl.search_terms();
            cols.push("search_terms".to_string());
            vals.push(Value::String {
                val: search_terms.join(", "),
                span,
            });

            externals.push(Value::Record { cols, vals, span })
        }
    }

    externals
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::HelpExternals;
        use crate::test_examples;
        test_examples(HelpExternals {})
    }
}
