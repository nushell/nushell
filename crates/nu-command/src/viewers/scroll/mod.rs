mod pager;

use nu_engine::{get_columns, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Scroll;

impl Command for Scroll {
    fn name(&self) -> &str {
        "scroll"
    }

    fn usage(&self) -> &str {
        "11231321123"
    }

    fn signature(&self) -> nu_protocol::Signature {
        // todo: Fix error message when it's empty
        // if we set h i short flags it panics????

        Signature::build("tabless")
            .named("head", SyntaxShape::Boolean, "xxxx", None)
            .switch("index", "asdsad", Some('i'))
            .switch("reverse", "asdsad", Some('r'))
            .category(Category::Viewers)
    }

    fn extra_usage(&self) -> &str {
        "11231321123"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let show_head: bool = call.get_flag(engine_state, stack, "head")?.unwrap_or(true);
        let show_index: bool = call.has_flag("index");
        let is_reverse: bool = call.has_flag("reverse");

        let ctrlc = engine_state.ctrlc.clone();
        let config = engine_state.get_config();

        let (columns, data) = match input {
            PipelineData::Value(value, ..) => match value {
                Value::Record { cols, vals, .. } => (cols, vals),
                Value::List { vals, .. } => (get_columns(&vals), vals),
                value => (Vec::new(), vec![value]),
            },
            PipelineData::ListStream(mut stream, ..) => {
                let mut data = vec![];
                for item in stream.by_ref() {
                    data.push(item);
                }

                let cols = get_columns(&data);

                (cols, data)
            }
            input => todo!("{:?}", input),
        };

        pager::handler(
            &columns, &data, config, ctrlc, show_index, show_head, is_reverse,
        );

        Ok(PipelineData::Value(Value::default(), None))
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}
