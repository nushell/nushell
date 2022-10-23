use std::path::{Path, PathBuf};

use nu_engine::env::current_dir_str;
use nu_path::canonicalize_with;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::ShellError;

use dialoguer::Input;
use std::error::Error;
use std::io::{BufRead, BufReader, Read};

#[derive(Default)]
pub struct FileStructure {
    pub resources: Vec<Resource>,
}

impl FileStructure {
    pub fn new() -> FileStructure {
        FileStructure { resources: vec![] }
    }

    pub fn paths_applying_with<F>(
        &mut self,
        to: F,
    ) -> Result<Vec<(PathBuf, PathBuf)>, Box<dyn std::error::Error>>
    where
        F: Fn((PathBuf, usize)) -> Result<(PathBuf, PathBuf), Box<dyn std::error::Error>>,
    {
        self.resources
            .iter()
            .map(|f| (PathBuf::from(&f.location), f.at))
            .map(to)
            .collect()
    }

    pub fn walk_decorate(
        &mut self,
        start_path: &Path,
        engine_state: &EngineState,
        stack: &Stack,
    ) -> Result<(), ShellError> {
        self.resources = Vec::<Resource>::new();
        self.build(start_path, 0, engine_state, stack)?;
        self.resources.sort();

        Ok(())
    }

    fn build(
        &mut self,
        src: &Path,
        lvl: usize,
        engine_state: &EngineState,
        stack: &Stack,
    ) -> Result<(), ShellError> {
        let source = canonicalize_with(src, current_dir_str(engine_state, stack)?)?;

        if source.is_dir() {
            for entry in std::fs::read_dir(src)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    self.build(&path, lvl + 1, engine_state, stack)?;
                }

                self.resources.push(Resource {
                    location: path.to_path_buf(),
                    at: lvl,
                });
            }
        } else {
            self.resources.push(Resource {
                location: source,
                at: lvl,
            });
        }

        Ok(())
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Resource {
    pub at: usize,
    pub location: PathBuf,
}

impl Resource {}

pub fn try_interaction(
    interactive: bool,
    prompt_msg: &str,
    file_name: &str,
) -> (Result<Option<bool>, Box<dyn Error>>, bool) {
    let interaction = if interactive {
        let prompt = format!("{} '{}'? ", prompt_msg, file_name);
        match get_interactive_confirmation(prompt) {
            Ok(i) => Ok(Some(i)),
            Err(e) => Err(e),
        }
    } else {
        Ok(None)
    };

    let confirmed = match interaction {
        Ok(maybe_input) => maybe_input.unwrap_or(false),
        Err(_) => false,
    };

    (interaction, confirmed)
}

#[allow(dead_code)]
fn get_interactive_confirmation(prompt: String) -> Result<bool, Box<dyn Error>> {
    let input = Input::new()
        .with_prompt(prompt)
        .validate_with(|c_input: &String| -> Result<(), String> {
            if c_input.len() == 1
                && (c_input == "y" || c_input == "Y" || c_input == "n" || c_input == "N")
            {
                Ok(())
            } else if c_input.len() > 1 {
                Err("Enter only one letter (Y/N)".to_string())
            } else {
                Err("Input not valid".to_string())
            }
        })
        .default("Y/N".into())
        .interact_text()?;

    if input == "y" || input == "Y" {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub struct BufferedReader<R: Read> {
    pub input: BufReader<R>,
}

impl<R: Read> BufferedReader<R> {
    pub fn new(input: BufReader<R>) -> Self {
        Self { input }
    }
}

impl<R: Read> Iterator for BufferedReader<R> {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        let buffer = self.input.fill_buf();
        match buffer {
            Ok(s) => {
                let result = s.to_vec();

                let buffer_len = s.len();

                if buffer_len == 0 {
                    None
                } else {
                    self.input.consume(buffer_len);

                    Some(Ok(result))
                }
            }
            Err(e) => Some(Err(ShellError::IOError(e.to_string()))),
        }
    }
}
