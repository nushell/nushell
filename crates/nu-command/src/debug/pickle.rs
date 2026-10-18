use nu_engine::command_prelude::*;
use nu_parser::{parse, pickle};
use nu_path::expand_path_with;
use nu_protocol::{
    ModuleId,
    ast::{Block, Expr, Expression, Traverse},
    engine::{DEFAULT_OVERLAY_NAME, FileStack, ScopeFrame, StateWorkingSet},
    report_error::report_compile_error,
    report_parse_error, report_parse_warning,
    shell_error::{
        generic::GenericError,
        io::{IoError, IoErrorExt},
    },
};
use std::{collections::BTreeSet, path::Path};

#[derive(Clone)]
pub struct Pickle;

impl Command for Pickle {
    fn name(&self) -> &str {
        "pickle"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("script", SyntaxShape::Filepath, "The script to pickle.")
            .named(
                "output",
                SyntaxShape::Filepath,
                "Where to write the pickle, instead of next to the script with a `.nupkl` extension.",
                Some('o'),
            )
            .input_output_type(Type::Nothing, Type::record())
            .category(Category::Debug)
    }

    fn description(&self) -> &str {
        "Parse and compile a script, and save the result as a pickle that `nu` runs without parsing it again."
    }

    fn extra_description(&self) -> &str {
        "`nu script.nupkl [args]` runs the pickle the way `nu script.nu [args]` runs the script, from \
any directory or machine and without the files it `use`s or `source`s. Constants and parameter \
defaults are computed again where it runs. Only the same nushell version and commit runs it, and \
it runs with the experimental options this shell has now, whatever `nu` would otherwise use.

The script is parsed in this shell, the way `source` would parse it. Commands and constants from this \
shell's configuration are linked by name, so a nushell without them refuses to run the pickle. Aliases \
are compiled as what they expand to, which `nu script.nu` wouldn't do, so `warnings` lists the ones \
the script used. An existing pickle is replaced, any other file is kept."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["nupkl", "compile", "ir", "precompile"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let script: Spanned<String> = call.req(engine_state, stack, 0)?;
        let output: Option<Spanned<String>> = call.get_flag(engine_state, stack, "output")?;

        let cwd = engine_state.cwd(Some(stack))?;
        let script_path = expand_path_with(&script.item, &cwd, true);
        let contents = std::fs::read(&script_path).map_err(|err| {
            IoError::new(
                err.not_found_as(NotFound::File),
                script.span,
                script_path.clone(),
            )
        })?;
        let (output, output_span) = match output {
            Some(output) => (expand_path_with(&output.item, &cwd, true), output.span),
            None => (script_path.with_extension("nupkl"), script.span),
        };
        // This also keeps the script from being replaced by its own pickle.
        if std::fs::read(&output).is_ok_and(|existing| !pickle::is_pickle(&existing)) {
            return Err(ShellError::Generic(
                GenericError::new(
                    format!("{} already exists", output.display()),
                    "it isn't a pickle, so it's kept",
                    output_span,
                )
                .with_help("choose another file with --output"),
            ));
        }

        // Parse like `nu script.nu` does, with the script on the file stack so that `use`,
        // `source` and `path self` start from its directory, and in the default overlay, where a
        // script runs, whatever overlay is active here. It's parsed standalone, so what this shell
        // already loaded is parsed again, into the pickle. The working set is never merged.
        let mut working_set = StateWorkingSet::new(engine_state);
        working_set.files = FileStack::with_file(script_path.clone());
        working_set.delta.scope = vec![ScopeFrame::with_empty_overlay(
            DEFAULT_OVERLAY_NAME.into(),
            ModuleId::ZERO,
            false,
        )];
        working_set.standalone = true;
        let block = parse(
            &mut working_set,
            Some(&script_path.to_string_lossy()),
            &contents,
            false,
        );
        for warning in &working_set.parse_warnings {
            report_parse_warning(Some(stack), &working_set, warning);
        }
        let reported = if let Some(err) = working_set.parse_errors.first() {
            report_parse_error(Some(stack), &working_set, err);
            true
        } else if let Some(err) = working_set.compile_errors.first() {
            report_compile_error(Some(stack), &working_set, err);
            true
        } else {
            false
        };
        if reported {
            return Err(ShellError::Generic(GenericError::new(
                "Can't pickle program",
                "the script has errors, shown above",
                script.span,
            )));
        }

        let dir = script_path.parent().unwrap_or(Path::new(""));
        let pickled = pickle::save(&working_set, &block, dir)?;
        std::fs::write(&output, &pickled)
            .map_err(|err| IoError::new(err, output_span, output.clone()))?;

        let warnings: Vec<String> = shell_aliases(&working_set, &block)
            .into_iter()
            .map(|name| {
                format!(
                    "`{name}` is an alias in this shell and was compiled as what it expands to. \
`nu {}` wouldn't use the alias.",
                    script.item
                )
            })
            .collect();
        Ok(Value::record(
            record! {
                "path" => Value::string(output.to_string_lossy(), head),
                "size" => Value::filesize(pickled.len() as i64, head),
                "warnings" => warnings.into_value(head),
            },
            head,
        )
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Pickle `script.nu` into `script.nupkl`, then run the pickle.",
                example: "pickle script.nu; nu script.nupkl",
                result: None,
            },
            Example {
                description: "Choose where the pickle goes.",
                example: "pickle script.nu --output build/tool.nupkl",
                result: None,
            },
        ]
    }
}

/// The names of this shell's aliases that `block`, or any block parsed with it, calls.
///
/// The parser replaces a call of an alias with the call the alias wraps, but keeps the alias's
/// name as the call's head. So a call is an alias expansion when its head names one of the
/// shell's aliases and it calls what that alias wraps.
fn shell_aliases(working_set: &StateWorkingSet, block: &Block) -> BTreeSet<String> {
    let shell_decls = working_set.permanent_state.num_decls();
    let expanded_alias = |expr: &Expression| {
        let head = match &expr.expr {
            Expr::Call(call) => call.head,
            Expr::ExternalCall(head, _) => head.span,
            _ => return None,
        };
        let name = working_set.get_span_contents(head);
        let decl_id = working_set
            .find_decl(name)
            .filter(|decl_id| decl_id.get() < shell_decls)?;
        let alias = working_set.get_decl(decl_id).as_alias()?;
        let expands = match (&alias.wrapped_call.expr, &expr.expr) {
            (Expr::Call(wrapped), Expr::Call(call)) => wrapped.decl_id == call.decl_id,
            (Expr::ExternalCall(wrapped, _), Expr::ExternalCall(called, _)) => {
                wrapped.expr == called.expr
            }
            _ => false,
        };
        expands.then(|| String::from_utf8_lossy(name).into_owned())
    };

    let mut names = vec![];
    for block in working_set
        .delta
        .blocks
        .iter()
        .map(|block| block.as_ref())
        .chain([block])
    {
        block.flat_map(
            working_set,
            &|expr| expanded_alias(expr).into_iter().collect(),
            &mut names,
        );
    }
    names.into_iter().collect()
}
