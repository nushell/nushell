use nu_engine::command_prelude::*;
use nu_protocol::{
    Config, DeclId, PipelineMetadata, Span,
    ast::{Expr, Expression, Traverse},
    shell_error::generic::GenericError,
};

use std::collections::HashSet;
use std::fmt::Write;

#[derive(Clone)]
pub struct ViewSource;

impl Command for ViewSource {
    fn name(&self) -> &str {
        "view source"
    }

    fn description(&self) -> &str {
        "View a block, module, or a definition."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("view source")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .required("item", SyntaxShape::Any, "Name or block to view.")
            .switch(
                "dependencies",
                "Also show the source of every custom command the item calls, transitively.",
                Some('d'),
            )
            .category(Category::Debug)
    }

    fn extra_description(&self) -> &str {
        "The `def` header is rebuilt from the signature instead of being quoted from
the source, so `export` and attributes are lost. That holds with or without
`--dependencies`.

`--dependencies` applies to a custom command; on an alias, a module, a
closure or a block id it does nothing. It follows only the calls the parser
can see, so a closure written inside the body is followed even when it is run
through a variable, but one that arrives as an argument is not. And a command
built on a large module can pull in a lot of output."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "View the source of a code block.",
                example: "let abc = {|| echo 'hi' }; view source $abc",
                result: Some(Value::test_string("{|| echo 'hi' }")),
            },
            Example {
                description: "View the source of a custom command.",
                example: "def hi [] { echo 'Hi!' }; view source hi",
                result: Some(Value::test_string("def hi [] { echo 'Hi!' }")),
            },
            Example {
                description: "View the source of a custom command, which participates in the caller environment.",
                example: "def --env foo [] { $env.BAR = 'BAZ' }; view source foo",
                result: Some(Value::test_string("def --env foo [] { $env.BAR = 'BAZ' }")),
            },
            Example {
                description: "View the source of a custom command with flags.",
                example: "def --wrapped --env foo [...rest: string] { print $rest }; view source foo",
                result: Some(Value::test_string(
                    "def --env --wrapped foo [ ...rest: string] { print $rest }",
                )),
            },
            Example {
                description: "View the source of a custom command with flags and arguments.",
                example: "def test [a?:any --b:int ...rest:string] { echo 'test' }; view source test",
                result: Some(Value::test_string(
                    "def test [ a?: any --b: int ...rest: string] { echo 'test' }",
                )),
            },
            Example {
                description: "View the source of a module.",
                example: "module mod-foo { export-env { $env.FOO_ENV = 'BAZ' } }; view source mod-foo",
                result: Some(Value::test_string(" export-env { $env.FOO_ENV = 'BAZ' }")),
            },
            Example {
                description: "View the source of an alias.",
                example: "alias hello = echo hi; view source hello",
                result: Some(Value::test_string("echo hi")),
            },
            Example {
                description: "View a command together with the commands it calls.",
                example: "def helper [] { 42 }; def caller [] { helper }; view source caller --dependencies",
                result: Some(Value::test_string(
                    "def caller [] { helper }\n\ndef helper [] { 42 }",
                )),
            },
            Example {
                description: "View the file where a definition lives via metadata.",
                example: "view source some_command | metadata",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let arg: Value = call.req(engine_state, stack, 0)?;
        let arg_span = arg.span();

        match arg {
            Value::Int { val, .. } => {
                if let Some(block) =
                    engine_state.try_get_block(nu_protocol::BlockId::new(val as usize))
                {
                    if let Some(span) = block.span {
                        let contents = engine_state.get_span_contents(span);
                        let src = String::from_utf8_lossy(contents).to_string();

                        Ok(make_output(engine_state, src, Some(span), call.head))
                    } else {
                        Err(ShellError::Generic(GenericError::new(
                            "Cannot view int value",
                            "the block does not have a viewable span",
                            arg_span,
                        )))
                    }
                } else {
                    Err(ShellError::Generic(GenericError::new(
                        format!("Block Id {} does not exist", arg.coerce_into_string()?),
                        "this number does not correspond to a block",
                        arg_span,
                    )))
                }
            }

            Value::String { val, .. } => {
                if let Some(decl_id) = engine_state.find_decl(val.as_bytes(), &[]) {
                    // arg is a command
                    let decl = engine_state.get_decl(decl_id);

                    if decl.is_alias() {
                        if let Some(alias) = &decl.as_alias() {
                            let contents = String::from_utf8_lossy(
                                engine_state.get_span_contents(alias.wrapped_call.span),
                            );

                            Ok(make_output(
                                engine_state,
                                contents.to_string(),
                                Some(alias.wrapped_call.span),
                                call.head,
                            ))
                        } else {
                            Ok(make_output(
                                engine_state,
                                "no alias found".to_string(),
                                None,
                                call.head,
                            ))
                        }
                    }
                    // gets vector of positionals.
                    else if decl.block_id().is_some() {
                        if let Some((src, block_span)) = render_def(engine_state, &val, decl_id) {
                            let mut sources = vec![src];

                            if call.has_flag(engine_state, stack, "dependencies")? {
                                let mut emitted = vec![block_span];
                                // Why: the root is already rendered above, under the name the user
                                // typed — which for an imported command is the qualified one.
                                for dep in
                                    decls_with_deps(engine_state, decl_id).into_iter().skip(1)
                                {
                                    // Why: the defining name, not the name the caller imported it
                                    // under. Bodies are quoted verbatim, so a renamed header would
                                    // disagree with its own call sites.
                                    let name = engine_state.get_decl(dep).name();
                                    let Some((dep_src, dep_span)) =
                                        render_def(engine_state, name, dep)
                                    else {
                                        continue;
                                    };
                                    // Why: a `def` nested inside another body is already printed as
                                    // part of that body, so emitting it again just duplicates text.
                                    if emitted.iter().any(|s| s.contains_span(dep_span)) {
                                        continue;
                                    }
                                    emitted.push(dep_span);
                                    sources.push(dep_src);
                                }
                            }

                            Ok(make_output(
                                engine_state,
                                sources.join("\n\n"),
                                Some(block_span),
                                call.head,
                            ))
                        } else {
                            Err(ShellError::Generic(GenericError::new(
                                "Cannot view string value",
                                "the command does not have a viewable block span",
                                arg_span,
                            )))
                        }
                    } else {
                        Err(ShellError::Generic(GenericError::new(
                            "Cannot view string decl value",
                            "the command does not have a viewable block",
                            arg_span,
                        )))
                    }
                } else if let Some(module_id) = engine_state.find_module(val.as_bytes(), &[]) {
                    // arg is a module
                    let module = engine_state.get_module(module_id);
                    if let Some(module_span) = module.span {
                        let contents = engine_state.get_span_contents(module_span);

                        Ok(make_output(
                            engine_state,
                            String::from_utf8_lossy(contents).to_string(),
                            Some(module_span),
                            call.head,
                        ))
                    } else {
                        Err(ShellError::Generic(GenericError::new(
                            "Cannot view string module value",
                            "the module does not have a viewable block",
                            arg_span,
                        )))
                    }
                } else {
                    Err(ShellError::Generic(GenericError::new(
                        "Cannot view string value",
                        "this name does not correspond to a viewable value",
                        arg_span,
                    )))
                }
            }
            value => {
                if let Ok(closure) = value.as_closure() {
                    let block = engine_state.get_block(closure.block_id);

                    if let Some(span) = block.span {
                        let contents = engine_state.get_span_contents(span);

                        Ok(make_output(
                            engine_state,
                            String::from_utf8_lossy(contents).to_string(),
                            Some(span),
                            call.head,
                        ))
                    } else {
                        Ok(make_output(
                            engine_state,
                            "<internal command>".to_string(),
                            None,
                            call.head,
                        ))
                    }
                } else {
                    Err(ShellError::Generic(GenericError::new(
                        "Cannot view value",
                        "this value cannot be viewed",
                        arg_span,
                    )))
                }
            }
        }
    }
}

// Helper function to render a custom command as a `def` header followed by the original text of its
// body. The header is rebuilt from the signature.
// Why: a custom command records only the span of its body, so the header cannot be quoted from the
// source. That also means `export` and attributes cannot be recovered.
fn render_def(engine_state: &EngineState, name: &str, decl_id: DeclId) -> Option<(String, Span)> {
    let decl = engine_state.get_decl(decl_id);
    let sig = decl.signature();
    let vec_of_required = &sig.required_positional;
    let vec_of_optional = &sig.optional_positional;
    let rest = &sig.rest_positional;
    let vec_of_flags = &sig.named;
    let type_signatures = &sig.input_output_types;

    let block = engine_state.get_block(decl.block_id()?);
    let block_span = block.span?;
    let contents = engine_state.get_span_contents(block_span);
    // name of function
    let mut final_contents = String::new();
    // Collect def flags based on block and signature properties
    let flags: Vec<&str> = [
        block.redirect_env.then_some("--env"),
        sig.allows_unknown_args.then_some("--wrapped"),
    ]
    .into_iter()
    .flatten()
    .collect();
    let flags_str = if flags.is_empty() {
        String::new()
    } else {
        format!("{} ", flags.join(" "))
    };
    if name.contains(' ') {
        let _ = write!(&mut final_contents, "def {flags_str}\"{name}\" [");
    } else {
        let _ = write!(&mut final_contents, "def {flags_str}{name} [");
    };
    if !vec_of_required.is_empty()
        || !vec_of_optional.is_empty()
        || vec_of_flags.len() != 1
        || rest.is_some()
    {
        final_contents.push(' ');
    }
    for n in vec_of_required {
        let _ = write!(&mut final_contents, "{}: {} ", n.name, n.shape);
        // positional arguments
    }
    for n in vec_of_optional {
        if let Some(s) = n.default_value.clone() {
            let _ = write!(
                &mut final_contents,
                "{}: {} = {} ",
                n.name,
                n.shape,
                s.to_expanded_string(" ", &Config::default())
            );
        } else {
            let _ = write!(&mut final_contents, "{}?: {} ", n.name, n.shape);
        }
    }
    for n in vec_of_flags {
        // skip adding the help flag
        if n.long == "help" {
            continue;
        }
        let _ = write!(&mut final_contents, "--{}", n.long);
        if let Some(short) = n.short {
            let _ = write!(&mut final_contents, "(-{short})");
        }
        if let Some(arg) = &n.arg {
            let _ = write!(&mut final_contents, ": {arg}");
        }
        final_contents.push(' ');
    }
    if let Some(rest_arg) = rest {
        let _ = write!(
            &mut final_contents,
            "...{}:{}",
            rest_arg.name, rest_arg.shape
        );
    }
    let len = type_signatures.len();
    if len != 0 {
        final_contents.push_str("]: [");
        let mut c = 0;
        for (insig, outsig) in type_signatures {
            c += 1;
            let s = format!("{insig} -> {outsig}");
            final_contents.push_str(&s);
            if c != len {
                final_contents.push_str(", ")
            }
        }
    }
    final_contents.push_str("] ");
    final_contents.push_str(&String::from_utf8_lossy(contents));

    Some((final_contents, block_span))
}

// Helper function to collect a command and every custom command it calls, transitively, in
// breadth-first order.
// Why: every `Expr::Call` carries a `decl_id` bound at parse time, so a command private to a module
// is reachable here even though `find_decl` in the caller's scope cannot see it by name.
fn decls_with_deps(engine_state: &EngineState, root: DeclId) -> Vec<DeclId> {
    let working_set = StateWorkingSet::new(engine_state);
    // Why: commands may call each other in a cycle, so the walk needs a visited set to stop.
    let mut seen = HashSet::from([root]);
    let mut order = vec![root];
    let mut next = 0;

    while next < order.len() {
        let decl_id = order[next];
        next += 1;

        let Some(block_id) = engine_state.get_decl(decl_id).block_id() else {
            continue;
        };
        // `flat_map` takes `Fn`, not `FnMut`, so the closure cannot touch the visited set — it is
        // updated after the traversal returns.
        let leaf = |expr: &Expression| match &expr.expr {
            Expr::Call(call) => vec![call.decl_id],
            _ => Vec::new(),
        };
        let mut called = Vec::new();
        engine_state
            .get_block(block_id)
            .flat_map(&working_set, &leaf, &mut called);

        for dep in called {
            // Why: having a block is the single filter that drops builtins, keywords, known
            // externals and plugins — none of them have a body to show.
            if engine_state.get_decl(dep).block_id().is_some() && seen.insert(dep) {
                order.push(dep);
            }
        }
    }

    order
}

// Helper function to find the file path associated with a given span, if any.
fn file_for_span(engine_state: &EngineState, span: Span) -> Option<String> {
    engine_state
        .files()
        .find(|f| f.covered_span.contains_span(span))
        .map(|f| f.name.to_string())
}

// Helper function to wrap source text into a string value and record the file path in
// datasource metadata when available
fn make_output(
    engine_state: &EngineState,
    src: String,
    span_opt: Option<Span>,
    call_span: Span,
) -> PipelineData {
    let pd = Value::string(src, call_span).into_pipeline_data();

    let mut metadata = PipelineMetadata {
        content_type: Some("application/x-nuscript".into()),
        ..Default::default()
    };
    if let Some(span) = span_opt
        && let Some(fname) = file_for_span(engine_state, span)
    {
        metadata.data_source = nu_protocol::DataSource::FilePath(std::path::PathBuf::from(fname));
    }

    pd.set_metadata(Some(metadata))
}
