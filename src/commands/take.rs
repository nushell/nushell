use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;
use derive_new::new;

#[derive(new)]
pub struct TakeBlueprint;

impl crate::CommandBlueprint for TakeBlueprint {
    fn create(
        &self,
        args: Vec<Value>,
        _host: &dyn Host,
        _env: &mut Environment,
    ) -> Result<Box<dyn Command>, ShellError> {
        if args.is_empty() {
            return Err(ShellError::string("take requires an integer"));
        }

        let amount = args[0].as_int()?;

        Ok(Box::new(Take { amount }))
    }
}

#[derive(new)]
pub struct Take {
    amount: i64,
}

impl crate::Command for Take {
    fn run(&mut self, stream: VecDeque<Value>) -> Result<VecDeque<ReturnValue>, ShellError> {
        let amount = if stream.len() > self.amount as usize {
            self.amount as usize
        } else {
            stream.len()
        };

        let out: VecDeque<ReturnValue> = stream
            .into_iter()
            .take(amount)
            .map(|v| ReturnValue::Value(v))
            .collect();

        Ok(out)
    }
}
