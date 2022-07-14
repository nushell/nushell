use std::path::Path;

use super::commands::{DescribeDb, ToDataBase};
use super::expressions::ExprAsNu;
use crate::SQLiteDatabase;
use nu_engine::{eval_block, CallExt};
use nu_parser::parse;
use nu_protocol::{
    engine::{Command, EngineState, Stack, StateWorkingSet},
    Category, IntoPipelineData, PipelineData, Signature, Span, Type,
};

#[derive(Clone)]
pub struct CustomOpen;

impl Command for CustomOpen {
    fn name(&self) -> &str {
        "open"
    }

    fn usage(&self) -> &str {
        "Mock open file command"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .required(
                "filename",
                nu_protocol::SyntaxShape::String,
                "the filename to use",
            )
            .input_type(Type::Any)
            .output_type(Type::Custom("database".into()))
            .category(Category::Custom("database".into()))
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &nu_protocol::ast::Call,
        _input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let path: String = call.req(engine_state, stack, 0)?;
        let path = Path::new(&path);

        let db = SQLiteDatabase::new(path);
        Ok(db.into_value(call.head).into_pipeline_data())
    }
}

pub fn test_database(cmds: Vec<Box<dyn Command + 'static>>) {
    if cmds.is_empty() {
        panic!("Empty commands vector")
    }

    // The first element in the cmds vector must be the one tested
    let examples = cmds[0].examples();
    let mut engine_state = Box::new(EngineState::new());

    let delta = {
        // Base functions that are needed for testing
        // Try to keep this working set small to keep tests running as fast as possible
        let mut working_set = StateWorkingSet::new(&*engine_state);
        working_set.add_decl(Box::new(DescribeDb {}));
        working_set.add_decl(Box::new(ToDataBase {}));
        working_set.add_decl(Box::new(CustomOpen {}));
        working_set.add_decl(Box::new(ExprAsNu {}));

        // Adding the command that is being tested to the working set
        for cmd in cmds {
            working_set.add_decl(cmd);
        }

        working_set.render()
    };

    engine_state
        .merge_delta(delta)
        .expect("Error merging delta");

    for example in examples {
        // Skip tests that don't have results to compare to
        if example.result.is_none() {
            continue;
        }
        let start = std::time::Instant::now();

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
