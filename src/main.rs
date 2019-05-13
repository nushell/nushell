#![feature(crate_visibility_modifier)]
#![feature(in_band_lifetimes)]
#![allow(unused)]

mod commands;
mod env;
mod errors;
mod format;
mod object;
mod parser;
mod prelude;

crate use crate::commands::args::{Args, Streams};
use crate::commands::command::ReturnValue;
crate use crate::commands::command::{Command, CommandAction, CommandBlueprint};
crate use crate::env::{Environment, Host};
crate use crate::errors::ShellError;
crate use crate::format::RenderView;
use crate::object::base::{ToEntriesView, ToGenericView};
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

type Commands = BTreeMap<String, Box<dyn crate::CommandBlueprint>>;

struct Context {
    commands: BTreeMap<String, Box<dyn crate::CommandBlueprint>>,
    host: Box<dyn crate::Host>,
    env: Environment,
}

impl Context {
    fn basic() -> Result<Context, Box<Error>> {
        Ok(Context {
            commands: BTreeMap::new(),
            host: Box::new(crate::env::host::BasicHost),
            env: crate::Environment::basic()?,
        })
    }
}

fn main() -> Result<(), Box<Error>> {
    let mut rl = Editor::<()>::new();
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }

    let mut context = Context::basic()?;

    // let mut commands = BTreeMap::<String, Box<dyn crate::CommandBlueprint>>::new();

    let mut system = Rc::new(RefCell::new(sysinfo::System::new()));
    let mut ps = crate::commands::ps::PsBlueprint::new(system);
    let mut ls = crate::commands::ls::LsBlueprint;
    let mut cd = crate::commands::cd::CdBlueprint;
    let mut to_array = crate::commands::to_array::ToArrayBlueprint;

    context.commands.insert("ps".to_string(), Box::new(ps));
    context.commands.insert("ls".to_string(), Box::new(ls));
    context.commands.insert("cd".to_string(), Box::new(cd));
    context
        .commands
        .insert("to-array".to_string(), Box::new(to_array));

    loop {
        let readline = rl.readline(&format!(
            "{}> ",
            Color::Green.paint(context.env.cwd().display().to_string())
        ));

        match readline {
            Ok(line) => {
                let result = crate::parser::shell_parser(&line)
                    .map_err(|e| ShellError::string(format!("{:?}", e)))?;

                let parsed = result.1;

                rl.add_history_entry(line.as_ref());

                let mut input = VecDeque::new();

                for item in parsed {
                    // println!("Processing {:?}", item);
                    input = process_command(
                        crate::parser::print_items(&item),
                        item.clone(),
                        input,
                        &mut context,
                    )?;

                    // println!("OUTPUT: {:?}", input);
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
    context: &mut Context,
) -> Result<VecDeque<Value>, ShellError> {
    let command = &parsed[0].name();
    let arg_list = parsed[1..]
        .iter()
        .map(|i| Value::string(i.name().to_string()))
        .collect();

    let streams = Streams::new();

    // let args = Args::new(arg_list);

    match *command {
        "format" => {
            for item in input {
                let view = item.to_generic_view();
                crate::format::print_rendered(&view.render_view(&context.host), &mut context.host);
            }

            Ok(VecDeque::new())
        }

        command => match context.commands.get_mut(command) {
            Some(command) => {
                let mut instance = command.create(arg_list, &context.host, &mut context.env)?;

                let mut result = instance.run(input)?;
                let mut next = VecDeque::new();

                for v in result {
                    match v {
                        ReturnValue::Action(action) => match action {
                            crate::CommandAction::ChangeCwd(cwd) => context.env.cwd = cwd,
                        },

                        ReturnValue::Value(v) => next.push_back(v),
                    }
                }

                Ok(next)
            }

            other => {
                Exec::shell(line).cwd(context.env.cwd()).join().unwrap();
                Ok(VecDeque::new())
            }
        },
    }
}
