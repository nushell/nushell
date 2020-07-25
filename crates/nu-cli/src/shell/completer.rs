use std::fs::{read_dir, DirEntry};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use indexmap::set::IndexSet;
use nu_errors::ShellError;
use rustyline::completion::{Completer as _, FilenameCompleter};
use rustyline::hint::{Hinter as _, HistoryHinter};

#[cfg(all(windows, feature = "ichwh"))]
use ichwh::{IchwhError, IchwhResult};

use crate::completion::{self, Completer};
use crate::context;
use crate::data::config;
use crate::prelude::*;

pub(crate) struct NuCompleter {
    file_completer: FilenameCompleter,
    hinter: HistoryHinter,
}

#[derive(PartialEq, Eq, Debug)]
enum ReplacementLocation {
    Command,
    Other,
}

impl NuCompleter {
    fn complete_internal(
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

        // Only complete executables or commands if the thing we're completing
        // is syntactically a command
        if replace_loc == ReplacementLocation::Command {
            let context: &context::Context = context.as_ref();
            let commands: Vec<String> = context.registry.names();
            let mut all_executables: IndexSet<_> = commands.iter().map(|x| x.to_string()).collect();

            let complete_from_path = config::config(Tag::unknown())
                .map(|conf| {
                    conf.get("complete_from_path")
                        .map(|v| v.is_true())
                        .unwrap_or(true)
                })
                .unwrap_or(true);

            if complete_from_path {
                let path_executables = find_path_executables().unwrap_or_default();
                for path_exe in path_executables {
                    all_executables.insert(path_exe);
                }
            };

            for exe in all_executables.iter() {
                let mut pos = replace_pos;
                let mut matched = false;
                if pos < line_chars.len() {
                    for chr in exe.chars() {
                        if line_chars[pos] != chr {
                            break;
                        }

                        pos += 1;
                        if pos == line_chars.len() {
                            matched = true;
                            break;
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

                            if full_flag.starts_with(&substring) {
                                matching_arguments.push(rustyline::completion::Pair {
                                    display: full_flag.clone(),
                                    replacement: full_flag,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    matching_arguments
}

// These is_executable/pathext implementations are copied from ichwh and modified
// to not be async

#[cfg(windows)]
fn pathext() -> IchwhResult<Vec<String>> {
    Ok(std::env::var_os("PATHEXT")
        .ok_or(IchwhError::PathextNotDefined)?
        .to_string_lossy()
        .split(';')
        // Cut off the leading '.' character
        .map(|ext| ext[1..].to_string())
        .collect::<Vec<_>>())
}

#[cfg(windows)]
fn is_executable(file: &DirEntry) -> bool {
    if let Ok(metadata) = file.metadata() {
        let file_type = metadata.file_type();

        // If the entry isn't a file, it cannot be executable
        if !(file_type.is_file() || file_type.is_symlink()) {
            return false;
        }

        if let Some(extension) = file.path().extension() {
            if let Ok(exts) = pathext() {
                exts.iter()
                    .any(|ext| extension.to_string_lossy().eq_ignore_ascii_case(ext))
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }
}

#[cfg(target_arch = "wasm32")]
fn is_executable(file: &DirEntry) -> bool {
    false
}

#[cfg(unix)]
fn is_executable(file: &DirEntry) -> bool {
    let metadata = file.metadata();

    if let Ok(metadata) = metadata {
        let filetype = metadata.file_type();
        let permissions = metadata.permissions();

        // The file is executable if it is a directory or a symlink and the permissions are set for
        // owner, group, or other
        (filetype.is_file() || filetype.is_symlink()) && (permissions.mode() & 0o111 != 0)
    } else {
        false
    }
}

fn find_path_executables() -> Option<IndexSet<String>> {
    let path_var = std::env::var_os("PATH")?;
    let paths: Vec<_> = std::env::split_paths(&path_var).collect();

    let mut executables: IndexSet<String> = IndexSet::new();
    for path in paths {
        if let Ok(mut contents) = read_dir(path) {
            while let Some(Ok(item)) = contents.next() {
                if is_executable(&item) {
                    if let Ok(name) = item.file_name().into_string() {
                        executables.insert(name);
                    }
                }
            }
        }
    }

    Some(executables)
}

fn get_replace_pos(line: &str, pos: usize) -> (usize, ReplacementLocation) {
    let line_chars: Vec<_> = line[..pos].chars().collect();
    let mut replace_pos = line_chars.len();
    let mut parsed_pos = false;
    let mut loc = ReplacementLocation::Other;
    if let Ok(lite_block) = nu_parser::lite_parse(line, 0) {
        'outer: for pipeline in lite_block.block.iter() {
            for command in pipeline.commands.iter() {
                let name_span = command.name.span;
                if name_span.start() <= pos && name_span.end() >= pos {
                    replace_pos = name_span.start();
                    parsed_pos = true;
                    loc = ReplacementLocation::Command;
                    break 'outer;
                }

                for arg in command.args.iter() {
                    if arg.span.start() <= pos && arg.span.end() >= pos {
                        replace_pos = arg.span.start();
                        parsed_pos = true;
                        break 'outer;
                    }
                }
            }
        }
    }

    if !parsed_pos {
        // If the command won't parse, naively detect the completion start point
        while replace_pos > 0 {
            if line_chars[replace_pos - 1] == ' ' {
                break;
            }
            replace_pos -= 1;
        }
    }

    (replace_pos, loc)
}

fn requote(
    items: (usize, Vec<rustyline::completion::Pair>),
) -> (usize, Vec<rustyline::completion::Pair>) {
    let mut new_items = Vec::with_capacity(items.1.len());

    for item in items.1 {
        let unescaped = rustyline::completion::unescape(&item.replacement, Some('\\'));
        let maybe_quote = if unescaped != item.replacement {
            "\""
        } else {
            ""
        };

        new_items.push(rustyline::completion::Pair {
            display: item.display,
            replacement: format!("{}{}{}", maybe_quote, unescaped, maybe_quote),
        });
    }

    (items.0, new_items)
}
