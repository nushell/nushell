use crate::prelude::*;

use crate::commands::classified::{ClassifiedCommand, ExternalCommand, InternalCommand};
use crate::commands::command::ReturnValue;
use crate::context::Context;
crate use crate::env::Host;
crate use crate::errors::ShellError;
crate use crate::format::{EntriesListView, GenericView};
use crate::object::Value;

use rustyline::error::ReadlineError;
use rustyline::{self, ColorMode, Config, Editor};
use std::collections::VecDeque;
use std::error::Error;
use std::sync::Arc;

#[derive(Debug)]
pub enum MaybeOwned<'a, T> {
    Owned(T),
    Borrowed(&'a T),
}

impl<T> MaybeOwned<'a, T> {
    crate fn borrow(&self) -> &T {
        match self {
            MaybeOwned::Owned(v) => v,
            MaybeOwned::Borrowed(v) => v,
        }
    }
}

pub fn cli() -> Result<(), Box<Error>> {
    let config = Config::builder().color_mode(ColorMode::Forced).build();
    let h = crate::shell::Helper::new();
    let mut rl: Editor<crate::shell::Helper> = Editor::with_config(config);

    #[cfg(windows)]
    {
        let _ = ansi_term::enable_ansi_support();
    }

    rl.set_helper(Some(h));
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }

    let mut context = Context::basic()?;

    {
        use crate::commands::*;

        context.add_commands(vec![
            ("format", Arc::new(format)),
            ("format-list", Arc::new(format_list)),
            ("ps", Arc::new(ps::ps)),
            ("ls", Arc::new(ls::ls)),
            ("cd", Arc::new(cd::cd)),
            ("view", Arc::new(view::view)),
            ("skip", Arc::new(skip::skip)),
            ("take", Arc::new(take::take)),
            ("select", Arc::new(select::select)),
            ("reject", Arc::new(reject::reject)),
            ("to-array", Arc::new(to_array::to_array)),
            ("where", Arc::new(where_::r#where)),
            ("sort-by", Arc::new(sort_by::sort_by)),
        ]);
    }

    loop {
        let readline = rl.readline(&format!("{}> ", context.env.cwd().display().to_string()));

        match process_line(readline, &mut context) {
            LineResult::Success(line) => {
                rl.add_history_entry(line.clone());
            }

            LineResult::Error(err) => {
                context.host.stdout(&err);
            }

            LineResult::Break => {
                break;
            }

            LineResult::FatalError(err) => {
                context
                    .host
                    .stdout(&format!("A surprising fatal error occurred.\n{:?}", err));
            }
        }
    }
    rl.save_history("history.txt").unwrap();

    Ok(())
}

enum LineResult {
    Success(String),
    Error(String),
    Break,

    #[allow(unused)]
    FatalError(ShellError),
}

fn process_line(readline: Result<String, ReadlineError>, ctx: &mut Context) -> LineResult {
    match &readline {
        Ok(line) if line.trim() == "exit" => LineResult::Break,

        Ok(line) if line.trim() == "" => LineResult::Success(line.clone()),

        Ok(line) => {
            let result = match crate::parser::shell_parser(&line) {
                Err(err) => {
                    return LineResult::Error(format!("{:?}", err));
                }

                Ok(val) => val,
            };

            let parsed = result.1;

            let mut input = VecDeque::new();

            for item in parsed {
                input = match process_command(item.clone(), input, ctx) {
                    Ok(val) => val,
                    Err(err) => return LineResult::Error(format!("{}", err.description())),
                };
            }

            if input.len() > 0 {
                if equal_shapes(&input) {
                    let array = crate::commands::stream_to_array(input);
                    let args = CommandArgs::from_context(ctx, vec![], array);
                    match format(args) {
                        Ok(_) => {}
                        Err(err) => return LineResult::Error(err.to_string()),
                    }
                } else {
                    let args = CommandArgs::from_context(ctx, vec![], input);
                    match format(args) {
                        Ok(_) => {}
                        Err(err) => return LineResult::Error(err.to_string()),
                    }
                }
            }

            LineResult::Success(line.to_string())
        }
        Err(ReadlineError::Interrupted) => {
            println!("CTRL-C");
            LineResult::Break
        }
        Err(ReadlineError::Eof) => {
            println!("CTRL-D");
            LineResult::Break
        }
        Err(err) => {
            println!("Error: {:?}", err);
            LineResult::Break
        }
    }
}

fn process_command(
    parsed: Vec<crate::parser::Item>,
    input: VecDeque<Value>,
    context: &mut Context,
) -> Result<VecDeque<Value>, ShellError> {
    let command = classify_command(&parsed, context)?;

    command.run(input, context)
}

fn classify_command(
    command: &[crate::parser::Item],
    context: &Context,
) -> Result<ClassifiedCommand, ShellError> {
    let command_name = &command[0].name()?;

    let arg_list: Vec<Value> = command[1..].iter().map(|i| i.as_value()).collect();
    let arg_list_strings: Vec<String> = command[1..].iter().map(|i| i.print()).collect();

    match *command_name {
        other => match context.has_command(*command_name) {
            true => {
                let command = context.get_command(command_name);
                Ok(ClassifiedCommand::Internal(InternalCommand {
                    command,
                    args: arg_list,
                }))
            }
            false => Ok(ClassifiedCommand::External(ExternalCommand {
                name: other.to_string(),
                args: arg_list_strings,
            })),
        },
    }
}

fn format(args: CommandArgs<'caller>) -> Result<VecDeque<ReturnValue>, ShellError> {
    let last = args.input.len() - 1;
    for (i, item) in args.input.iter().enumerate() {
        let view = GenericView::new(item);
        crate::format::print_view(&view, args.host);

        if last != i {
            println!("");
        }
    }

    Ok(VecDeque::new())
}

fn format_list(args: CommandArgs<'caller>) -> Result<VecDeque<ReturnValue>, ShellError> {
    let view = EntriesListView::from_stream(args.input);
    crate::format::print_view(&view, args.host);

    Ok(VecDeque::new())
}

fn equal_shapes(input: &VecDeque<Value>) -> bool {
    let mut items = input.iter();

    let item = match items.next() {
        Some(item) => item,
        None => return false,
    };

    let desc = item.data_descriptors();

    for item in items {
        if desc != item.data_descriptors() {
            return false;
        }
    }

    true
}
