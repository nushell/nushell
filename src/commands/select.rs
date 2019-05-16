use crate::errors::ShellError;
use crate::object::base::select;
use crate::object::Value;
use crate::prelude::*;
use derive_new::new;

#[derive(new)]
pub struct Select;

impl crate::Command for Select {
    fn run(&self, args: CommandArgs<'caller>) -> Result<VecDeque<ReturnValue>, ShellError> {
        if args.args.is_empty() {
            return Err(ShellError::string("select requires a field"));
        }

        let fields: Result<Vec<String>, _> = args.args.iter().map(|a| a.as_string()).collect();
        let fields = fields?;

        let objects = args
            .input
            .iter()
            .map(|item| Value::Object(select(item, &fields)))
            .map(|item| ReturnValue::Value(item))
            .collect();

        Ok(objects)
    }
}
