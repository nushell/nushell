use std::path::PathBuf;

use crate::completion::{Context, Suggestion};

const SEP: char = std::path::MAIN_SEPARATOR;

pub struct Completer;

impl Completer {
    pub fn complete(&self, _ctx: &Context<'_>, partial: &str) -> Vec<Suggestion> {
        let expanded = nu_parser::expand_ndots(partial);
        let expanded = expanded.as_ref();

        let (base_dir_name, partial) = match expanded.rfind(SEP) {
            Some(pos) => expanded.split_at(pos + SEP.len_utf8()),
            None => ("", expanded),
        };

        let base_dir = if base_dir_name == "" {
            PathBuf::from(".")
        } else if base_dir_name == format!("~{}", SEP) {
            #[cfg(feature = "directories")]
            {
                dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
            }
            #[cfg(not(feature = "directories"))]
            {
                PathBuf::from("~")
            }
        } else {
            PathBuf::from(base_dir_name)
        };

        if let Ok(result) = base_dir.read_dir() {
            result
                .filter_map(|entry| {
                    entry.ok().and_then(|entry| {
                        let mut file_name = entry.file_name().to_string_lossy().into_owned();
                        if file_name.starts_with(partial) {
                            let mut path = format!("{}{}", base_dir_name, file_name);
                            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                                path.push(std::path::MAIN_SEPARATOR);
                                file_name.push(std::path::MAIN_SEPARATOR);
                            }

                            Some(Suggestion {
                                replacement: path,
                                display: file_name,
                            })
                        } else {
                            None
                        }
                    })
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}
