use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};

pub struct Ls;

#[async_trait]
impl WholeStreamCommand for Ls {
    fn name(&self) -> &str {
        "ls"
    }

    fn signature(&self) -> Signature {
        Signature::build("ls")
            .optional(
                "path",
                SyntaxShape::GlobPattern,
                "a path to get the directory contents from",
            )
            .switch("all", "Show hidden files", Some('a'))
            .switch(
                "long",
                "List all available columns for each entry",
                Some('l'),
            )
            .switch(
                "short-names",
                "Only print the file names and not the path",
                Some('s'),
            )
            .switch(
                "du",
                "Display the apparent directory size in place of the directory metadata size",
                Some('d'),
            )
    }

    fn usage(&self) -> &str {
        "View the contents of the current or given path."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let name = args.call_info.name_tag.clone();
        let ctrl_c = args.ctrl_c.clone();
        let shell_manager = args.shell_manager.clone();
        let (args, _) = args.process().await?;
        shell_manager.ls(args, name, ctrl_c)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "List all files in the current directory",
                example: "ls",
                result: None,
            },
            Example {
                description: "List all files in a subdirectory",
                example: "ls subdir",
                result: None,
            },
            Example {
                description: "List all rust files",
                example: "ls *.rs",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::Ls;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Ls {})
    }
}
