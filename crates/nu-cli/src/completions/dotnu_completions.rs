use crate::completions::{file_path_completion, Completer, CompletionOptions};
use nu_protocol::{
    engine::{Stack, StateWorkingSet},
    Span,
};
use reedline::Suggestion;
use std::path::{is_separator, Path, MAIN_SEPARATOR as SEP, MAIN_SEPARATOR_STR};

use super::SemanticSuggestion;

#[derive(Clone, Default)]
pub struct DotNuCompletion {}

impl DotNuCompletion {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Completer for DotNuCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        prefix: &[u8],
        span: Span,
        offset: usize,
        _pos: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let prefix_str = String::from_utf8_lossy(prefix).replace('`', "");
        let mut search_dirs: Vec<String> = vec![];

        // If prefix_str is only a word we want to search in the current dir
        let (base, partial) = prefix_str
            .rsplit_once(is_separator)
            .unwrap_or((".", &prefix_str));
        let base_dir = base.replace(is_separator, MAIN_SEPARATOR_STR);
        let mut partial = partial.to_string();
        // On windows, this standardizes paths to use \
        let mut is_current_folder = false;

        // Fetch the lib dirs
        let lib_dirs: Vec<String> = if let Some(lib_dirs) = working_set.get_env_var("NU_LIB_DIRS") {
            lib_dirs
                .as_list()
                .into_iter()
                .flat_map(|it| {
                    it.iter().map(|x| {
                        x.to_path()
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
        // rsplit_once removes the separator
        if base_dir != "." {
            // Add the base dir into the directories to be searched
            search_dirs.push(base_dir.clone());

            // Reset the partial adding the basic dir back
            // in order to make the span replace work properly
            let mut base_dir_partial = base_dir;
            base_dir_partial.push_str(&partial);

            partial = base_dir_partial;
        } else {
            // Fetch the current folder
            #[allow(deprecated)]
            let current_folder = working_set.permanent_state.current_work_dir();
            is_current_folder = true;

            // Add the current folder and the lib dirs into the
            // directories to be searched
            search_dirs.push(current_folder);
            search_dirs.extend(lib_dirs);
        }

        // Fetch the files filtering the ones that ends with .nu
        // and transform them into suggestions

        let completions = file_path_completion(
            span,
            &partial,
            &search_dirs.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
            options,
            working_set.permanent_state,
            stack,
        );
        completions
            .into_iter()
            .filter(move |it| {
                // Different base dir, so we list the .nu files or folders
                if !is_current_folder {
                    it.path.ends_with(".nu") || it.path.ends_with(SEP)
                } else {
                    // Lib dirs, so we filter only the .nu files or directory modules
                    if it.path.ends_with(SEP) {
                        Path::new(&it.cwd).join(&it.path).join("mod.nu").exists()
                    } else {
                        it.path.ends_with(".nu")
                    }
                }
            })
            .map(move |x| SemanticSuggestion {
                suggestion: Suggestion {
                    value: x.path,
                    style: x.style,
                    span: reedline::Span {
                        start: x.span.start - offset,
                        end: x.span.end - offset,
                    },
                    append_whitespace: true,
                    ..Suggestion::default()
                },
                // TODO????
                kind: None,
            })
            .collect::<Vec<_>>()
    }
}
