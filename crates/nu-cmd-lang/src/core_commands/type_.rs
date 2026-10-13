use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct TypeKeyword;

impl Command for TypeKeyword {
    fn name(&self) -> &str {
        "type"
    }

    fn description(&self) -> &str {
        "Give a name to a type annotation."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("type")
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
        "This command is a parser keyword. The parser replaces the name with its type wherever \
        the name is used as a type: in parameters, input/output types, variable annotations and \
        other types. The alias is visible from its definition to the end of the block.

A completer belongs to a parameter, not to its type, so `type` takes no `@completer`. Write \
        it on the parameter instead, as in `color: Color@[red green blue]`.

`type` is reserved, so a custom command or alias can't be named `type`. To run an external \
        `type` command, use `^type`.

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
        vec![
            Example {
                description: "Name a record type and use it for a parameter and an output type.",
                example: "type Point = record<x: int, y: int>
    def shift [p: Point]: nothing -> Point { {x: ($p.x + 1), y: $p.y} }
    (shift {x: 1, y: 2}).x",
                result: Some(Value::test_int(2)),
            },
            Example {
                description: "Build a type from another type alias.",
                example: "type Point = record<x: int, y: int>
    type Segment = record<from: Point, to: Point>
    def width [s: Segment]: nothing -> int { $s.to.x - $s.from.x }
    width {from: {x: 1, y: 0}, to: {x: 4, y: 2}}",
                result: Some(Value::test_int(3)),
            },
            Example {
                description: "Give a parameter of an aliased type a completer.",
                example: "type Color = string
    def paint [color: Color@[red green blue]] { $\"painting it ($color)\" }
    paint red",
                result: Some(Value::test_string("painting it red")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(TypeKeyword)
    }
}
