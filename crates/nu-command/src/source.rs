use nu_engine::{eval_block, eval_expression};
use nu_parser::parse;
use nu_protocol::ast::{Block, Call};
use nu_protocol::engine::{Command, EngineState, EvaluationContext, StateWorkingSet};
use nu_protocol::{ShellError, Signature, SyntaxShape, Value};
use std::task::Context;
use std::{borrow::Cow, path::Path, path::PathBuf};

/// Source a file for environment variables.
pub struct Source;

impl Command for Source {
    fn name(&self) -> &str {
        "source"
    }

    fn signature(&self) -> Signature {
        Signature::build("source").required(
            "filename",
            SyntaxShape::FilePath,
            "the filepath to the script file to source",
        )
    }

    fn usage(&self) -> &str {
        "Runs a script file in the current context."
    }

    fn run(
        &self,
        _context: &EvaluationContext,
        call: &Call,
        input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        source(_context, call, input)
    }
}

pub fn source(ctx: &EvaluationContext, call: &Call, input: Value) -> Result<Value, ShellError> {
    let filename = call.positional[0]
        .as_string()
        .expect("internal error: missing file name");

    let source_file = Path::new(&filename);

    // This code is in the current Nushell version
    // ...Not entirely sure what it's doing or if there's an equivalent in engine-q

    // Note: this is a special case for setting the context from a command
    // In this case, if we don't set it now, we'll lose the scope that this
    // variable should be set into.

    // let lib_dirs = &ctx
    //     .configs()
    //     .lock()
    //     .global_config
    //     .as_ref()
    //     .map(|configuration| match configuration.var("lib_dirs") {
    //         Some(paths) => paths
    //             .table_entries()
    //             .cloned()
    //             .map(|path| path.as_string())
    //             .collect(),
    //         None => vec![],
    //     });

    // if let Some(dir) = lib_dirs {
    //     for lib_path in dir {
    //         match lib_path {
    //             Ok(name) => {
    //                 let path = PathBuf::from(name).join(source_file);

    //                 if let Ok(contents) =
    //                     std::fs::read_to_string(&expand_path(Cow::Borrowed(path.as_path())))
    //                 {
    //                     let result = script::run_script_standalone(contents, true, ctx, false);

    //                     if let Err(err) = result {
    //                         ctx.error(err);
    //                     }
    //                     return Ok(OutputStream::empty());
    //                 }
    //             }
    //             Err(reason) => {
    //                 ctx.error(reason.clone());
    //             }
    //         }
    //     }
    // }

    // This is to stay consistent w/ the code taken from nushell
    let path = source_file;

    let contents = std::fs::read(path);

    match contents {
        Ok(contents) => {
            let engine_state = EngineState::new();
            let mut working_set = StateWorkingSet::new(&engine_state);

            let (block, err) = parse(&mut working_set, None, &contents, true);
            if let Some(e) = err {
                // Be more specific here: need to convert parse error to string
                Err(e.into())
            } else {
                let result = eval_block(ctx, &block, input);
                match result {
                    Err(e) => Err(e),
                    _ => Ok(Value::nothing()),
                }
            }
        }
        Err(_) => Err(ShellError::InternalError(
            "Can't load file to source".to_string(),
        )),
    }
}
