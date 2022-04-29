use crate::completions::{Completer, CompletionOptions, MatchAlgorithm};
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
        let output: Vec<_> = file_path_completion(span, &prefix, &cwd, options.match_algorithm)
            .into_iter()
            .map(move |x| Suggestion {
                value: x.1,
                description: None,
                extra: None,
                span: reedline::Span {
                    start: x.0.start - offset,
                    end: x.0.end - offset,
                },
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

pub fn partial_from(input: &str) -> (String, String) {
    let partial = input.replace('\'', "");

    // If partial is only a word we want to search in the current dir
    let (base, rest) = partial.rsplit_once(is_separator).unwrap_or((".", &partial));
    // On windows, this standardizes paths to use \
    let mut base = base.replace(is_separator, &SEP.to_string());

    // rsplit_once removes the separator
    base.push(SEP);

    (base.to_string(), rest.to_string())
}

pub fn file_path_completion(
    span: nu_protocol::Span,
    partial: &str,
    cwd: &str,
    match_algorithm: MatchAlgorithm,
) -> Vec<(nu_protocol::Span, String)> {
    let (base_dir_name, partial) = partial_from(partial);

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
                    if matches(&partial, &file_name, match_algorithm) {
                        let mut path = format!("{}{}", base_dir_name, file_name);
                        if entry.path().is_dir() {
                            path.push(SEP);
                            file_name.push(SEP);
                        }

                        // Escape path string if necessary
                        path = escape_path_str(path);

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

pub fn matches(partial: &str, from: &str, match_algorithm: MatchAlgorithm) -> bool {
    match_algorithm.matches_str(&from.to_ascii_lowercase(), &partial.to_ascii_lowercase())
}

// escape paths that contains some special characters
pub fn escape_path_str(path: String) -> String {
    let mut path = path;

    // List of special characters that need to be escaped
    let special_characters = b"\\\'\"";
    let replacements = [b"\\\\", b"\\\'", b"\\\""];

    // Check if path needs to be escaped
    let needs_escape = path.bytes().fold(false, |acc, x| {
        acc
        || x == b'\\' // 0x5c
        || x == b'`' // 0x60
        || x == b'"'
        || x == b' '
        || x == b'\''
    });

    if needs_escape {
        let mut result: Vec<u8> = vec![b'\"'];

        // Walk through the path characters
        for b in path.bytes() {
            // Basically the equivalent of str.find(), but expanded
            if let Some(idx) = special_characters.iter().enumerate().fold(None, |idx, c| {
                if *c.1 == b {
                    Some(c.0)
                } else {
                    idx
                }
            }) {
                for rb in replacements[idx] {
                    result.push(*rb);
                }
            } else {
                result.push(b);
            }
        }

        // Final quote
        result.push(b'\"');

        // Update path
        path = String::from_utf8(result).unwrap_or(path);
    }

    path
}

mod test {
    #[test]
    fn escape_path() {
        // Vec of (path, expected escape)
        let cases: Vec<(&str, &str)> = vec![
            ("/home/nushell/filewith`", "\"/home/nushell/filewith`\""),
            (
                "/home/nushell/folder with spaces",
                "\"/home/nushell/folder with spaces\"",
            ),
            (
                "/home/nushell/folder\"withquotes",
                "\"/home/nushell/folder\\\"withquotes\"",
            ),
            (
                "C:\\windows\\system32\\escape path",
                "\"C:\\\\windows\\\\system32\\\\escape path\"",
            ),
            (
                "/home/nushell/shouldnt/be/escaped",
                "/home/nushell/shouldnt/be/escaped",
            ),
        ];

        for item in cases.into_iter() {
            assert_eq!(
                crate::completions::escape_path_str(item.0.to_string()),
                item.1.to_string()
            )
        }
    }
}
