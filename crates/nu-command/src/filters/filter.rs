use nu_engine::command_prelude::*;
use nu_protocol::{DeprecationEntry, DeprecationType, ReportMode};

#[derive(Clone)]
pub struct Filter;

impl Command for Filter {
    fn name(&self) -> &str {
        "filter"
    }

    fn description(&self) -> &str {
        "Filter values based on a predicate closure."
    }

    fn extra_description(&self) -> &str {
        r#"This command works similar to 'where' but can only use a closure as a predicate.
The "row condition" syntax is not supported."#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("filter")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::Range, Type::List(Box::new(Type::Any))),
            ])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "Predicate closure.",
            )
            .category(Category::Filters)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["where", "find", "search", "condition"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        use super::where_::Where;
        <Where as Command>::run(&Where, engine_state, stack, call, input)
    }

    fn deprecation_info(&self) -> Vec<nu_protocol::DeprecationEntry> {
        vec![
            DeprecationEntry {
                ty: DeprecationType::Command,
                report_mode: ReportMode::FirstUse,
                since: Some("0.105.0".into()),
                expected_removal: None,
                help: Some("`where` command can be used instead, as it can now read the predicate closure from a variable".into()),
            }
        ]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Filter items of a list according to a condition",
                example: "[1 2] | filter {|x| $x > 1}",
                result: Some(Value::test_list(vec![Value::test_int(2)])),
            },
            Example {
                description: "Filter rows of a table according to a condition",
                example: "[{a: 1} {a: 2}] | filter {|x| $x.a > 1}",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "a" => Value::test_int(2),
                })])),
            },
            Example {
                description: "Filter rows of a table according to a stored condition",
                example: "let cond = {|x| $x.a > 1}; [{a: 1} {a: 2}] | filter $cond",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "a" => Value::test_int(2),
                })])),
            },
            Example {
                description: "Filter items of a range according to a condition",
                example: "9..13 | filter {|el| $el mod 2 != 0}",
                result: Some(Value::test_list(vec![
                    Value::test_int(9),
                    Value::test_int(11),
                    Value::test_int(13),
                ])),
            },
            Example {
                description: "List all numbers above 3, using an existing closure condition",
                example: "let a = {$in > 3}; [1, 2, 5, 6] | filter $a",
                result: None, // TODO: This should work
                              // result: Some(Value::test_list(
                              //     vec![
                              //         Value::Int {
                              //             val: 5,
                              //             Span::test_data(),
                              //         },
                              //         Value::Int {
                              //             val: 6,
                              //             span: Span::test_data(),
                              //         },
                              //     ],
                              // }),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Filter {})
    }
}
