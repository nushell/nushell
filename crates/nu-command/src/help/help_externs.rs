use crate::help::highlight_search_in_table;
use nu_color_config::StyleComputer;
use nu_engine::{command_prelude::*, get_full_help, scope::ScopeData};
use nu_protocol::span;

#[derive(Clone)]
pub struct HelpExterns;

impl Command for HelpExterns {
    fn name(&self) -> &str {
        "help externs"
    }

    fn usage(&self) -> &str {
        "Show help on nushell externs."
    }

    fn signature(&self) -> Signature {
        Signature::build("help externs")
            .category(Category::Core)
            .rest(
                "rest",
                SyntaxShape::String,
                "The name of extern to get help on.",
            )
            .named(
                "find",
                SyntaxShape::String,
                "string to find in extern names and usage",
                Some('f'),
            )
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .allow_variants_without_examples(true)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "show all externs",
                example: "help externs",
                result: None,
            },
            Example {
                description: "show help for single extern",
                example: "help externs smth",
                result: None,
            },
            Example {
                description: "search for string in extern names and usages",
                example: "help externs --find smth",
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
        help_externs(engine_state, stack, call)
    }
}

pub fn help_externs(
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
        let all_cmds_vec = build_help_externs(engine_state, stack, head);
        let found_cmds_vec = highlight_search_in_table(
            all_cmds_vec,
            &f.item,
            &["name", "usage"],
            &string_style,
            &highlight_style,
        )?;

        return Ok(found_cmds_vec
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone()));
    }

    if rest.is_empty() {
        let found_cmds_vec = build_help_externs(engine_state, stack, head);

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
            Ok(
                Value::string(output.join("======================\n\n"), call.head)
                    .into_pipeline_data(),
            )
        } else {
            Err(ShellError::CommandNotFound {
                span: span(&[rest[0].span, rest[rest.len() - 1].span]),
            })
        }
    }
}

fn build_help_externs(engine_state: &EngineState, stack: &Stack, span: Span) -> Vec<Value> {
    let mut scope = ScopeData::new(engine_state, stack);
    scope.populate_decls();
    scope.collect_externs(span)
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::HelpExterns;
        use crate::test_examples;
        test_examples(HelpExterns {})
    }
}
