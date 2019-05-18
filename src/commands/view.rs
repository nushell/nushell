use crate::errors::ShellError;
use crate::prelude::*;
use derive_new::new;
use prettyprint::PrettyPrinter;

#[derive(new)]
pub struct View;

impl crate::Command for View {
    fn run(&self, args: CommandArgs<'caller>) -> Result<VecDeque<ReturnValue>, ShellError> {
        let target = match args.args.first() {
            // TODO: This needs better infra
            None => return Err(ShellError::string(format!("cat must take one arg"))),
            Some(v) => v.as_string()?.clone(),
        };

        let cwd = args.env.cwd().to_path_buf();

        let printer = PrettyPrinter::default()
            .line_numbers(false)
            .header(false)
            .grid(false)
            .build()
            .map_err(|e| ShellError::string(e))?;

        let file = cwd.join(target);

        let _ = printer.file(file.display().to_string());

        Ok(VecDeque::new())
    }
}
