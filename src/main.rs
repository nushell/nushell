#![feature(crate_visibility_modifier)]
#![feature(in_band_lifetimes)]
#![allow(unused)]

mod commands;
mod context;
mod env;
mod errors;
mod format;
mod object;
mod parser;
mod prelude;

crate use crate::commands::args::{Args, Streams};
use crate::commands::command::ReturnValue;
crate use crate::commands::command::{Command, CommandAction, CommandBlueprint};
use crate::context::Context;
crate use crate::env::{Environment, Host};
crate use crate::errors::ShellError;
crate use crate::format::{EntriesListView, GenericView, RenderView};
use crate::object::Value;

use ansi_term::Color;
use conch_parser::lexer::Lexer;
use conch_parser::parse::DefaultParser;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use subprocess::Exec;
use sysinfo::{self, SystemExt};

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

fn main() -> Result<(), Box<Error>> {
    let mut rl = Editor::<()>::new();
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }

    let mut context = Arc::new(Mutex::new(Context::basic()?));

    {
        use crate::commands::*;

        context.lock().unwrap().add_commands(vec![
            ("ps", Box::new(ps::PsBlueprint)),
            ("ls", Box::new(ls::LsBlueprint)),
            ("cd", Box::new(cd::CdBlueprint)),
            ("take", Box::new(take::TakeBlueprint)),
            ("select", Box::new(select::SelectBlueprint)),
            ("reject", Box::new(reject::RejectBlueprint)),
            ("to-array", Box::new(to_array::ToArrayBlueprint)),
        ]);
    }

    loop {
        let readline = rl.readline(&format!(
            "{}> ",
            Color::Green.paint(context.lock().unwrap().env.cwd().display().to_string())
        ));

        match readline {
            Ok(line) => {
                let result = crate::parser::shell_parser(&line)
                    .map_err(|e| ShellError::string(format!("{:?}", e)))?;

                let parsed = result.1;

                rl.add_history_entry(line.as_ref());

                let mut input = VecDeque::new();

                let last = parsed.len() - 1;

                for item in parsed {
                    input = process_command(
                        crate::parser::print_items(&item),
                        item.clone(),
                        input,
                        context.clone(),
                    )?;
                }

                if input.len() > 0 {
                    if equal_shapes(&input) {
                        format(crate::commands::to_array(input), context.clone());
                    } else {
                        format(input, context.clone());
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    rl.save_history("history.txt").unwrap();

    Ok(())
}

fn process_command(
    line: String,
    parsed: Vec<crate::parser::Item>,
    input: VecDeque<Value>,
    context: Arc<Mutex<Context>>,
) -> Result<VecDeque<Value>, ShellError> {
    let command = &parsed[0].name();
    let arg_list = parsed[1..].iter().map(|i| i.as_value()).collect();

    let streams = Streams::new();

    if command == &"format" {
        format(input, context);

        Ok(VecDeque::new())
    } else if command == &"format-list" {
        let view = EntriesListView::from_stream(input);
        let mut ctx = context.lock().unwrap();

        crate::format::print_rendered(&view.render_view(&ctx.host), &mut ctx.host);

        Ok(VecDeque::new())
    } else {
        let mut ctx = context.lock().unwrap();

        match ctx.has_command(*command) {
            true => {
                let mut instance = ctx.create_command(command, arg_list)?;

                let mut result = instance.run(input)?;
                let mut next = VecDeque::new();

                for v in result {
                    match v {
                        ReturnValue::Action(action) => match action {
                            crate::CommandAction::ChangeCwd(cwd) => ctx.env.cwd = cwd,
                        },

                        ReturnValue::Value(v) => next.push_back(v),
                    }
                }

                Ok(next)
            }

            false => {
                Exec::shell(line).cwd(ctx.env.cwd()).join().unwrap();
                Ok(VecDeque::new())
            }
        }
    }
}

fn format(input: VecDeque<Value>, context: Arc<Mutex<Context>>) {
    let mut ctx = context.lock().unwrap();

    let last = input.len() - 1;
    for (i, item) in input.iter().enumerate() {
        let view = GenericView::new(item);
        crate::format::print_rendered(&view.render_view(&ctx.host), &mut ctx.host);

        if last != i {
            println!("");
        }
    }
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
