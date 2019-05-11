#![feature(crate_visibility_modifier)]
#![feature(in_band_lifetimes)]
#![allow(unused)]

mod commands;
mod env;
mod errors;
mod format;
mod object;
mod parser;

crate use crate::commands::command::{Command, CommandAction, CommandBlueprint, CommandSuccess};
crate use crate::env::{Environment, Host};
crate use crate::errors::ShellError;
crate use crate::format::RenderView;
use crate::object::base::{ToEntriesView, ToGenericView};
use ansi_term::Color;
use conch_parser::lexer::Lexer;
use conch_parser::parse::DefaultParser;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::cell::RefCell;
use std::collections::BTreeMap;
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

fn main() -> Result<(), Box<Error>> {
    let mut rl = Editor::<()>::new();
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }

    let mut host = crate::env::host::BasicHost;
    let mut env = crate::Environment::basic()?;

    let mut commands = BTreeMap::<String, Box<dyn crate::CommandBlueprint>>::new();

    let mut system = Rc::new(RefCell::new(sysinfo::System::new()));
    let mut ps = crate::commands::ps::PsBlueprint::new(system);
    let mut ls = crate::commands::ls::LsBlueprint;
    let mut cd = crate::commands::cd::CdBlueprint;

    commands.insert("ps".to_string(), Box::new(ps));
    commands.insert("ls".to_string(), Box::new(ls));
    commands.insert("cd".to_string(), Box::new(cd));

    loop {
        let readline = rl.readline(&format!(
            "{}> ",
            Color::Green.paint(env.cwd().display().to_string())
        ));

        match readline {
            Ok(line) => {
                let result = crate::parser::shell_parser(&line)
                    .map_err(|e| ShellError::new(format!("{:?}", e)))?;

                let parsed = result.1;

                rl.add_history_entry(line.as_ref());

                if parsed.len() > 1 {
                    println!("Piping is not yet implemented");
                }

                let command = &parsed[0][0].name();
                let args = parsed[0][1..]
                    .iter()
                    .map(|i| i.name().to_string())
                    .collect();

                match commands.get_mut(*command) {
                    Some(command) => {
                        let mut instance = command.create(args, &mut host, &mut env);
                        let result = instance.run()?;

                        for action in result.action {
                            match action {
                                crate::CommandAction::ChangeCwd(cwd) => env.cwd = cwd,
                            }
                        }

                        let view = result.value.to_generic_view();
                        let rendered = view.render_view(&mut host);

                        for line in rendered {
                            match line.as_ref() {
                                "\n" => println!(""),
                                line => println!("{}", line),
                            }
                        }
                    }

                    other => {
                        Exec::shell(line).cwd(env.cwd()).join().unwrap();
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
