use crate::context::CommandRegistry;

use derive_new::new;
use rustyline::completion::{Completer, FilenameCompleter};
use std::path::PathBuf;

#[derive(new)]
pub(crate) struct NuCompleter {
    pub file_completer: FilenameCompleter,
    pub commands: CommandRegistry,
    pub homedir: Option<PathBuf>,
}

impl NuCompleter {
    pub fn complete(
        &self,
        line: &str,
        pos: usize,
        context: &rustyline::Context,
    ) -> rustyline::Result<(usize, Vec<rustyline::completion::Pair>)> {
        let commands: Vec<String> = self.commands.names();

        let line_chars: Vec<_> = line[..pos].chars().collect();

        let mut replace_pos = line_chars.len();

        let mut parsed_pos = false;
        if let Ok(lite_block) = nu_parser::lite_parse(line, 0) {
            'outer: for pipeline in lite_block.block.iter() {
                for command in pipeline.commands.iter() {
                    let name_span = command.name.span;
                    if name_span.start() <= pos && name_span.end() >= pos {
                        replace_pos = name_span.start();
                        parsed_pos = true;
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

        for command in commands.iter() {
            let mut pos = replace_pos;
            let mut matched = false;
            if pos < line_chars.len() {
                for chr in command.chars() {
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
                    display: command.clone(),
                    replacement: command.clone(),
                });
            }
        }

        for completion in &mut completions {
            // If the cursor is at a double-quote, remove the double-quote in the replacement
            // This prevents duplicate quotes
            if line.chars().nth(pos).unwrap_or(' ') == '"' && completion.replacement.ends_with('"') {
                completion.replacement.pop();
            }
        }

        Ok((replace_pos, completions))
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
}
