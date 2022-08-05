use crate::completions::{Completer, CompletionOptions};
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    levenshtein_distance, Span,
};
use reedline::Suggestion;
use std::path::{is_separator, Path};
use std::sync::Arc;

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
        let cwd = if let Some(d) = self.engine_state.get_env_var("PWD") {
            match d.as_string() {
                Ok(s) => s,
                Err(_) => "".to_string(),
            }
        } else {
            "".to_string()
        };
        let prefix = String::from_utf8_lossy(&prefix).to_string();
        let output: Vec<_> = file_path_completion_with_parents(span, &prefix, &cwd, options)
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
        sorted_items.sort_by(|a, b| a.value.cmp(&b.value));
        sorted_items.sort_by(|a, b| {
            let a_distance = levenshtein_distance(&prefix_str, &a.value);
            let b_distance = levenshtein_distance(&prefix_str, &b.value);
            a_distance.cmp(&b_distance)
        });

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

pub fn partial_from_with_parents(input: &str) -> Option<(String, String)> {
    let partial = input.replace('`', "");

    // If partial is only a word we want to search in the current dir
    let (base, rest) = partial.rsplit_once(is_separator).unwrap_or((".", &partial));
    // On windows, this standardizes paths to use \
    let mut base = base.replace(is_separator, &SEP.to_string());

    // rsplit_once removes the separator
    base.push(SEP);

    let num_leading_dots = rest.bytes().take_while(|c| *c == b'.').count();

    let (leading_dots, following_leading_dots) = rest.split_at(num_leading_dots);

    // only push dots from rest into base if we believe input is not a prefix to a single dot hidden folder
    if num_leading_dots > 1 {
        base.push_str(leading_dots);
        base.push(SEP);

        return Some((base.to_string(), following_leading_dots.to_string()));
    }

    None
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

pub fn generic_completion_with_parents<F>(
    span: nu_protocol::Span,
    partial_input: &str,
    cwd: &str,
    options: &CompletionOptions,
    completer: F,
) -> Vec<(nu_protocol::Span, String)>
where
    F: Fn(
        nu_protocol::Span,
        &str,
        &str,
        &str,
        &str,
        &CompletionOptions,
    ) -> Vec<(nu_protocol::Span, String)>,
{
    let original_input = partial_input;

    let (base_completion, partial) = partial_from(partial_input);

    let mut completions = completer(
        span,
        original_input,
        &base_completion,
        &partial,
        cwd,
        options,
    );

    if let Some((base_completion, partial)) = partial_from_with_parents(partial_input) {
        let mut completions_with_parents = completer(
            span,
            original_input,
            &base_completion,
            &partial,
            cwd,
            options,
        );

        completions.append(completions_with_parents.as_mut());
    }

    completions
}

pub fn file_path_completion_with_parents(
    span: nu_protocol::Span,
    partial_input: &str,
    cwd: &str,
    options: &CompletionOptions,
) -> Vec<(nu_protocol::Span, String)> {
    generic_completion_with_parents(span, partial_input, cwd, options, file_path_completion)
}

pub fn file_path_completion(
    span: nu_protocol::Span,
    original_input: &str,
    base_dir_name: &str,
    partial: &str,
    cwd: &str,
    options: &CompletionOptions,
) -> Vec<(nu_protocol::Span, String)> {
    let base_dir = nu_path::expand_path_with(&base_dir_name, cwd);
    // This check is here as base_dir.read_dir() with base_dir == "" will open the current dir
    // which we don't want in this case (if we did, base_dir would already be ".")
    if base_dir == Path::new("") {
        return Vec::new();
    }

    if let Ok(result) = base_dir.read_dir() {
        return result
            .filter_map(|entry| {
                entry.ok().and_then(|entry| {
                    let mut file_name = entry.file_name().to_string_lossy().into_owned();
                    if matches(partial, &file_name, options) {
                        let mut path = if prepend_base_dir(original_input, base_dir_name) {
                            format!("{}{}", base_dir_name, file_name)
                        } else {
                            file_name.to_string()
                        };

                        if entry.path().is_dir() {
                            path.push(SEP);
                            file_name.push(SEP);
                        }

                        // Fix files or folders with quotes
                        if path.contains('\'') || path.contains('"') || path.contains(' ') {
                            path = format!("`{}`", path);
                        }

                        Some((span, path))
                    } else {
                        None
                    }
                })
            })
            .collect();
    }

    Vec::new()
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
    if base_dir == format!(".{}", SEP) {
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
