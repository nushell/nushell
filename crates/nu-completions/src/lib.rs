use derive_new::new;
#[cfg(windows)]
use ichwh::IchwhError;
use ichwh::IchwhResult;
use indexmap::set::IndexSet;
use rustyline::completion::{Completer, FilenameCompleter};
use std::fs::{read_dir, DirEntry};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use nu_parser::SignatureRegistry;

#[derive(new)]
pub struct NuCompleter<R: SignatureRegistry + Send> {
    pub file_completer: FilenameCompleter,
    pub commands: R,
    pub homedir: Option<PathBuf>,
}

#[derive(PartialEq, Eq, Debug)]
enum ReplacementLocation {
    Command,
    Other,
}

impl<R: SignatureRegistry + Send> NuCompleter<R> {
    pub fn complete(
        &self,
        line: &str,
        pos: usize,
        context: &rustyline::Context,
    ) -> rustyline::Result<(usize, Vec<rustyline::completion::Pair>)> {
        let line_chars: Vec<_> = line[..pos].chars().collect();

        let (replace_pos, replace_loc) = self.get_replace_pos(line, pos);

        let mut completions;

        // See if we're a flag
        if pos > 0 && replace_pos < line_chars.len() && line_chars[replace_pos] == '-' {
            if let Ok(lite_block) = nu_parser::lite_parse(line, 0) {
                completions =
                    self.get_matching_arguments(&lite_block, &line_chars, line, replace_pos, pos);
            } else {
                completions = self.file_completer.complete(line, pos, context)?.1;
            }
        } else {
            completions = self.file_completer.complete(line, pos, context)?.1;

            for completion in &mut completions {
                if completion.replacement.contains("\\ ") {
                    completion.replacement = completion.replacement.replace("\\ ", " ");
                }
                if completion.replacement.contains("\\(") {
                    completion.replacement = completion.replacement.replace("\\(", "(");
                }

                if completion.replacement.contains(' ') || completion.replacement.contains('(') {
                    if !completion.replacement.starts_with('\"') {
                        completion.replacement = format!("\"{}", completion.replacement);
                    }
                    if !completion.replacement.ends_with('\"') {
                        completion.replacement = format!("{}\"", completion.replacement);
                    }
                }
            }
        };

        // Only complete executables or commands if the thing we're completing
        // is syntactically a command
        if replace_loc == ReplacementLocation::Command {
            let all_executables = self
                .commands
                .names()
                .into_iter()
                .chain(self.find_path_executables().into_iter());

            for exe in all_executables {
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

        for completion in &mut completions {
            // If the cursor is at a double-quote, remove the double-quote in the replacement
            // This prevents duplicate quotes
            let cursor_char = line.chars().nth(pos);
            if cursor_char.unwrap_or(' ') == '"' && completion.replacement.ends_with('"') {
                completion.replacement.pop();
            }
        }

        Ok((replace_pos, completions))
    }

    fn get_replace_pos(&self, line: &str, pos: usize) -> (usize, ReplacementLocation) {
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

    fn get_matching_arguments(
        &self,
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

        let result = nu_parser::classify_block(&lite_block, &self.commands);

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
    fn pathext(&self) -> IchwhResult<Vec<String>> {
        Ok(std::env::var_os("PATHEXT")
            .ok_or(IchwhError::PathextNotDefined)?
            .to_string_lossy()
            .split(';')
            // Cut off the leading '.' character
            .map(|ext| ext[1..].to_string())
            .collect::<Vec<_>>())
    }

    #[cfg(windows)]
    fn is_executable(&self, file: &DirEntry) -> IchwhResult<bool> {
        let file_type = file.metadata()?.file_type();

        // If the entry isn't a file, it cannot be executable
        if !(file_type.is_file() || file_type.is_symlink()) {
            return Ok(false);
        }

        if let Some(extension) = file.path().extension() {
            let exts = self.pathext()?;

            Ok(exts
                .iter()
                .any(|ext| extension.to_string_lossy().eq_ignore_ascii_case(ext)))
        } else {
            Ok(false)
        }
    }

    #[cfg(unix)]
    fn is_executable(&self, file: &DirEntry) -> IchwhResult<bool> {
        let metadata = file.metadata()?;

        let filetype = metadata.file_type();
        let permissions = metadata.permissions();

        // The file is executable if it is a directory or a symlink and the permissions are set for
        // owner, group, or other
        Ok((filetype.is_file() || filetype.is_symlink()) && (permissions.mode() & 0o111 != 0))
    }

    fn find_path_executables(&self) -> IndexSet<String> {
        if let Some(path_var) = std::env::var_os("PATH") {
            let paths: Vec<_> = std::env::split_paths(&path_var).collect();

            let mut executables = IndexSet::new();
            for path in paths {
                if let Ok(mut contents) = read_dir(path) {
                    while let Some(Ok(item)) = contents.next() {
                        if let Ok(true) = self.is_executable(&item) {
                            if let Ok(name) = item.file_name().into_string() {
                                executables.insert(name);
                            }
                        }
                    }
                }
            }

            executables
        } else {
            IndexSet::new()
        }
    }
}
