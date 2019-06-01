use crate::errors::ShellError;
use crate::prelude::*;
use prettyprint::PrettyPrinter;

pub fn view(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let target = match args.positional.first() {
        // TODO: This needs better infra
        None => return Err(ShellError::string(format!("cat must take one arg"))),
        Some(v) => v.as_string()?.clone(),
    };

    let cwd = args.env.lock().unwrap().cwd().to_path_buf();

    let printer = PrettyPrinter::default()
        .line_numbers(false)
        .header(false)
        .grid(false)
        .build()
        .map_err(|e| ShellError::string(e))?;

    let file = cwd.join(target);

    let _ = printer.file(file.display().to_string());

    Ok(VecDeque::new().boxed())
}
