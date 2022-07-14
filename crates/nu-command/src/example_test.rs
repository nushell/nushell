#[cfg(test)]
use nu_engine::eval_block;
#[cfg(test)]
use nu_parser::parse;
#[cfg(test)]
use nu_protocol::{
    engine::{Command, EngineState, Stack, StateWorkingSet},
    PipelineData, Span, Value,
};

#[cfg(test)]
use crate::To;

#[cfg(test)]
use super::{
    Ansi, Date, From, If, Into, Math, Path, Random, Split, SplitColumn, SplitRow, Str, StrCollect,
    StrLength, StrReplace, Url, Wrap,
};

#[cfg(test)]
pub fn test_examples(cmd: impl Command + 'static) {
    use crate::BuildString;

    let examples = cmd.examples();
    let mut engine_state = Box::new(EngineState::new());

    let delta = {
        // Base functions that are needed for testing
        // Try to keep this working set small to keep tests running as fast as possible
        let mut working_set = StateWorkingSet::new(&*engine_state);
        working_set.add_decl(Box::new(Str));
        working_set.add_decl(Box::new(StrCollect));
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

        use super::Echo;
        working_set.add_decl(Box::new(Echo));
        // Adding the command that is being tested to the working set
        working_set.add_decl(Box::new(cmd));

        working_set.render()
    };

    let cwd = std::env::current_dir().expect("Could not get current working directory.");

    engine_state
        .merge_delta(delta)
        .expect("Error merging delta");

    for example in examples {
        // Skip tests that don't have results to compare to
        if example.result.is_none() {
            continue;
        }
        let start = std::time::Instant::now();

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

        let (block, delta) = {
            let mut working_set = StateWorkingSet::new(&*engine_state);
            let (output, err) = parse(
                &mut working_set,
                None,
                example.example.as_bytes(),
                false,
                &[],
            );

            if let Some(err) = err {
                panic!("test parse error in `{}`: {:?}", example.example, err)
            }

            (output, working_set.render())
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let mut stack = Stack::new();

        // Set up PWD
        stack.add_env_var(
            "PWD".to_string(),
            Value::String {
                val: cwd.to_string_lossy().to_string(),
                span: Span::test_data(),
            },
        );

        match eval_block(
            &engine_state,
            &mut stack,
            &block,
            PipelineData::new(Span::test_data()),
            true,
            true,
        ) {
            Err(err) => panic!("test eval error in `{}`: {:?}", example.example, err),
            Ok(result) => {
                let result = result.into_value(Span::test_data());
                println!("input: {}", example.example);
                println!("result: {:?}", result);
                println!("done: {:?}", start.elapsed());

                // Note. Value implements PartialEq for Bool, Int, Float, String and Block
                // If the command you are testing requires to compare another case, then
                // you need to define its equality in the Value struct
                if let Some(expected) = example.result {
                    if result != expected {
                        panic!(
                            "the example result is different to expected value: {:?} != {:?}",
                            result, expected
                        )
                    }
                }
            }
        }
    }
}
