use crate::errors::ShellError;
use crate::object::base::reject;
use crate::object::Value;
use crate::prelude::*;
use derive_new::new;

#[derive(new)]
pub struct Reject;

impl crate::Command for Reject {
    fn run(&self, args: CommandArgs<'value>) -> Result<VecDeque<ReturnValue>, ShellError> {
        if args.args.is_empty() {
            return Err(ShellError::string("select requires a field"));
        }

        let fields: Result<Vec<String>, _> = args.args.iter().map(|a| a.as_string()).collect();
        let fields = fields?;

        let objects = args
            .input
            .iter()
            .map(|item| Value::Object(reject(item, &fields)))
            .map(|item| ReturnValue::Value(item))
            .collect();

        Ok(objects)
    }
}
