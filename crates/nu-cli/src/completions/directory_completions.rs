use crate::completions::{matches, Completer, CompletionOptions};
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    levenshtein_distance, Span,
};
use reedline::Suggestion;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use super::SortBy;

const SEP: char = std::path::MAIN_SEPARATOR;

#[derive(Clone)]
pub struct DirectoryCompletion {
    engine_state: Arc<EngineState>,
}

impl DirectoryCompletion {
    pub fn new(engine_state: Arc<EngineState>) -> Self {
        Self { engine_state }
    }
}

impl Completer for DirectoryCompletion {
    fn fetch(
        &mut self,
        _: &StateWorkingSet,
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        _: usize,
        options: &CompletionOptions,
    ) -> Vec<Suggestion> {
        let partial = String::from_utf8_lossy(&prefix).to_string();

        // Filter only the folders
        let output: Vec<_> = directory_completion(
            span,
            &partial,
            &self.engine_state.current_work_dir(),
            options,
        )
        .into_iter()
        .map(move |x| Suggestion {
            value: x.1,
            description: None,
            extra: None,
            span: reedline::Span {
                start: x.0.start - offset,
                end: x.0.end - offset,
            },
            append_whitespace: false,
        })
        .collect();

        output
    }

    // Sort results prioritizing the non hidden folders
    fn sort(&self, items: Vec<Suggestion>, prefix: Vec<u8>) -> Vec<Suggestion> {
        let prefix_str = String::from_utf8_lossy(&prefix).to_string();

        // Sort items
        let mut sorted_items = items;

        match self.get_sort_by() {
            SortBy::Ascending => {
                sorted_items.sort_by(|a, b| {
                    // Ignore trailing slashes in folder names when sorting
                    a.value
                        .trim_end_matches(SEP)
                        .cmp(b.value.trim_end_matches(SEP))
                });
            }
            SortBy::LevenshteinDistance => {
                sorted_items.sort_by(|a, b| {
                    let a_distance = levenshtein_distance(&prefix_str, &a.value);
                    let b_distance = levenshtein_distance(&prefix_str, &b.value);
                    a_distance.cmp(&b_distance)
                });
            }
            _ => (),
        }

        // Separate the results between hidden and non hidden
        let mut hidden: Vec<Suggestion> = vec![];
        let mut non_hidden: Vec<Suggestion> = vec![];

        for item in sorted_items.into_iter() {
            let item_path = Path::new(&item.value);

            if let Some(value) = item_path.file_name() {
                if let Some(value) = value.to_str() {
                    if value.starts_with('.') {
                        hidden.push(item);
                    } else {
                        non_hidden.push(item);
                    }
                }
            }
        }

        // Append the hidden folders to the non hidden vec to avoid creating a new vec
        non_hidden.append(&mut hidden);

        non_hidden
    }
}

pub fn plain_listdir(source: &str) -> Vec<String> {
    let mut completions = vec![];

    if let Ok(result) = Path::new(source).read_dir() {
        for entry in result.filter_map(|e| e.ok()) {
            let mut path = entry.path().to_string_lossy().into_owned();
            if entry.path().is_dir() {
                path.push(SEP);
                completions.push(escape_path(path));
            }
        }
    }
    completions
}

pub fn directory_completion(
    span: nu_protocol::Span,
    partial: &str,
    cwd: &str,
    options: &CompletionOptions,
) -> Vec<(nu_protocol::Span, String)> {
    let original_path = Path::new(partial);

    if partial.ends_with(SEP) && Path::new(partial).exists() {
        plain_listdir(partial)
    } else {
        complete_rec(
            &original_path.to_string_lossy(),
            Path::new(cwd),
            cwd,
            options,
        )
    }
    .into_iter()
    .map(|f| (span, f))
    .collect()
}

fn escape_path(path: String) -> String {
    if path.contains(['\'', '"', ' ', '#']) {
        format!("`{path}`")
    } else {
        path
    }
}

fn complete_rec(
    partial: &str,
    cwd: &Path,
    original_cwd: &str,
    options: &CompletionOptions,
) -> Vec<String> {
    let (base, trail) = match partial.split_once(SEP) {
        Some((base, trail)) => (base, trail),
        None => (partial, ""),
    };

    let mut completions = vec![];

    if let Ok(result) = cwd.read_dir() {
        for entry in result
            .filter_map(|e| e.ok())
            .filter(|e| fs::metadata(e.path()).map(|e| e.is_dir()).unwrap_or(false))
        {
            let entry_name = entry.file_name().to_string_lossy().into_owned();
            if matches(base, &entry_name, options) && entry.path().is_dir() {
                if trail.is_empty() {
                    let path = entry.path();
                    let mut path_string = pathdiff::diff_paths(path.clone(), original_cwd)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .into_owned();
                    path_string.push(SEP);
                    completions.push(escape_path(path_string));
                } else {
                    completions.extend(complete_rec(trail, &entry.path(), original_cwd, options));
                }
            }
        }
    }
    completions
}
