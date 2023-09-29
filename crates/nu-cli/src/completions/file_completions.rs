use crate::completions::{Completer, CompletionOptions};
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    levenshtein_distance, Span,
};
use reedline::Suggestion;
use std::path::{is_separator, Path};
use std::sync::Arc;

use super::SortBy;

const SEP: char = std::path::MAIN_SEPARATOR;

#[derive(Clone)]
pub struct FileCompletion {
    engine_state: Arc<EngineState>,
}

impl FileCompletion {
    pub fn new(engine_state: Arc<EngineState>) -> Self {
        Self { engine_state }
    }
}

impl Completer for FileCompletion {
    fn fetch(
        &mut self,
        _: &StateWorkingSet,
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        _: usize,
        options: &CompletionOptions,
    ) -> Vec<Suggestion> {
        let cwd = self.engine_state.current_work_dir();
        let prefix = String::from_utf8_lossy(&prefix).to_string();
        let output: Vec<_> = file_path_completion(span, &prefix, &cwd, options)
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

pub fn partial_from(input: &str) -> (String, String) {
    let partial = input.replace('`', "");

    // If partial is only a word we want to search in the current dir
    let (base, rest) = partial.rsplit_once(is_separator).unwrap_or((".", &partial));
    // On windows, this standardizes paths to use \
    let mut base = base.replace(is_separator, &SEP.to_string());

    // rsplit_once removes the separator
    base.push(SEP);

    (base.to_string(), rest.to_string())
}

// Fix files or folders with quotes or hashes
fn escape_path(path: String) -> String {
    if path.contains('\'')
        || path.contains('"')
        || path.contains(' ')
        || path.contains('#')
        || path.contains('(')
        || path.contains(')')
        || path.starts_with('0')
        || path.starts_with('1')
        || path.starts_with('2')
        || path.starts_with('3')
        || path.starts_with('4')
        || path.starts_with('5')
        || path.starts_with('6')
        || path.starts_with('7')
        || path.starts_with('8')
        || path.starts_with('9')
    {
        return format!("`{path}`");
    }
    path
}

fn complete_rec(partial: &str, cwd: &str, options: &CompletionOptions) -> Vec<String> {
    let (base, trail) = match partial.split_once(SEP) {
        Some((base, trail)) => (base, trail),
        None => (partial, ""),
    };

    let mut completions = vec![];

    let here = Path::new(cwd);
    if let Ok(result) = here.read_dir() {
        for entry in
            result.filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        {
            if matches(base, &entry, options) {
                if trail.is_empty() {
                    let path = format!("{}{}{}", cwd, SEP, entry);
                    completions.push(escape_path(path));
                } else {
                    completions.extend(complete_rec(
                        trail,
                        &here.join(entry).to_string_lossy(),
                        options,
                    ));
                }
            }
        }
        if completions.is_empty() && trail.is_empty() {
            let path = format!("{}{}{}", cwd, SEP, base);
            completions.push(escape_path(path));
        }
    }
    completions
}

pub fn file_path_completion(
    span: nu_protocol::Span,
    partial: &str,
    _cwd: &str,
    options: &CompletionOptions,
) -> Vec<(nu_protocol::Span, String)> {
    let mut original_path = Path::new(partial);
    let mut cwd = ".";
    if original_path.is_relative() {
        if original_path.starts_with("..") {
            original_path = original_path.strip_prefix("..").unwrap_or(original_path);
            cwd = "..";
        }

        if original_path.starts_with("..") {
            original_path = original_path.strip_prefix("..").unwrap_or(original_path);
        }
    } else {
        original_path = original_path.strip_prefix("/").unwrap_or(original_path);
        cwd = "/";
    }

    let plain_completions = complete_rec(&original_path.to_string_lossy(), cwd, options);

    plain_completions.into_iter().map(|f| (span, f)).collect()
}

pub fn matches(partial: &str, from: &str, options: &CompletionOptions) -> bool {
    // Check for case sensitive
    if !options.case_sensitive {
        return options
            .match_algorithm
            .matches_str(&from.to_ascii_lowercase(), &partial.to_ascii_lowercase());
    }

    options.match_algorithm.matches_str(from, partial)
}

/// Returns whether the base_dir should be prepended to the file path
pub fn prepend_base_dir(input: &str, base_dir: &str) -> bool {
    if base_dir == format!(".{SEP}") {
        // if the current base_dir path is the local folder we only add a "./" prefix if the user
        // input already includes a local folder prefix.
        let manually_entered = {
            let mut chars = input.chars();
            let first_char = chars.next();
            let second_char = chars.next();

            first_char == Some('.') && second_char.map(is_separator).unwrap_or(false)
        };

        manually_entered
    } else {
        // always prepend the base dir if it is a subfolder
        true
    }
}
