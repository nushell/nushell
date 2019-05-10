#![feature(crate_visibility_modifier)]
#![feature(in_band_lifetimes)]
#![allow(unused)]

mod commands;
mod env;
mod errors;
mod format;
mod object;

crate use crate::commands::command::Command;
crate use crate::env::{Environment, Host};
crate use crate::format::RenderView;
use crate::object::base::{ToEntriesView, ToGenericView};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::collections::BTreeMap;
use std::error::Error;
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

    let mut commands = BTreeMap::<String, Box<dyn crate::Command>>::new();

    let mut system = sysinfo::System::new();
    let mut ps = crate::commands::ps::Ps::new(system);
    let mut ls = crate::commands::ls::Ls;

    commands.insert("ps".to_string(), Box::new(ps));
    commands.insert("ls".to_string(), Box::new(ls));

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_ref());

                match commands.get_mut(&line) {
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
