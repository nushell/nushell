#[cfg(test)]
use nu_protocol::engine::Command;

#[cfg(test)]
/// Runs the test examples in the passed in command and check their signatures and return values.
///
/// # Panics
/// If you get a ExternalNotSupported panic, you may be using a command
/// that's not in the default working set of the test harness.
/// You may want to use test_examples_with_commands and include any other dependencies.
pub fn test_examples(cmd: impl Command + 'static) {
    test_examples::test_examples(cmd, &[]);
}

#[cfg(test)]
pub fn test_examples_with_commands(cmd: impl Command + 'static, commands: &[&dyn Command]) {
    test_examples::test_examples(cmd, commands);
}

#[cfg(test)]
mod test_examples {
    use super::super::{
        Ansi, Date, Enumerate, Filter, First, Flatten, From, Get, Into, IntoDatetime, IntoString,
        Lines, Math, MathRound, MathSum, ParEach, Path, PathParse, Random, Seq, Sort, SortBy,
        Split, SplitColumn, SplitRow, Str, StrJoin, StrLength, StrReplace, Update, Url, Values,
        Wrap,
    };
    use crate::{Default, Each, To};
    use nu_cmd_lang::example_support::{
        check_all_signature_input_output_types_entries_have_examples,
        check_example_evaluates_to_expected_output,
        check_example_input_and_output_types_match_command_signature,
    };
    use nu_cmd_lang::{Break, Echo, If, Let, Mut};
    use nu_protocol::{
        engine::{Command, EngineState, StateWorkingSet},
        Type,
    };
    use std::collections::HashSet;

    pub fn test_examples(cmd: impl Command + 'static, commands: &[&dyn Command]) {
        let examples = cmd.examples();
        let signature = cmd.signature();
        let mut engine_state = make_engine_state(cmd.clone_box(), commands);

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
                    &mut make_engine_state(cmd.clone_box(), commands),
                    &signature.input_output_types,
                    signature.operates_on_cell_paths(),
                ),
            );
            check_example_evaluates_to_expected_output(&example, cwd.as_path(), &mut engine_state);
        }

        check_all_signature_input_output_types_entries_have_examples(
            signature,
            witnessed_type_transformations,
        );
    }

    fn make_engine_state(cmd: Box<dyn Command>, commands: &[&dyn Command]) -> Box<EngineState> {
        let mut engine_state = Box::new(EngineState::new());

        let delta = {
            // Base functions that are needed for testing
            // Try to keep this working set small to keep tests running as fast as possible
            let mut working_set = StateWorkingSet::new(&engine_state);
            working_set.add_decl(Box::new(Ansi));
            working_set.add_decl(Box::new(Break));
            working_set.add_decl(Box::new(Date));
            working_set.add_decl(Box::new(Default));
            working_set.add_decl(Box::new(Each));
            working_set.add_decl(Box::new(Echo));
            working_set.add_decl(Box::new(Enumerate));
            working_set.add_decl(Box::new(Filter));
            working_set.add_decl(Box::new(First));
            working_set.add_decl(Box::new(Flatten));
            working_set.add_decl(Box::new(From));
            working_set.add_decl(Box::new(Get));
            working_set.add_decl(Box::new(If));
            working_set.add_decl(Box::new(Into));
            working_set.add_decl(Box::new(IntoString));
            working_set.add_decl(Box::new(IntoDatetime));
            working_set.add_decl(Box::new(Let));
            working_set.add_decl(Box::new(Lines));
            working_set.add_decl(Box::new(Math));
            working_set.add_decl(Box::new(MathRound));
            working_set.add_decl(Box::new(MathSum));
            working_set.add_decl(Box::new(Mut));
            working_set.add_decl(Box::new(Path));
            working_set.add_decl(Box::new(PathParse));
            working_set.add_decl(Box::new(ParEach));
            working_set.add_decl(Box::new(Random));
            working_set.add_decl(Box::new(Seq));
            working_set.add_decl(Box::new(Sort));
            working_set.add_decl(Box::new(SortBy));
            working_set.add_decl(Box::new(Split));
            working_set.add_decl(Box::new(SplitColumn));
            working_set.add_decl(Box::new(SplitRow));
            working_set.add_decl(Box::new(Str));
            working_set.add_decl(Box::new(StrJoin));
            working_set.add_decl(Box::new(StrLength));
            working_set.add_decl(Box::new(StrReplace));
            working_set.add_decl(Box::new(To));
            working_set.add_decl(Box::new(Url));
            working_set.add_decl(Box::new(Update));
            working_set.add_decl(Box::new(Values));
            working_set.add_decl(Box::new(Wrap));

            // Add any extra commands that the test harness needs
            for command in commands {
                working_set.add_decl(command.clone_box());
            }

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
