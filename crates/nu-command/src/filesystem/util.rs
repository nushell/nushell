use std::path::{Path, PathBuf};

use nu_path::canonicalize_with;
use nu_protocol::ShellError;

use dialoguer::Input;
use std::error::Error;

#[derive(Default)]
pub struct FileStructure {
    pub resources: Vec<Resource>,
}

#[allow(dead_code)]
impl FileStructure {
    pub fn new() -> FileStructure {
        FileStructure { resources: vec![] }
    }

    pub fn contains_more_than_one_file(&self) -> bool {
        self.resources.len() > 1
    }

    pub fn contains_files(&self) -> bool {
        !self.resources.is_empty()
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

    pub fn walk_decorate(&mut self, start_path: &Path) -> Result<(), ShellError> {
        self.resources = Vec::<Resource>::new();
        self.build(start_path, 0)?;
        self.resources.sort();

        Ok(())
    }

    fn build(&mut self, src: &Path, lvl: usize) -> Result<(), ShellError> {
        let source = canonicalize_with(src, std::env::current_dir()?)?;

        if source.is_dir() {
            for entry in std::fs::read_dir(src)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    self.build(&path, lvl + 1)?;
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

pub fn get_interactive_confirmation(prompt: String) -> Result<bool, Box<dyn Error>> {
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
