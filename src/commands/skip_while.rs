use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::prelude::*;

pub struct SkipWhile;

#[derive(Deserialize)]
pub struct SkipWhileArgs {
    condition: value::Block,
}

impl WholeStreamCommand for SkipWhile {
    fn name(&self) -> &str {
        "skip-while"
    }

    fn signature(&self) -> Signature {
        Signature::build("skip-while")
            .required("condition", SyntaxType::Block)
            .filter()
    }

    fn usage(&self) -> &str {
        "Skips rows while the condition matches."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, skip_while)?.run()
    }
}

pub fn skip_while(
    SkipWhileArgs { condition }: SkipWhileArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let objects = input.values.skip_while(move |item| {
        let result = condition.invoke(&item);

        let return_value = match result {
            Ok(ref v) if v.is_true() => true,
            _ => false,
        };

        futures::future::ready(return_value)
    });

    Ok(objects.from_input_stream())
}
