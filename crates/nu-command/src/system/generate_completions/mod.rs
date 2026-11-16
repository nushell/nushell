//! `generate completions` builds `extern` definitions, with completions, for an external command
//! from what other tools already know about it: a carapace spec, a fish or zsh completion script,
//! a man page or the command's own `--help` text.

mod carapace;
mod completers;
mod fish;
mod help_text;
mod spec;
mod zsh;

use nu_engine::{command_prelude::*, env_to_strings};
use nu_path::AbsolutePathBuf;
use nu_protocol::{engine::CommandType, shell_error::generic::GenericError};
use spec::{CommandSpec, is_command_word};
use std::{
    collections::HashMap,
    io::Read,
    process::{Command as Process, Stdio},
};

#[derive(Clone)]
pub struct GenerateCompletions;

impl Command for GenerateCompletions {
    fn name(&self) -> &str {
        "generate completions"
    }

    fn signature(&self) -> Signature {
        Signature::build("generate completions")
            .input_output_types(vec![
                (Type::Nothing, Type::String),
                (Type::String, Type::String),
            ])
            .required(
                "command",
                SyntaxShape::String,
                "The external command, or one of its subcommands such as \"git status\".",
            )
            .param(
                Flag::new("source")
                    .short('s')
                    .arg(SyntaxShape::String)
                    .desc(
                        "Where to read the flags from: carapace, fish, zsh, man, help, or all \
                         of them merged. Required when the source text is piped in.",
                    )
                    .completion(Completion::new_list(&Source::CHOICES)),
            )
            .category(Category::System)
    }

    fn description(&self) -> &str {
        "Generate `extern` definitions, with completions, for an external command from its carapace spec, fish or zsh completions, man page or --help text."
    }

    fn extra_description(&self) -> &str {
        "Without --source, the sources are tried in the order carapace, fish, zsh, man, help, and \
the first one that knows any flags or subcommands is used. A man page lists no subcommands, and \
a source may list a subcommand without any of its flags, as fish's own completions for ouch do. \
So when man is used, or a source that lists a subcommand it knows nothing else about, and the \
command's --help lists subcommands, what help finds is merged in as with --source all. With \
--source all, every source \
that knows any is used and their commands and flags are merged, the earlier source winning when \
they disagree. A flag they disagree on taking a value is declared as a switch, and the comment \
above its extern names the sources on each side. carapace, fish and zsh know the whole tree of subcommands, and man gives the \
flags of the named command only. The help source runs the command with --help, then each \
subcommand that help lists, and theirs in turn, to read the whole tree. It runs a subcommand \
that way only when another source knows it, the --help of the command above it lists it along \
with a --help option, or it has a man page, as git's subcommands do, since a command that \
doesn't read --help first would run instead. launchctl's help lists reboot, but no --help \
option, so `launchctl reboot --help` isn't run.

The result is the text of a module of `export extern` definitions. Externs exist only once \
nushell has parsed them, so save the text to a file and load it with `use` or `source`, or save \
it in one of $nu.user-autoload-dirs to load it at startup. For `use`, name the file something \
other than the command, such as git-completions.nu: `use` names the module after the file, and \
a module can't export an extern with its own name.

Flag values and arguments the source lists, such as --color's always, never and auto, become \
completions. Values that fish or zsh compute become completer commands in the module when they \
translate to nushell: a command such as `git for-each-ref ... refs/heads/`, or a shared helper \
for users, groups, hosts, processes, commands, network interfaces or make targets. A tool built \
with cobra, such as gh, docker or kubectl, completes its own arguments through its `__complete` \
command. Any other extern gets `@complete external`, so an argument none of these covers is \
completed by the external completer set in $env.config.completions.external, when there is one.

Flags with an optional value (--color[=when]) and single-dash long options (-name) can't be \
declared in a signature. They are listed in the extern's description and still reach the \
command."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "extern",
            "externs",
            "signature",
            "carapace",
            "fish",
            "zsh",
            "man",
            "help",
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let target: Spanned<String> = call.req(engine_state, stack, 0)?;
        let source: Option<Spanned<String>> = call.get_flag(engine_state, stack, "source")?;

        let words = target
            .item
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let Some((tool, path)) = words.split_first() else {
            return Err(ShellError::IncorrectValue {
                msg: "expected the name of an external command".into(),
                val_span: target.span,
                call_span: head,
            });
        };
        if let Some(word) = words.iter().find(|word| !is_command_word(word)) {
            return Err(ShellError::IncorrectValue {
                msg: format!("`{word}` can't be part of an extern's name"),
                val_span: target.span,
                call_span: head,
            });
        }
        // An extern named like a built-in or plugin command replaces it, and a keyword's name
        // can't be an extern's.
        if let Some(command_type) = engine_state
            .find_decl(tool.as_bytes(), &[])
            .map(|decl| engine_state.get_decl(decl).command_type())
            && matches!(
                command_type,
                CommandType::Keyword | CommandType::Builtin | CommandType::Plugin
            )
        {
            return Err(ShellError::IncorrectValue {
                msg: match command_type {
                    CommandType::Keyword => {
                        format!("`{tool}` is a nushell keyword, so an extern can't have its name")
                    }
                    _ => format!(
                        "`{tool}` is a nushell {command_type} command, and an extern with its name would replace it"
                    ),
                },
                val_span: target.span,
                call_span: head,
            });
        }

        // `None` tries the sources in order, `Some(None)` merges them all.
        let selection =
            source
                .map(|source| match source.item.as_str() {
                    "all" => Ok(None),
                    name => Source::from_name(name).map(Some).ok_or_else(|| {
                        ShellError::IncorrectValue {
                            msg: format!("expected one of {}", Source::CHOICES.join(", ")),
                            val_span: source.span,
                            call_span: head,
                        }
                    }),
                })
                .transpose()?;

        let piped = match input {
            PipelineData::Empty | PipelineData::Value(Value::Nothing { .. }, ..) => None,
            input => Some(input.collect_string("", &stack.get_config(engine_state))?),
        };

        // Piped text is read as the given source; otherwise each source runs its program.
        let (sources, runner) = match (&piped, selection) {
            (Some(_), Some(Some(source))) => (vec![source], None),
            (Some(_), _) => {
                return Err(ShellError::Generic(
                    GenericError::new(
                        "the source of the piped text is unknown",
                        "generate completions needs --source with one source to read piped text",
                        head,
                    )
                    .with_help(
                        "for example: ^git status -h | generate completions 'git status' --source help",
                    ),
                ));
            }
            (None, Some(Some(source))) => {
                (vec![source], Some(Runner::new(engine_state, stack, head)?))
            }
            (None, _) => (
                Source::ALL.to_vec(),
                Some(Runner::new(engine_state, stack, head)?),
            ),
        };
        let merge_all = matches!(selection, Some(None));

        let mut learned: Vec<(Vec<CommandSpec>, String)> = vec![];
        // Why the help source didn't run the subcommand, for the error.
        let mut unlisted = None;
        for source in sources {
            // In the default order, the loop only goes on past a source that knew anything to
            // the help source, when help may know subcommands that source doesn't (see
            // `leaves_subcommands_to_help`). Help then adds the subcommands its --help lists,
            // or nothing.
            let after_first = !merge_all && !learned.is_empty();
            if after_first && !matches!(source, Source::Help) {
                continue;
            }
            // `<tool> <words> --help` runs the subcommand when the tool doesn't read --help
            // first: `rm notes.txt --help` removes notes.txt. So each word must be a
            // subcommand that another source knows, that the help above it lists along with a
            // --help option, or that has a man page, as git's subcommands do.
            if let Some(runner) = &runner
                && matches!(source, Source::Help)
                && !learned
                    .iter()
                    .any(|(specs, _)| specs.iter().any(|spec| spec.path == path))
                && let Some((end, listing)) = (2..=words.len()).find_map(|end| {
                    let listing = listing_command(runner, &words[..end]);
                    (!listing.as_ref().is_some_and(CommandSpec::lists_help)
                        && !has_man_page(runner, &words[..end]))
                    .then_some((end, listing))
                })
            {
                let word = &words[end - 1];
                unlisted = Some(format!(
                    "`{} --help` {} and there's no `man {}`, so `{} --help` wasn't run. Pipe its \
                     help text in with --source help",
                    words[..end - 1].join(" "),
                    match listing {
                        Some(_) => format!("lists `{word}` but no --help option,"),
                        None => format!("doesn't list `{word}` as a subcommand"),
                    },
                    words[..end].join("-"),
                    words.join(" ")
                ));
                continue;
            }
            let (texts, origin) = match (&piped, &runner) {
                (Some(text), _) => (
                    vec![text.clone()],
                    format!("{} text piped in", source.name()),
                ),
                (None, Some(runner)) => (source.fetch(runner, &words), source.origin(&words)),
                (None, None) => continue,
            };
            engine_state.signals().check(&head)?;

            for text in texts {
                let specs = match source.parse(&text, tool, path) {
                    Ok(specs) => specs,
                    Err(err) if piped.is_some() => {
                        return Err(ShellError::Generic(GenericError::new(
                            "piped text is not a carapace export",
                            err.to_string(),
                            head,
                        )));
                    }
                    // Whatever the program printed instead of an export.
                    Err(_) => continue,
                };
                // The first spec is the command's own, and any others are its subcommands.
                if after_first && specs.len() < 2 {
                    continue;
                }
                let specs = match (source, &runner) {
                    (Source::Help, Some(runner)) => walk_help(
                        runner,
                        &words,
                        &mut vec![text],
                        specs,
                        engine_state.signals(),
                        head,
                    )?,
                    _ => specs,
                };
                if knows_anything(&specs) {
                    learned.push((specs, origin));
                    break;
                }
            }
            if !merge_all
                && let Some((specs, _)) = learned.last()
                && !leaves_subcommands_to_help(source, specs, path)
            {
                break;
            }
        }

        if !learned.is_empty() {
            let (specs, origins): (Vec<_>, Vec<_>) = learned.into_iter().unzip();
            let specs = spec::merge(specs);
            let cobra = runner
                .as_ref()
                .is_some_and(|runner| completes_itself(runner, &words, &specs));
            let module = spec::render(tool, &origins, &specs, cobra);
            return Ok(Value::string(module, head).into_pipeline_data());
        }

        let tried = match (&piped, selection) {
            (Some(_), Some(Some(source))) => format!("the piped {} text", source.name()),
            (None, Some(Some(source))) => source.name().to_string(),
            _ => Source::NAMES.join(", "),
        };
        Err(ShellError::Generic(
            GenericError::new(
                format!("couldn't generate completions for `{}`", target.item),
                format!("no flags or subcommands found in {tried}"),
                target.span,
            )
            .with_help(unlisted.unwrap_or_else(|| {
                "install carapace or fish, or pipe the help text in with --source help".into()
            })),
        ))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Generate completions for git and load them at every startup.",
                example: "let dir = $nu.user-autoload-dirs | first; mkdir $dir; generate completions git | save --force ($dir | path join git.nu)",
                result: None,
            },
            Example {
                description: "Generate completions for one subcommand from the command's own --help text.",
                example: "generate completions 'cargo build' --source help",
                result: None,
            },
            Example {
                description: "Merge what every source knows about git into one module.",
                example: "generate completions git --source all",
                result: None,
            },
            Example {
                description: "Read help text that is piped in.",
                example: "'  -s, --short   show status concisely' | generate completions 'git status' --source help",
                result: Some(Value::test_string(
                    "# Generated by `generate completions` from help text piped in.\n\
                     # Flags that are not declared here still reach `git`. Arguments without a completer\n\
                     # here are completed by the external completer in $env.config.completions.external.\n\
                     \n\
                     @complete external\n\
                     export extern \"git status\" [\n    \
                         --short(-s)                    # show status concisely\n\
                     ]\n",
                )),
            },
        ]
    }
}

/// Reads the `--help` of each subcommand that the help of the command `words` lists, and of
/// theirs in turn, so the specs of the whole tree have their flags, as carapace, fish and zsh
/// give them. `specs` is what that help gave: the command's own spec, then one without flags
/// for each subcommand it lists. `helps` holds that help text and those of the commands above.
///
/// Each subcommand that runs this way is one that the help above it lists along with a --help
/// option, as the help source requires before running a subcommand: launchctl's help lists
/// `reboot` but says --help is an unrecognized subcommand, so `launchctl reboot` may read --help
/// as an argument. clap's `help` reads the word after it as a command to describe, so it isn't
/// run. An alias gets the flags of its command, without subcommands, as carapace gives it. A
/// subcommand that prints the help of a command above it didn't read its name, so the
/// subcommands that help lists aren't its own and aren't walked.
fn walk_help(
    runner: &Runner,
    words: &[String],
    helps: &mut Vec<String>,
    specs: Vec<CommandSpec>,
    signals: &nu_protocol::Signals,
    span: Span,
) -> Result<Vec<CommandSpec>, ShellError> {
    let mut specs = specs.into_iter();
    let mut tree: Vec<CommandSpec> = specs.next().into_iter().collect();
    let lists_help = tree.first().is_some_and(CommandSpec::lists_help);
    for listed in specs {
        let Some(name) = listed.path.last() else {
            continue;
        };
        if let Some(command) = &listed.alias_of {
            let command_path = [&words[1..], std::slice::from_ref(command)].concat();
            match tree.iter().find(|spec| spec.path == command_path) {
                Some(walked) => tree.push(CommandSpec {
                    path: listed.path,
                    description: listed.description,
                    alias_of: listed.alias_of,
                    ..walked.clone()
                }),
                None => tree.push(listed),
            }
            continue;
        }
        let sub_words = [words, std::slice::from_ref(name)].concat();
        signals.check(&span)?;
        let text = match name.as_str() {
            "help" => None,
            _ if lists_help => Source::Help.fetch(runner, &sub_words).into_iter().next(),
            _ => None,
        };
        let Some(text) = text.filter(|text| !helps.contains(text)) else {
            tree.push(listed);
            continue;
        };
        let mut sub_specs = help_text::parse(&text, listed.path.clone(), true);
        if let Some(own) = sub_specs.first_mut() {
            own.description = listed.description;
        }
        helps.push(text);
        tree.extend(walk_help(
            runner, &sub_words, helps, sub_specs, signals, span,
        )?);
        helps.pop();
    }
    Ok(tree)
}

/// Whether the specs declare anything beyond an empty signature for the command.
fn knows_anything(specs: &[CommandSpec]) -> bool {
    specs.len() > 1 || specs.iter().any(CommandSpec::declares_anything)
}

/// Whether the help source may know subcommands of the command at `path` that the specs from
/// `source` don't: a man page lists none, and a source may list a subcommand without any of its
/// flags, as fish's own ouch.fish lists ouch's, with every flag on ouch itself. A `help`
/// subcommand and those under it don't count, since the help source doesn't run them either.
fn leaves_subcommands_to_help(source: Source, specs: &[CommandSpec], path: &[String]) -> bool {
    matches!(source, Source::Man)
        || specs.iter().any(|spec| {
            spec.path.len() > path.len()
                && spec.path[path.len()..].iter().all(|word| word != "help")
                && !spec.declares_anything()
        })
}

/// Whether the tool is built with cobra, which completes its own command lines through a
/// hidden `__complete` command, as gh, docker and kubectl are.
///
/// Running a tool with an argument it doesn't know can do something, so the tool is only asked
/// when it has a `completion` subcommand, as every cobra tool does. It answers with candidates
/// and a last line such as `:4` that tells the shell how to use them.
///
/// The specs of a subcommand such as `gh pr` don't include the tool's own `completion`, so
/// then the tool's help is read.
fn completes_itself(runner: &Runner, words: &[String], specs: &[CommandSpec]) -> bool {
    let tool = words[0].as_str();
    let has_completion = match words.len() {
        1 => specs.iter().any(|spec| spec.path == ["completion"]),
        _ => listing_command(runner, &[tool.to_string(), "completion".to_string()]).is_some(),
    };
    has_completion
        && runner
            .stdout(tool, &["__complete", ""], &[])
            .is_some_and(|text| {
                text.lines()
                    .rev()
                    .find(|line| !line.trim().is_empty())
                    .and_then(|line| line.strip_prefix(':'))
                    .is_some_and(|directive| directive.chars().all(|c| c.is_ascii_digit()))
            })
}

/// The spec that the help of the command `words` without its last word gives that command, when
/// it lists that word as a subcommand, as `cargo --help` lists `build`.
fn listing_command(runner: &Runner, words: &[String]) -> Option<CommandSpec> {
    let [parent @ .., _] = words else {
        return None;
    };
    let [_, parent_path @ ..] = parent else {
        return None;
    };
    Source::Help.fetch(runner, parent).iter().find_map(|text| {
        let specs = help_text::parse(text, parent_path.to_vec(), true);
        match specs.iter().any(|spec| spec.path == words[1..]) {
            true => specs.into_iter().next(),
            false => None,
        }
    })
}

/// Whether man has a page for the command `words`, such as `man git-status` for `git status`:
/// its header names the page, and man prints an error instead when it has none.
fn has_man_page(runner: &Runner, words: &[String]) -> bool {
    let page = words.join("-");
    Source::Man.fetch(runner, words).iter().any(|text| {
        help_text::man_page_name(text).is_some_and(|name| name.eq_ignore_ascii_case(&page))
    })
}

#[derive(Clone, Copy)]
enum Source {
    Carapace,
    Fish,
    Zsh,
    Man,
    Help,
}

impl Source {
    const ALL: [Source; 5] = [
        Source::Carapace,
        Source::Fish,
        Source::Zsh,
        Source::Man,
        Source::Help,
    ];
    const NAMES: [&str; 5] = ["carapace", "fish", "zsh", "man", "help"];
    /// The values of `--source`: a source's name, or `all` to merge them.
    const CHOICES: [&str; 6] = ["carapace", "fish", "zsh", "man", "help", "all"];

    fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    fn from_name(name: &str) -> Option<Self> {
        Self::NAMES
            .iter()
            .position(|known| *known == name)
            .map(|index| Self::ALL[index])
    }

    /// Runs the program that prints this source's text for the command `words`. Returns no
    /// text when the program isn't installed or prints nothing, and for zsh, every completion
    /// file for the command on `$fpath`, in order.
    fn fetch(self, runner: &Runner, words: &[String]) -> Vec<String> {
        let tool = words[0].as_str();
        let text = match self {
            Source::Carapace => runner.stdout("carapace", &[tool, "export"], &[]),
            Source::Fish => runner.stdout("fish", &["--no-config", "-c", FISH_SCRIPT, tool], &[]),
            Source::Zsh => {
                let files = runner.stdout(
                    "zsh",
                    &["-c", ZSH_SCRIPT, "generate completions", tool],
                    &[],
                );
                return files
                    .iter()
                    .flat_map(|files| files.lines())
                    .filter_map(|file| std::fs::read_to_string(file).ok())
                    .collect();
            }
            Source::Man => runner.stdout(
                "man",
                &[&words.join("-")],
                &[("MANPAGER", "cat"), ("PAGER", "cat")],
            ),
            Source::Help => {
                let args = words[1..]
                    .iter()
                    .map(String::as_str)
                    .chain(["--help"])
                    .collect::<Vec<_>>();
                runner.stdout(tool, &args, &[])
            }
        };
        text.into_iter().collect()
    }

    /// What the generated module says it was learned from.
    fn origin(self, words: &[String]) -> String {
        match self {
            Source::Carapace => format!("`carapace {} export`", words[0]),
            Source::Fish => format!("the fish completions of `{}`", words[0]),
            Source::Zsh => format!("the zsh completions of `{}`", words[0]),
            Source::Man => format!("`man {}`", words.join("-")),
            Source::Help => format!("`{} --help`", words.join(" ")),
        }
    }

    fn parse(
        self,
        text: &str,
        tool: &str,
        path: &[String],
    ) -> Result<Vec<CommandSpec>, serde_json::Error> {
        match self {
            Source::Carapace => carapace::parse(text, path),
            Source::Fish => Ok(fish::parse(text, tool, path)),
            Source::Zsh => Ok(zsh::parse(text, tool, path)),
            // A man page's command list is mostly the commands of an interactive prompt.
            Source::Man => Ok(help_text::parse(text, path.to_vec(), false)),
            Source::Help => Ok(help_text::parse(text, path.to_vec(), true)),
        }
    }
}

/// Prints the completion script fish would load for the command `$argv[1]`: the first one on
/// `$fish_complete_path`, or the one compiled into fish.
const FISH_SCRIPT: &str = "\
for dir in $fish_complete_path
    if test -f $dir/$argv[1].fish
        cat $dir/$argv[1].fish
        exit
    end
end
status get-file completions/$argv[1].fish 2>/dev/null";

/// Prints the path of each completion file zsh has for the command `$1`, in `$fpath` order.
/// zsh loads the first one, but that may only be a wrapper, such as git's own `_git`, which
/// loads its bash completion.
const ZSH_SCRIPT: &str = "for dir in $fpath; do [[ -f $dir/_$1 ]] && print -r -- $dir/_$1; done";

/// Runs the programs that print source text, with nushell's `PATH`, environment and current
/// directory.
struct Runner {
    paths: String,
    cwd: AbsolutePathBuf,
    envs: HashMap<String, String>,
}

impl Runner {
    fn new(engine_state: &EngineState, stack: &Stack, span: Span) -> Result<Self, ShellError> {
        Ok(Self {
            paths: nu_engine::env::path_str(engine_state, stack, span)?,
            cwd: engine_state.cwd(Some(stack))?,
            envs: env_to_strings(engine_state, stack)?,
        })
    }

    /// Runs `program` without input and returns its stdout, or its stderr when stdout is
    /// empty, since some commands print their help there. The exit status is ignored: many
    /// commands exit with an error after printing their help.
    ///
    /// A stream longer than [`MAX_TEXT`] isn't help, and gives no text. Once it is read that
    /// far, it is closed, so the program gets SIGPIPE when it writes more, and a program still
    /// writing to stdout is killed.
    fn stdout(&self, program: &str, args: &[&str], envs: &[(&str, &str)]) -> Option<String> {
        let executable = crate::which(program, &self.paths, self.cwd.as_ref())?;
        let mut child = Process::new(executable)
            .args(args)
            .current_dir(&self.cwd)
            .env_clear()
            .envs(&self.envs)
            .envs(envs.iter().copied())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .ok()?;
        // stderr is read on a thread of its own, so a program that fills it while stdout is
        // read doesn't wait forever.
        let stderr = child
            .stderr
            .take()
            .map(|stderr| std::thread::spawn(move || read_text(stderr)));
        let stdout = child.stdout.take().and_then(read_text);
        if stdout.is_none() {
            let _ = child.kill();
        }
        let stderr = stderr.and_then(|reader| reader.join().ok().flatten());
        let _ = child.wait();

        let text = match stdout? {
            stdout if stdout.is_empty() => stderr?,
            stdout => stdout,
        };
        let text = String::from_utf8_lossy(&text).into_owned();
        (!text.trim().is_empty()).then_some(text)
    }
}

/// The most text read from a stream of a program that prints source text. The longest text a
/// source gives here is `man ffmpeg-all`, at 2.2 MB. A program that prints more isn't printing
/// help: macOS's `yes --help` prints `--help` forever.
const MAX_TEXT: u64 = 16 * 1024 * 1024;

/// Reads a stream to its end, or returns `None` when it is longer than [`MAX_TEXT`].
fn read_text(stream: impl Read) -> Option<Vec<u8>> {
    let mut text = vec![];
    stream.take(MAX_TEXT + 1).read_to_end(&mut text).ok()?;
    (text.len() as u64 <= MAX_TEXT).then_some(text)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(GenerateCompletions)
    }
}
