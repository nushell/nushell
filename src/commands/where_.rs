use crate::errors::ShellError;
use crate::object::base::find;
use crate::prelude::*;
use derive_new::new;

#[derive(new)]
pub struct Where;

impl crate::Command for Where {
    fn run(&self, args: CommandArgs<'caller>) -> Result<VecDeque<ReturnValue>, ShellError> {
        if args.args.is_empty() {
            return Err(ShellError::string("select requires a field"));
        }

        let field: Result<String, _> = args.args[0].as_string();
        let field = field?;

        let op: Result<String, _> = args.args[1].as_string();
        let op = op?;

        let objects = args
            .input
            .iter()
            .filter(|item| find(&item, &field, &op, &args.args[2]))
            .map(|item| ReturnValue::Value(item.copy()))
            .collect();

        Ok(objects)
    }
}
