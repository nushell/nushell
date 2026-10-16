use crate::help::{help_aliases, help_commands, help_modules};
use nu_engine::{HELP_DECL_ID_PARSER_INFO, command_prelude::*, find_builtin_decl, get_full_help};
use nu_protocol::{DeclId, ast::Expr};

#[derive(Clone)]
pub struct Help;

impl Command for Help {
    fn name(&self) -> &str {
        "help"
    }

    fn signature(&self) -> Signature {
        Signature::build("help")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .rest(
                "rest",
                SyntaxShape::String,
                "The name of command, alias or module to get help on.",
            )
            .named(
                "find",
                SyntaxShape::String,
                "String to find in command names, descriptions, and search terms.",
                Some('f'),
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Display help information about different parts of Nushell."
    }

    fn extra_description(&self) -> &str {
        r#"`help word` searches for "word" in commands, aliases and modules, in that order.
If you want your own help implementation, create a custom command named `help` and it will also be used for `--help` invocations.
There already is an alternative `help` command in the standard library you can try with `use std/help`."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let find: Option<Spanned<String>> = call.get_flag(engine_state, stack, "find")?;
        let rest: Vec<Spanned<String>> = call.rest(engine_state, stack, 0)?;

        if let Some(resolved_decl_id) = resolved_help_decl_id(call, stack, engine_state) {
            return Ok(help_for_decl_id(
                engine_state,
                stack,
                head,
                resolved_decl_id,
            ));
        }

        // `help %cmd` is parsed as a string argument, so `%` must be handled here.
        if find.is_none()
            && let Some(name) = builtin_help_lookup_name(&rest)
        {
            if let Some(decl_id) = find_builtin_decl(engine_state, &name) {
                return Ok(help_for_decl_id(engine_state, stack, head, decl_id));
            }

            return Err(ShellError::NotFound {
                span: Span::merge_many(rest.iter().map(|s| s.span)),
            });
        }

        fn help_for_decl_id(
            engine_state: &EngineState,
            stack: &mut Stack,
            head: Span,
            decl_id: DeclId,
        ) -> PipelineData {
            let decl = engine_state.get_decl(decl_id);
            let help = get_full_help(decl, engine_state, stack, head);
            Value::string(help, head).into_pipeline_data()
        }
        if rest.is_empty() && find.is_none() {
            let msg = r#"Welcome to Nushell.

Here are some tips to help you get started.
  * help -h or help help - show available `help` subcommands and examples
  * help commands - list all available commands
  * help <name> - display help about a particular command, alias, or module
  * help --find <text to search> - search through all help commands table

Nushell works on the idea of a "pipeline". Pipelines are commands connected with the '|' character.
Each stage in the pipeline works together to load, parse, and display information to you.

[Examples]

List the files in the current directory, sorted by size:
    ls | sort-by size

Get the current system host name:
    sys host | get hostname

Get the processes on your system actively using CPU:
    ps | where cpu > 0

You can also learn more at https://www.nushell.sh/book/"#;

            Ok(Value::string(msg, head).into_pipeline_data())
        } else if find.is_some() {
            help_commands(engine_state, stack, call)
        } else {
            let result = help_aliases(engine_state, stack, call);

            let result = if let Err(ShellError::AliasNotFound { .. }) = result {
                help_commands(engine_state, stack, call)
            } else {
                result
            };

            let result = if let Err(ShellError::CommandNotFound { .. }) = result {
                help_modules(engine_state, stack, call)
            } else {
                result
            };

            if let Err(ShellError::ModuleNotFoundAtRuntime {
                mod_name: _,
                span: _,
            }) = result
            {
                Err(ShellError::NotFound {
                    span: Span::merge_many(rest.iter().map(|s| s.span)),
                })
            } else {
                result
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "show help for single command, alias, or module.",
                example: "help match",
                result: None,
            },
            Example {
                description: "show help for single sub-command, alias, or module.",
                example: "help str join",
                result: None,
            },
            Example {
                description: "search for string in command names, descriptions, and search terms.",
                example: "help --find char",
                result: None,
            },
        ]
    }
}

// `compile_call` rewrites `<cmd> --help` to `help <name>`. This helper restores the original
// resolved declaration identity from parser info so help output stays tied to the original call.
fn resolved_help_decl_id(call: &Call, stack: &Stack, engine_state: &EngineState) -> Option<DeclId> {
    call.get_parser_info(stack, HELP_DECL_ID_PARSER_INFO)
        .and_then(|expr| match expr.expr {
            Expr::Int(id) => usize::try_from(id).ok().map(DeclId::new),
            _ => None,
        })
        .filter(|decl_id| decl_id.get() < engine_state.num_decls())
}

// For plain `help`, treat `%` on the first token as a built-in resolution request and normalize
// the command name to be looked up (for example `%str join` -> `str join`).
fn builtin_help_lookup_name(rest: &[Spanned<String>]) -> Option<String> {
    let (first, tail) = rest.split_first()?;
    let first = first.item.strip_prefix('%')?;

    let mut name = String::new();
    if !first.is_empty() {
        name.push_str(first);
    }

    for item in tail {
        if !name.is_empty() {
            name.push(' ');
        }
        name.push_str(&item.item);
    }

    Some(name)
}
