use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct ExportTypeKeyword;

impl Command for ExportTypeKeyword {
    fn name(&self) -> &str {
        "export type"
    }

    fn description(&self) -> &str {
        "Give a name to a type annotation and export it from a module."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export type")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("name", SyntaxShape::String, "Name of the type alias.")
            .required(
                "type",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::String)),
                "Equals sign followed by a type, as written after `:` on a parameter.",
            )
            .category(Category::Core)
    }

    fn extra_description(&self) -> &str {
        "This command is a parser keyword. `use <module> <name>`, `use <module> [<names>]` and \
        `use <module> *` import a type alias. A bare `use <module>` doesn't, because a type alias \
        name can't take the module name as a prefix the way a command name does.

For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["type alias", "typedef", "typealias"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Export a type alias from a module and import it with a command.",
            example: "module geo {
        export type Point = record<x: int, y: int>
        export def origin []: nothing -> Point { {x: 0, y: 0} }
    }
    use geo [Point origin]
    let p: Point = origin
    $p.y",
            result: Some(Value::test_int(0)),
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(ExportTypeKeyword)
    }
}
