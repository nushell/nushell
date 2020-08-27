use std::fs::{read_dir, metadata, DirEntry};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use indexmap::set::IndexSet;
use nu_errors::ShellError;
use rustyline::completion::{Completer as _, FilenameCompleter};
use rustyline::hint::{Hinter as _, HistoryHinter};

#[cfg(all(windows, feature = "ichwh"))]
use ichwh::{IchwhError, IchwhResult};

use crate::completion::{self, Completer, Suggestion};
use crate::context;

pub(crate) struct NuCompleter {}

impl NuCompleter {}

impl NuCompleter {
    pub fn complete(
        &self,
        line: &str,
        pos: usize,
        context: &completion::Context,
    ) -> rustyline::Result<(usize, Vec<rustyline::completion::Pair>)> {
        let line_chars: Vec<_> = line[..pos].chars().collect();

        let (replace_pos, replace_loc) = get_replace_pos(line, pos);

        // See if we're a flag
        let mut completions;
        if pos > 0 && replace_pos < line_chars.len() && line_chars[replace_pos] == '-' {
            if let Ok(lite_block) = nu_parser::lite_parse(line, 0) {
                completions = get_matching_arguments(
                    context.as_ref(),
                    &lite_block,
                    &line_chars,
                    line,
                    replace_pos,
                    pos,
                );
            } else {
                completions = self.file_completer.complete(line, pos, context.as_ref())?.1;
            }
        } else {
            completions = self.file_completer.complete(line, pos, context.as_ref())?.1;
        }
        
        if line[..2] == "cd".to_string(){
            completions = autocomplete_only_folders(completions);
        }

        // Only complete executables or commands if the thing we're completing
        // is syntactically a command
        if replace_loc == ReplacementLocation::Command {
            let context: &context::Context = context.as_ref();
            let commands: Vec<String> = context.registry.names();
            let mut all_executables: IndexSet<_> = commands.iter().map(|x| x.to_string()).collect();
    ) -> (usize, Vec<Suggestion>) {
        use crate::completion::engine::LocationType;

        let nu_context: &context::Context = context.as_ref();
        let lite_block = match nu_parser::lite_parse(line, 0) {
            Ok(block) => Some(block),
            Err(result) => result.partial,
        };

        let locations = lite_block
            .map(|block| nu_parser::classify_block(&block, &nu_context.registry))
            .map(|block| crate::completion::engine::completion_location(line, &block.block, pos))
            .unwrap_or_default();

        if locations.is_empty() {
            (pos, Vec::new())
        } else {
            let pos = locations[0].span.start();
            let suggestions = locations
                .into_iter()
                .flat_map(|location| {
                    let partial = location.span.slice(line);
                    match location.item {
                        LocationType::Command => {
                            let command_completer = crate::completion::command::Completer {};
                            command_completer.complete(context, partial)
                        }

                        LocationType::Flag(cmd) => {
                            let flag_completer = crate::completion::flag::Completer {};
                            flag_completer.complete(context, cmd, partial)
                        }
                    }
                }

                if matched {
                    completions.push(rustyline::completion::Pair {
                        display: exe.to_string(),
                        replacement: exe.to_string(),
                    });
                }
            }
        }

        // Adjust replacement to deal with a quote already at the cursor. Specifically, if there's
        // already a quote at the cursor, but the replacement doesn't have one, we need to ensure
        // one exists (to be safe, even if the completion doesn't need it).
        for completion in &mut completions {
            let cursor_char = line.chars().nth(replace_pos);
            if cursor_char.unwrap_or(' ') == '"' && !completion.replacement.starts_with('"') {
                completion.replacement.insert(0, '"');
            }
        }

        Ok((replace_pos, completions))
    }
}

impl Completer for NuCompleter {
    fn complete(
        &self,
        line: &str,
        pos: usize,
        context: &completion::Context,
    ) -> Result<(usize, Vec<completion::Suggestion>), ShellError> {
        let expanded = nu_parser::expand_ndots(&line);

        // Find the first not-matching char position, if there is one
        let differ_pos = line
            .chars()
            .zip(expanded.chars())
            .enumerate()
            .find(|(_index, (a, b))| a != b)
            .map(|(differ_pos, _)| differ_pos);

        let pos = if let Some(differ_pos) = differ_pos {
            if differ_pos < pos {
                pos + (expanded.len() - line.len())
            } else {
                pos
            }
        } else {
            pos
        };

        self.complete_internal(&expanded, pos, context)
            .map_err(|e| ShellError::untagged_runtime_error(format!("{}", e)))
            .map(requote)
            .map(|(pos, completions)| {
                (
                    pos,
                    completions
                        .into_iter()
                        .map(|pair| completion::Suggestion {
                            display: pair.display,
                            replacement: pair.replacement,
                        })
                        .collect(),
                )
            })
    }

    fn hint(&self, line: &str, pos: usize, ctx: &completion::Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, &ctx.as_ref())
    }
}

impl Default for NuCompleter {
    fn default() -> NuCompleter {
        NuCompleter {
            file_completer: FilenameCompleter::new(),
            hinter: HistoryHinter {},
        }
    }
}

fn autocomplete_only_folders(
    completions: Vec<rustyline::completion::Pair>,
) -> Vec<rustyline::completion::Pair> {
    let mut result = Vec::new();
    for completion in completions {
        let filepath = completion.replacement.clone();
        let md = metadata(filepath).unwrap();
        if md.is_dir() {
            result.push(completion);
        }
    } 
    return result;
}

fn get_matching_arguments(
    context: &context::Context,
    lite_block: &nu_parser::LiteBlock,
    line_chars: &[char],
    line: &str,
    replace_pos: usize,
    pos: usize,
) -> Vec<rustyline::completion::Pair> {
    let mut matching_arguments = vec![];

    let mut line_copy = line.to_string();
    let substring = line_chars[replace_pos..pos].iter().collect::<String>();
    let replace_string = (replace_pos..pos).map(|_| " ").collect::<String>();
    line_copy.replace_range(replace_pos..pos, &replace_string);

    let result = nu_parser::classify_block(&lite_block, &context.registry);

    for pipeline in &result.block.block {
        for command in &pipeline.list {
            if let nu_protocol::hir::ClassifiedCommand::Internal(
                nu_protocol::hir::InternalCommand { args, .. },
            ) = command
            {
                if replace_pos >= args.span.start() && replace_pos <= args.span.end() {
                    if let Some(named) = &args.named {
                        for (name, _) in named.iter() {
                            let full_flag = format!("--{}", name);

                        LocationType::Argument(_cmd, _arg_name) => {
                            // TODO use cmd and arg_name to narrow things down further
                            let path_completer = crate::completion::path::Completer::new();
                            path_completer.complete(context, partial)
                        }

                        LocationType::Variable => Vec::new(),
                    }
                    .into_iter()
                    .map(requote)
                })
                .collect();

            (pos, suggestions)
        }
    }
}

fn requote(item: Suggestion) -> Suggestion {
    let unescaped = rustyline::completion::unescape(&item.replacement, Some('\\'));
    if unescaped != item.replacement {
        Suggestion {
            display: item.display,
            replacement: format!("\"{}\"", unescaped),
        }
    } else {
        item
    }
}
