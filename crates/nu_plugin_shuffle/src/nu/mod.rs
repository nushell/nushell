use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{ReturnValue, Signature, Value};

use rand::seq::SliceRandom;
use rand::thread_rng;

#[derive(Default)]
pub struct Shuffle {
    values: Vec<ReturnValue>,
}

impl Shuffle {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Plugin for Shuffle {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("shuffle")
            .desc("Shuffle input randomly")
            .filter())
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        self.values.push(input.into());
        Ok(vec![])
    }

    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        let mut rng = thread_rng();
        self.values.shuffle(&mut rng);
        Ok(self.values.clone())
    }
}
