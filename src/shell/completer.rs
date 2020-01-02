use crate::context::CommandRegistry;

use derive_new::new;
use nu_source::{HasSpan, Text};
use rustyline::completion::{Completer, FilenameCompleter};

#[derive(new)]
pub(crate) struct NuCompleter {
    pub file_completer: FilenameCompleter,
    pub commands: CommandRegistry,
}

impl NuCompleter {
    pub fn complete(
        &self,
        line: &str,
        pos: usize,
        context: &rustyline::Context,
    ) -> rustyline::Result<(usize, Vec<rustyline::completion::Pair>)> {
        let commands: Vec<String> = self.commands.names().unwrap_or_else(|_| vec![]);

        let line_chars: Vec<_> = line[..pos].chars().collect();

        let mut replace_pos = line_chars.len();
        while replace_pos > 0 {
            if line_chars[replace_pos - 1] == ' ' {
                break;
            }
            replace_pos -= 1;
        }

        let mut completions;

        // See if we're a flag
        if pos > 0 && replace_pos < line_chars.len() && line_chars[replace_pos] == '-' {
            completions = self.get_matching_arguments(&line_chars, line, replace_pos, pos);
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
            let mut matched = true;
            if pos < line_chars.len() {
                for chr in command.chars() {
                    if line_chars[pos] != chr {
                        matched = false;
                        break;
                    }
                    pos += 1;
                    if pos == line_chars.len() {
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

        Ok((replace_pos, completions))
    }

    fn get_matching_arguments(
        &self,
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

        if let Ok(val) = nu_parser::parse(&line_copy) {
            let source = Text::from(line);
            let pipeline_list = vec![val.clone()];
            let mut iterator =
                nu_parser::TokensIterator::all(&pipeline_list, source.clone(), val.span());

            let expand_context = nu_parser::ExpandContext {
                homedir: None,
                registry: Box::new(self.commands.clone()),
                source: &source,
            };

            let result =
                nu_parser::expand_syntax(&nu_parser::PipelineShape, &mut iterator, &expand_context);

            if let Ok(result) = result {
                for command in result.commands.list {
                    if let nu_parser::ClassifiedCommand::Internal(nu_parser::InternalCommand {
                        args,
                        ..
                    }) = command
                    {
                        if replace_pos >= args.span.start() && replace_pos <= args.span.end() {
                            if let Some(named) = args.named {
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
        }

        matching_arguments
    }
}
