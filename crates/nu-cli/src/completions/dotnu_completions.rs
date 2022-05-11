use crate::completions::{
    file_path_completion, partial_from, Completer, CompletionOptions, SortBy,
};
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    Span,
};
use reedline::Suggestion;
use std::sync::Arc;
const SEP: char = std::path::MAIN_SEPARATOR;

#[derive(Clone)]
pub struct DotNuCompletion {
    engine_state: Arc<EngineState>,
}

impl DotNuCompletion {
    pub fn new(engine_state: Arc<EngineState>) -> Self {
        Self { engine_state }
    }
}

impl Completer for DotNuCompletion {
    fn fetch(
        &mut self,
        _: &StateWorkingSet,
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        _: usize,
        options: &CompletionOptions,
    ) -> Vec<Suggestion> {
        let prefix_str = String::from_utf8_lossy(&prefix).to_string();
        let mut search_dirs: Vec<String> = vec![];
        let (base_dir, mut partial) = partial_from(&prefix_str);
        let mut is_current_folder = false;

        // Fetch the lib dirs
        let lib_dirs: Vec<String> =
            if let Some(lib_dirs) = self.engine_state.get_env_var("NU_LIB_DIRS") {
                lib_dirs
                    .as_list()
                    .into_iter()
                    .flat_map(|it| {
                        it.iter().map(|x| {
                            x.as_path()
                                .expect("internal error: failed to convert lib path")
                        })
                    })
                    .map(|it| {
                        it.into_os_string()
                            .into_string()
                            .expect("internal error: failed to convert OS path")
                    })
                    .collect()
            } else {
                vec![]
            };

        // Check if the base_dir is a folder
        if base_dir != format!(".{}", SEP) {
            // Add the base dir into the directories to be searched
            search_dirs.push(base_dir.clone());

            // Reset the partial adding the basic dir back
            // in order to make the span replace work properly
            let mut base_dir_partial = base_dir;
            base_dir_partial.push_str(&partial);

            partial = base_dir_partial;
        } else {
            // Fetch the current folder
            let current_folder = if let Some(d) = self.engine_state.get_env_var("PWD") {
                match d.as_string() {
                    Ok(s) => s,
                    Err(_) => "".to_string(),
                }
            } else {
                "".to_string()
            };
            is_current_folder = true;

            // Add the current folder and the lib dirs into the
            // directories to be searched
            search_dirs.push(current_folder);
            search_dirs.extend(lib_dirs);
        }

        // Fetch the files filtering the ones that ends with .nu
        // and transform them into suggestions
        let output: Vec<Suggestion> = search_dirs
            .into_iter()
            .flat_map(|it| {
                file_path_completion(span, &partial, &it, options)
                    .into_iter()
                    .filter(|it| {
                        // Different base dir, so we list the .nu files or folders
                        if !is_current_folder {
                            it.1.ends_with(".nu") || it.1.ends_with(SEP)
                        } else {
                            // Lib dirs, so we filter only the .nu files
                            it.1.ends_with(".nu")
                        }
                    })
                    .map(move |x| Suggestion {
                        value: x.1,
                        description: None,
                        extra: None,
                        span: reedline::Span {
                            start: x.0.start - offset,
                            end: x.0.end - offset,
                        },
                        append_whitespace: true,
                    })
            })
            .collect();

        output
    }

    fn get_sort_by(&self) -> SortBy {
        SortBy::LevenshteinDistance
    }
}
