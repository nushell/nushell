use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{
    CallInfo, ReturnSuccess, ReturnValue, Signature, SyntaxShape, UntaggedValue, Value,
};

use rand::seq::SliceRandom;
use rand::thread_rng;

#[derive(Default)]
pub struct Shuffle {
    values: Vec<ReturnValue>,
    limit: Option<u64>,
}

impl Shuffle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn setup(&mut self, call_info: CallInfo) -> ReturnValue {
        self.limit = if let Some(value) = call_info.args.get("num") {
            Some(value.as_u64()?)
        } else {
            None
        };
        ReturnSuccess::value(UntaggedValue::nothing().into_untagged_value())
    }
}

impl Plugin for Shuffle {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("shuffle")
            .desc("Shuffle input randomly")
            .named(
                "num",
                SyntaxShape::Int,
                "Limit output to `num` number of values",
                Some('n'),
            )
            .filter())
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        self.values.push(input.into());
        Ok(vec![])
    }

    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        let mut rng = thread_rng();
        if let Some(n) = self.limit {
            println!("Limited");
            let (shuffled, _) = self.values.partial_shuffle(&mut rng, n as usize);
            Ok(shuffled.to_vec())
        } else {
            self.values.shuffle(&mut rng);
            Ok(self.values.clone())
        }
    }
}
