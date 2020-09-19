use std::path::PathBuf;

use super::matchers::Matcher;
use crate::completion::{Completer, CompletionContext, Suggestion};

const SEP: char = std::path::MAIN_SEPARATOR;

pub struct PathCompleter;

pub struct PathSuggestion {
    pub(crate) path: PathBuf,
    pub(crate) suggestion: Suggestion,
}

impl PathCompleter {
    pub fn path_suggestions(&self, partial: &str, matcher: &dyn Matcher) -> Vec<PathSuggestion> {
        let expanded = nu_parser::expand_ndots(partial);
        let expanded = expanded.as_ref();

        let (base_dir_name, partial) = match expanded.rfind(SEP) {
            Some(pos) => expanded.split_at(pos + SEP.len_utf8()),
            None => ("", expanded),
        };

        let base_dir = if base_dir_name == "" {
            PathBuf::from(".")
        } else {
            #[cfg(feature = "directories")]
            {
                let home_prefix = format!("~{}", SEP);
                if base_dir_name.starts_with(&home_prefix) {
                    let mut home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
                    home_dir.push(&base_dir_name[2..]);
                    home_dir
                } else {
                    PathBuf::from(base_dir_name)
                }
            }
            #[cfg(not(feature = "directories"))]
            {
                PathBuf::from(base_dir_name)
            }
        };

        if let Ok(result) = base_dir.read_dir() {
            result
                .filter_map(|entry| {
                    entry.ok().and_then(|entry| {
                        let mut file_name = entry.file_name().to_string_lossy().into_owned();
                        if matcher.matches(partial, file_name.as_str()) {
                            let mut path = format!("{}{}", base_dir_name, file_name);
                            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                                path.push(std::path::MAIN_SEPARATOR);
                                file_name.push(std::path::MAIN_SEPARATOR);
                            }

                            Some(PathSuggestion {
                                path: entry.path(),
                                suggestion: Suggestion {
                                    replacement: path,
                                    display: file_name,
                                },
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

impl Completer for PathCompleter {
    fn complete(
        &self,
        _ctx: &CompletionContext<'_>,
        partial: &str,
        matcher: &dyn Matcher,
    ) -> Vec<Suggestion> {
        self.path_suggestions(partial, matcher)
            .into_iter()
            .map(|ps| ps.suggestion)
            .collect()
    }
}
