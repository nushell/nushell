use crate::completions::{Completer, CompletionOptions, SortBy};
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    Span,
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
    ) -> (Vec<Suggestion>, CompletionOptions) {
        let cwd = if let Some(d) = self.engine_state.env_vars.get("PWD") {
            match d.as_string() {
                Ok(s) => s,
                Err(_) => "".to_string(),
            }
        } else {
            "".to_string()
        };
        let prefix = String::from_utf8_lossy(&prefix).to_string();
        let output: Vec<_> = file_path_completion(span, &prefix, &cwd)
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

        // Options
        let options = CompletionOptions::new(true, true, SortBy::LevenshteinDistance);

        (output, options)
    }
}

pub fn file_path_completion(
    span: nu_protocol::Span,
    partial: &str,
    cwd: &str,
) -> Vec<(nu_protocol::Span, String)> {
    let partial = partial.replace('\'', "");

    let (base_dir_name, partial) = {
        // If partial is only a word we want to search in the current dir
        let (base, rest) = partial.rsplit_once(is_separator).unwrap_or((".", &partial));
        // On windows, this standardizes paths to use \
        let mut base = base.replace(is_separator, &SEP.to_string());

        // rsplit_once removes the separator
        base.push(SEP);
        (base, rest)
    };

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
                    if matches(partial, &file_name) {
                        let mut path = format!("{}{}", base_dir_name, file_name);
                        if entry.path().is_dir() {
                            path.push(SEP);
                            file_name.push(SEP);
                        }

                        if path.contains(' ') {
                            path = format!("\'{}\'", path);
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

pub fn matches(partial: &str, from: &str) -> bool {
    from.to_ascii_lowercase()
        .starts_with(&partial.to_ascii_lowercase())
}
