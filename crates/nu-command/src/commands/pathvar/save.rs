use super::get_var;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};
use nu_test_support::NATIVE_PATH_ENV_SEPARATOR;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "pathvar save"
    }

    fn signature(&self) -> Signature {
        Signature::build("pathvar save").named(
            "var",
            SyntaxShape::String,
            "Use a different variable than PATH",
            Some('v'),
        )
    }

    fn usage(&self) -> &str {
        "Save the current pathvar to the config file"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        save(args)
    }
}
pub fn save(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let ctx = &args.context;

    let var = get_var(&args)?;
    let var_lower = var.clone().map(|s| s.to_lowercase());

    if let Some(global_cfg) = &mut ctx.configs().lock().global_config {
        if let Some(pathvar) = ctx.scope.get_env(&var) {
            let paths: Vec<Value> = pathvar
                .split(NATIVE_PATH_ENV_SEPARATOR)
                .map(Value::from)
                .collect();

            let span_range = 0..paths.len();
            let row = Value::new(
                UntaggedValue::Table(paths),
                Tag::from(Span::from(&span_range)),
            );

            global_cfg.vars.insert(var_lower.item, row);
            global_cfg.write()?;
            ctx.reload_config(global_cfg)?;

            Ok(OutputStream::empty())
        } else {
            Err(ShellError::unexpected(&format!(
                "Variable {} not set",
                &var.item
            )))
        }
    } else {
        let value = UntaggedValue::Error(crate::commands::config::err_no_global_cfg_present())
            .into_value(name);

        Ok(OutputStream::one(value))
    }
}
