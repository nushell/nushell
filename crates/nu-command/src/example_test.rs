use std::{cell::RefCell, rc::Rc};

use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    engine::{Command, EngineState, EvaluationContext, StateWorkingSet},
    Value,
};

use super::From;

pub fn test_examples(cmd: impl Command + 'static) {
    let examples = cmd.examples();
    let engine_state = Rc::new(RefCell::new(EngineState::new()));

    let delta = {
        // Base functions that are needed for testing
        // Try to keep this working set as small to keep tests running as fast as possible
        let engine_state = engine_state.borrow();
        let mut working_set = StateWorkingSet::new(&*engine_state);
        working_set.add_decl(Box::new(From));

        // Adding the command that is being tested to the working set
        working_set.add_decl(Box::new(cmd));

        working_set.render()
    };

    EngineState::merge_delta(&mut *engine_state.borrow_mut(), delta);

    for example in examples {
        let start = std::time::Instant::now();

        let (block, delta) = {
            let engine_state = engine_state.borrow();
            let mut working_set = StateWorkingSet::new(&*engine_state);
            let (output, err) = parse(&mut working_set, None, example.example.as_bytes(), false);

            if let Some(err) = err {
                panic!("test parse error: {:?}", err)
            }

            (output, working_set.render())
        };

        EngineState::merge_delta(&mut *engine_state.borrow_mut(), delta);

        let state = EvaluationContext {
            engine_state: engine_state.clone(),
            stack: nu_protocol::engine::Stack::new(),
        };

        match eval_block(&state, &block, Value::nothing()) {
            Err(err) => panic!("test eval error: {:?}", err),
            Ok(result) => {
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
