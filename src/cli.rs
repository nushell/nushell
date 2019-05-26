use crate::prelude::*;

use crate::commands::classified::{
    ClassifiedCommand, ClassifiedInputStream, ClassifiedPipeline, ExternalCommand, InternalCommand,
    StreamNext,
};
use crate::context::Context;
crate use crate::errors::ShellError;
crate use crate::format::{EntriesListView, GenericView};
use crate::object::Value;
use crate::parser::{ParsedCommand, Pipeline};
use crate::stream::empty_stream;

use log::debug;
use rustyline::error::ReadlineError;
use rustyline::{self, ColorMode, Config, Editor};
use std::collections::VecDeque;
use std::error::Error;
use std::iter::Iterator;
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

pub async fn cli() -> Result<(), Box<Error>> {
    let mut context = Context::basic()?;

    {
        use crate::commands::*;

        context.add_commands(vec![
            ("format-list", Arc::new(format_list)),
            ("ps", Arc::new(ps::ps)),
            ("ls", Arc::new(ls::ls)),
            ("cd", Arc::new(cd::cd)),
            ("view", Arc::new(view::view)),
            ("skip", Arc::new(skip::skip)),
            ("first", Arc::new(take::take)),
            ("select", Arc::new(select::select)),
            ("split", Arc::new(split::split)),
            ("reject", Arc::new(reject::reject)),
            ("to-array", Arc::new(to_array::to_array)),
            ("where", Arc::new(where_::r#where)),
            ("sort-by", Arc::new(sort_by::sort_by)),
        ]);
    }

    let config = Config::builder().color_mode(ColorMode::Forced).build();
    let h = crate::shell::Helper::new(context.clone_commands());
    let mut rl: Editor<crate::shell::Helper> = Editor::with_config(config);

    #[cfg(windows)]
    {
        let _ = ansi_term::enable_ansi_support();
    }

    rl.set_helper(Some(h));
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }

    loop {
        let readline = rl.readline(&format!(
            "{}> ",
            context.env.lock().unwrap().cwd().display().to_string()
        ));

        match process_line(readline, &mut context).await {
            LineResult::Success(line) => {
                rl.add_history_entry(line.clone());
            }

            LineResult::Error(err) => {
                context.host.lock().unwrap().stdout(&err);
            }

            LineResult::Break => {
                break;
            }

            LineResult::FatalError(err) => {
                context
                    .host
                    .lock()
                    .unwrap()
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

impl std::ops::Try for LineResult {
    type Ok = Option<String>;
    type Error = ShellError;

    fn into_result(self) -> Result<Option<String>, ShellError> {
        match self {
            LineResult::Success(s) => Ok(Some(s)),
            LineResult::Error(s) => Err(ShellError::string(s)),
            LineResult::Break => Ok(None),
            LineResult::FatalError(err) => Err(err),
        }
    }
    fn from_error(v: ShellError) -> Self {
        LineResult::Error(v.to_string())
    }

    fn from_ok(v: Option<String>) -> Self {
        match v {
            None => LineResult::Break,
            Some(v) => LineResult::Success(v),
        }
    }
}

async fn process_line(readline: Result<String, ReadlineError>, ctx: &mut Context) -> LineResult {
    match &readline {
        Ok(line) if line.trim() == "exit" => LineResult::Break,

        Ok(line) if line.trim() == "" => LineResult::Success(line.clone()),

        Ok(line) => {
            let result = match crate::parser::parse(&line, &ctx.registry()) {
                Err(err) => {
                    return LineResult::Error(format!("{:?}", err));
                }

                Ok(val) => val,
            };

            debug!("=== Parsed ===");
            debug!("{:#?}", result);

            let pipeline = classify_pipeline(&result, ctx)?;

            let mut input = ClassifiedInputStream::new();

            let mut iter = pipeline.commands.into_iter().peekable();

            loop {
                let item: Option<ClassifiedCommand> = iter.next();
                let next: Option<&ClassifiedCommand> = iter.peek();

                input = match (item, next) {
                    (None, _) => break,

                    (
                        Some(ClassifiedCommand::Internal(left)),
                        Some(ClassifiedCommand::Internal(_)),
                    ) => match left.run(ctx, input).await {
                        Ok(val) => ClassifiedInputStream::from_input_stream(val),
                        Err(err) => return LineResult::Error(format!("{}", err.description())),
                    },

                    (Some(ClassifiedCommand::Internal(left)), None) => {
                        match left.run(ctx, input).await {
                            Ok(val) => ClassifiedInputStream::from_input_stream(val),
                            Err(err) => return LineResult::Error(format!("{}", err.description())),
                        }
                    }

                    (
                        Some(ClassifiedCommand::External(left)),
                        Some(ClassifiedCommand::External(_)),
                    ) => match left.run(ctx, input, StreamNext::External).await {
                        Ok(val) => val,
                        Err(err) => return LineResult::Error(format!("{}", err.description())),
                    },

                    (
                        Some(ClassifiedCommand::Internal(_)),
                        Some(ClassifiedCommand::External(_)),
                    ) => return LineResult::Error(format!("Unimplemented Internal -> External",)),

                    (
                        Some(ClassifiedCommand::External(left)),
                        Some(ClassifiedCommand::Internal(_)),
                    ) => match left.run(ctx, input, StreamNext::Internal).await {
                        Ok(val) => val,
                        Err(err) => return LineResult::Error(format!("{}", err.description())),
                    },

                    (Some(ClassifiedCommand::External(left)), None) => {
                        match left.run(ctx, input, StreamNext::Last).await {
                            Ok(val) => val,
                            Err(err) => return LineResult::Error(format!("{}", err.description())),
                        }
                    }
                }
            }

            let input_vec: VecDeque<_> = input.objects.collect().await;

            if input_vec.len() > 0 {
                if equal_shapes(&input_vec) {
                    let array = crate::commands::stream_to_array(input_vec.boxed()).await;
                    let args = CommandArgs::from_context(ctx, vec![], array);
                    let mut result = format(args);
                    let mut vec = vec![];
                    vec.send_all(&mut result).await?;
                } else {
                    let args = CommandArgs::from_context(ctx, vec![], input_vec.boxed());
                    format(args).collect::<Vec<_>>().await;
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

fn classify_pipeline(
    pipeline: &Pipeline,
    context: &Context,
) -> Result<ClassifiedPipeline, ShellError> {
    let commands: Result<Vec<_>, _> = pipeline
        .commands
        .iter()
        .cloned()
        .map(|item| classify_command(&item, context))
        .collect();

    Ok(ClassifiedPipeline {
        commands: commands?,
    })
}

fn classify_command(
    command: &ParsedCommand,
    context: &Context,
) -> Result<ClassifiedCommand, ShellError> {
    let command_name = &command.name[..];
    let args = &command.args;

    let arg_list: Vec<Value> = args.iter().map(|i| Value::from_expr(i)).collect();
    let arg_list_strings: Vec<String> = args.iter().map(|i| i.print()).collect();

    match command_name {
        other => match context.has_command(command_name) {
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

crate fn format(args: CommandArgs) -> OutputStream {
    let host = args.host.clone();
    let input = args.input.map(|a| a.copy());
    let input = input.collect::<Vec<_>>();

    input
        .then(move |input| {
            let last = input.len() - 1;
            let mut host = host.lock().unwrap();
            for (i, item) in input.iter().enumerate() {
                let view = GenericView::new(item);

                handle_unexpected(&mut *host, |host| crate::format::print_view(&view, host));

                if last != i {
                    host.stdout("");
                }
            }

            futures::future::ready(empty_stream())
        })
        .flatten_stream()
        .boxed()
}

crate fn format_list(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let host = args.host.clone();

    let view = EntriesListView::from_stream(args.input);

    Ok(view
        .then(move |view| {
            handle_unexpected(&mut *host.lock().unwrap(), |host| {
                crate::format::print_view(&view, host)
            });

            futures::future::ready(empty_stream())
        })
        .flatten_stream()
        .boxed())
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
