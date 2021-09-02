use crate::{Example, Signature};

pub trait Command {
    fn name(&self) -> &str;

    fn signature(&self) -> Signature {
        Signature::new(self.name()).desc(self.usage()).filter()
    }

    fn usage(&self) -> &str;

    fn extra_usage(&self) -> &str {
        ""
    }

    // fn run(&self, args: CommandArgs) -> Result<InputStream, ShellError> {
    //     let context = args.context.clone();
    //     let stream = self.run_with_actions(args)?;

    //     Ok(Box::new(crate::evaluate::internal::InternalIterator {
    //         context,
    //         input: stream,
    //         leftovers: InputStream::empty(),
    //     })
    //     .into_output_stream())
    // }

    fn is_binary(&self) -> bool {
        false
    }

    // Commands that are not meant to be run by users
    fn is_private(&self) -> bool {
        false
    }

    fn examples(&self) -> Vec<Example> {
        Vec::new()
    }

    // This is a built-in command
    fn is_builtin(&self) -> bool {
        true
    }

    // Is a sub command
    fn is_sub(&self) -> bool {
        self.name().contains(' ')
    }

    // Is a plugin command
    fn is_plugin(&self) -> bool {
        false
    }

    // Is a custom command i.e. def blah [] { }
    fn is_custom(&self) -> bool {
        false
    }
}
