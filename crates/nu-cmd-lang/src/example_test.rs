#[cfg(test)]
use nu_protocol::engine::Command;

#[cfg(test)]
pub fn test_examples(cmd: impl Command + 'static) {
    test_examples::test_examples(cmd);
}

#[cfg(test)]
mod test_examples {
    use crate::example_support::{
        check_all_signature_input_output_types_entries_have_examples,
        check_example_evaluates_to_expected_output,
        check_example_input_and_output_types_match_command_signature,
    };
    use crate::{Break, Describe, Mut};
    use crate::{Echo, If, Let};
    use nu_protocol::{
        engine::{Command, EngineState, StateWorkingSet},
        Type,
    };
    use std::collections::HashSet;

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
            check_example_evaluates_to_expected_output(&example, cwd.as_path(), &mut engine_state);
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
            let mut working_set = StateWorkingSet::new(&engine_state);
            working_set.add_decl(Box::new(Break));
            working_set.add_decl(Box::new(Describe));
            working_set.add_decl(Box::new(Echo));
            working_set.add_decl(Box::new(If));
            working_set.add_decl(Box::new(Let));
            working_set.add_decl(Box::new(Mut));

            // Adding the command that is being tested to the working set
            working_set.add_decl(cmd);

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");
        engine_state
    }
}
