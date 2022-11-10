#[cfg(test)]
use nu_protocol::engine::Command;

#[cfg(test)]
pub fn test_examples(cmd: impl Command + 'static) {
    test_examples::test_examples(cmd);
}

#[cfg(test)]
mod test_examples {
    use super::super::{
        Ansi, BuildString, Date, Echo, From, If, Into, LetEnv, Math, Path, Random, Split,
        SplitColumn, SplitRow, Str, StrJoin, StrLength, StrReplace, Url, Wrap,
    };
    use crate::To;
    use itertools::Itertools;
    use nu_protocol::{
        ast::Block,
        engine::{Command, EngineState, Stack, StateDelta, StateWorkingSet},
        Example, PipelineData, Signature, Span, Type, Value,
    };
    use std::{collections::HashSet, path::PathBuf};

    pub fn test_examples(cmd: impl Command + 'static) {
        let examples = cmd.examples();
        let signature = cmd.signature();
        let mut engine_state = make_engine_state(cmd.clone_box());

        let cwd = std::env::current_dir().expect("Could not get current working directory.");

        let mut witnessed_type_transformations = HashSet::<(Type, Type)>::new();

        for example in examples {
            if example.result.is_none() {
                continue;
            }
            witnessed_type_transformations.extend(
                check_example_input_and_output_types_match_command_signature(
                    &example,
                    &cwd,
                    &mut make_engine_state(cmd.clone_box()),
                    &signature.input_output_types,
                    signature.operates_on_cell_paths(),
                    signature.vectorizes_over_list,
                ),
            );
            check_example_evaluates_to_expected_output(&example, &cwd, &mut engine_state);
        }

        check_all_signature_input_output_types_entries_have_examples(
            signature,
            witnessed_type_transformations,
        );
    }

    fn make_engine_state(cmd: Box<dyn Command>) -> Box<EngineState> {
        let mut engine_state = Box::new(EngineState::new());

        let delta = {
            // Base functions that are needed for testing
            // Try to keep this working set small to keep tests running as fast as possible
            let mut working_set = StateWorkingSet::new(&*engine_state);
            working_set.add_decl(Box::new(Str));
            working_set.add_decl(Box::new(StrJoin));
            working_set.add_decl(Box::new(StrLength));
            working_set.add_decl(Box::new(StrReplace));
            working_set.add_decl(Box::new(BuildString));
            working_set.add_decl(Box::new(From));
            working_set.add_decl(Box::new(If));
            working_set.add_decl(Box::new(To));
            working_set.add_decl(Box::new(Into));
            working_set.add_decl(Box::new(Random));
            working_set.add_decl(Box::new(Split));
            working_set.add_decl(Box::new(SplitColumn));
            working_set.add_decl(Box::new(SplitRow));
            working_set.add_decl(Box::new(Math));
            working_set.add_decl(Box::new(Path));
            working_set.add_decl(Box::new(Date));
            working_set.add_decl(Box::new(Url));
            working_set.add_decl(Box::new(Ansi));
            working_set.add_decl(Box::new(Wrap));
            working_set.add_decl(Box::new(LetEnv));
            working_set.add_decl(Box::new(Echo));
            // Adding the command that is being tested to the working set
            working_set.add_decl(cmd);

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");
        engine_state
    }

    fn check_example_input_and_output_types_match_command_signature(
        example: &Example,
        cwd: &PathBuf,
        engine_state: &mut Box<EngineState>,
        signature_input_output_types: &Vec<(Type, Type)>,
        signature_operates_on_cell_paths: bool,
        signature_vectorizes_over_list: bool,
    ) -> HashSet<(Type, Type)> {
        let mut witnessed_type_transformations = HashSet::<(Type, Type)>::new();

        // Skip tests that don't have results to compare to
        if let Some(example_output) = example.result.as_ref() {
            if let Some(example_input_type) =
                eval_pipeline_without_terminal_expression(example.example, cwd, engine_state)
            {
                let example_input_type = example_input_type.get_type();
                let example_output_type = example_output.get_type();

                let example_matches_signature =
                    signature_input_output_types
                        .iter()
                        .any(|(sig_in_type, sig_out_type)| {
                            example_input_type.is_subtype(sig_in_type)
                                && example_output_type.is_subtype(sig_out_type)
                                && {
                                    witnessed_type_transformations
                                        .insert((sig_in_type.clone(), sig_out_type.clone()));
                                    true
                                }
                        });

                // The example type checks as vectorization over an input list if both:
                // 1. The command is declared to vectorize over list input.
                // 2. There exists an entry t -> u in the type map such that the
                //    example_input_type is a subtype of list<t> and the
                //    example_output_type is a subtype of list<u>.
                let example_matches_signature_via_vectorization_over_list =
                    signature_vectorizes_over_list
                        && match &example_input_type {
                            Type::List(ex_in_type) => {
                                match signature_input_output_types.iter().find_map(
                                    |(sig_in_type, sig_out_type)| {
                                        if ex_in_type.is_subtype(sig_in_type) {
                                            Some((sig_in_type, sig_out_type))
                                        } else {
                                            None
                                        }
                                    },
                                ) {
                                    Some((sig_in_type, sig_out_type)) => match &example_output_type
                                    {
                                        Type::List(ex_out_type)
                                            if ex_out_type.is_subtype(sig_out_type) =>
                                        {
                                            witnessed_type_transformations.insert((
                                                sig_in_type.clone(),
                                                sig_out_type.clone(),
                                            ));
                                            true
                                        }
                                        _ => false,
                                    },
                                    None => false,
                                }
                            }
                            _ => false,
                        };

                // The example type checks as a cell path operation if both:
                // 1. The command is declared to operate on cell paths.
                // 2. The example_input_type is list or record or table, and the example
                //    output shape is the same as the input shape.
                let example_matches_signature_via_cell_path_operation =
                    signature_operates_on_cell_paths
                        && example_input_type.accepts_cell_paths()
                        // TODO: This is too permissive; it should make use of the signature.input_output_types at least.
                        && example_output_type.to_shape() == example_input_type.to_shape();

                if !(example_matches_signature
                    || example_matches_signature_via_vectorization_over_list
                    || example_matches_signature_via_cell_path_operation)
                {
                    panic!(
                        "The example `{}` demonstrates a transformation of type {:?} -> {:?}. \
                        However, this does not match the declared signature: {:?}.{} \
                        For this command, `vectorizes_over_list` is {} and `operates_on_cell_paths()` is {}.",
                        example.example,
                        example_input_type,
                        example_output_type,
                        signature_input_output_types,
                        if signature_input_output_types.is_empty() { " (Did you forget to declare the input and output types for the command?)" } else { "" },
                        signature_vectorizes_over_list,
                        signature_operates_on_cell_paths
                    );
                };
            };
        }
        witnessed_type_transformations
    }

    fn check_example_evaluates_to_expected_output(
        example: &Example,
        cwd: &PathBuf,
        engine_state: &mut Box<EngineState>,
    ) {
        let mut stack = Stack::new();

        // Set up PWD
        stack.add_env_var(
            "PWD".to_string(),
            Value::String {
                val: cwd.to_string_lossy().to_string(),
                span: Span::test_data(),
            },
        );

        engine_state
            .merge_env(&mut stack, &cwd)
            .expect("Error merging environment");

        let empty_input = PipelineData::new(Span::test_data());
        let result = eval(example.example, empty_input, cwd, engine_state);

        // Note. Value implements PartialEq for Bool, Int, Float, String and Block
        // If the command you are testing requires to compare another case, then
        // you need to define its equality in the Value struct
        if let Some(expected) = example.result.as_ref() {
            assert_eq!(
                &result, expected,
                "The example result differs from the expected value",
            )
        }
    }

    fn check_all_signature_input_output_types_entries_have_examples(
        signature: Signature,
        witnessed_type_transformations: HashSet<(Type, Type)>,
    ) {
        let declared_type_transformations =
            HashSet::from_iter(signature.input_output_types.into_iter());
        assert!(
            witnessed_type_transformations.is_subset(&declared_type_transformations),
            "This should not be possible (bug in test): the type transformations \
            collected in the course of matching examples to the signature type map \
            contain type transformations not present in the signature type map."
        );

        if !signature.allow_variants_without_examples {
            assert_eq!(
                witnessed_type_transformations,
                declared_type_transformations,
                "There are entries in the signature type map which do not correspond to any example: \
                {:?}",
                declared_type_transformations
                    .difference(&witnessed_type_transformations)
                    .map(|(s1, s2)| format!("{} -> {}", s1, s2))
                    .join(", ")
            );
        }
    }

    fn eval(
        contents: &str,
        input: PipelineData,
        cwd: &PathBuf,
        engine_state: &mut Box<EngineState>,
    ) -> Value {
        let (block, delta) = parse(contents, engine_state);
        eval_block(block, input, cwd, engine_state, delta)
    }

    fn parse(contents: &str, engine_state: &EngineState) -> (Block, StateDelta) {
        let mut working_set = StateWorkingSet::new(engine_state);
        let (output, err) =
            nu_parser::parse(&mut working_set, None, contents.as_bytes(), false, &[]);

        if let Some(err) = err {
            panic!("test parse error in `{}`: {:?}", contents, err)
        }

        (output, working_set.render())
    }

    fn eval_block(
        block: Block,
        input: PipelineData,
        cwd: &PathBuf,
        engine_state: &mut Box<EngineState>,
        delta: StateDelta,
    ) -> Value {
        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let mut stack = Stack::new();

        stack.add_env_var(
            "PWD".to_string(),
            Value::String {
                val: cwd.to_string_lossy().to_string(),
                span: Span::test_data(),
            },
        );

        match nu_engine::eval_block(engine_state, &mut stack, &block, input, true, true) {
            Err(err) => panic!("test eval error in `{}`: {:?}", "TODO", err),
            Ok(result) => result.into_value(Span::test_data()),
        }
    }

    fn eval_pipeline_without_terminal_expression(
        src: &str,
        cwd: &PathBuf,
        engine_state: &mut Box<EngineState>,
    ) -> Option<Value> {
        let (mut block, delta) = parse(src, engine_state);
        match block.pipelines.len() {
            1 => {
                let n_expressions = block.pipelines[0].expressions.len();
                block.pipelines[0].expressions.truncate(&n_expressions - 1);

                if !block.pipelines[0].expressions.is_empty() {
                    let empty_input = PipelineData::new(Span::test_data());
                    Some(eval_block(block, empty_input, cwd, engine_state, delta))
                } else {
                    Some(Value::nothing(Span::test_data()))
                }
            }
            _ => {
                // E.g. multiple semicolon-separated statements
                None
            }
        }
    }
}
