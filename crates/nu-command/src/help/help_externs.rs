use crate::help::highlight_search_in_table;
use nu_color_config::StyleComputer;
use nu_engine::{command_prelude::*, get_full_help, scope::ScopeData};

#[derive(Clone)]
pub struct HelpExterns;

impl Command for HelpExterns {
    fn name(&self) -> &str {
        "help externs"
    }

    fn description(&self) -> &str {
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
                "string to find in extern names and descriptions",
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
                description: "search for string in extern names and descriptions",
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
            &["name", "description"],
            &string_style,
            &highlight_style,
        )?;

        return Ok(Value::list(found_cmds_vec, head).into_pipeline_data());
    }

    if rest.is_empty() {
        let found_cmds_vec = build_help_externs(engine_state, stack, head);
        Ok(Value::list(found_cmds_vec, head).into_pipeline_data())
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
