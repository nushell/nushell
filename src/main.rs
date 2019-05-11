#![feature(crate_visibility_modifier)]
#![feature(in_band_lifetimes)]
#![allow(unused)]

mod commands;
mod env;
mod errors;
mod format;
mod object;
mod parser;

crate use crate::commands::command::Command;
crate use crate::env::{Environment, Host};
crate use crate::format::RenderView;
crate use crate::errors::ShellError;
use crate::object::base::{ToEntriesView, ToGenericView};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::collections::BTreeMap;
use std::error::Error;
use sysinfo::{self, SystemExt};
use ansi_term::Color;
use conch_parser::lexer::Lexer;
use conch_parser::parse::DefaultParser;


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

    let mut commands = BTreeMap::<String, Box<dyn crate::Command>>::new();

    let mut system = sysinfo::System::new();
    let mut ps = crate::commands::ps::Ps::new(system);
    let mut ls = crate::commands::ls::Ls;

    commands.insert("ps".to_string(), Box::new(ps));
    commands.insert("ls".to_string(), Box::new(ls));

    loop {
        let readline = rl.readline(&format!("{}> ", Color::Green.paint(env.cwd().display().to_string())));

        match readline {
            Ok(line) => {
                let result = crate::parser::shell_parser(&line).map_err(|e| ShellError::new(format!("{:?}", e)))?;

                let parsed = result.1;

                rl.add_history_entry(line.as_ref());

                if parsed.len() > 1 {
                    println!("Piping is not yet implemented");
                }

                println!("DEBUG: {:?}", parsed);

                let command = &parsed[0][0].name();

                match commands.get_mut(*command) {
                    Some(command) => {
                        let result = command.run(&mut host, &mut env).unwrap();
                        let view = result.to_generic_view();
                        let rendered = view.render_view(&mut host);

                        for line in rendered {
                            match line.as_ref() {
                                "\n" => println!(""),
                                line => println!("{}", line),
                            }
                        }
                    }

                    _ => println!("Saw: {}", line),
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
