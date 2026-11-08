use super::{builder_io_types, with_app};
use crate::app::merge_context;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiContext;

impl Command for TuiContext {
    fn name(&self) -> &str {
        "tui context"
    }

    fn description(&self) -> &str {
        "Give this TUI a record of its own state, shared by every hook."
    }

    fn extra_description(&self) -> &str {
        "The record is what a TUI knows that no widget holds: the directory a file browser is in, whether it shows dotfiles. Every hook reads it as `$state.context`, and `tui run` returns it. A hook changes it with `{action: context, value: {...}}`, which merges key by key (a nested record replaces the one under its key); a hook's `$env` changes end with the hook, so this is where such state lives.\n\
         \n\
         Its `views` record keeps one part per widget id: `views.view` is what `tui preview --id view --on-change` returns for each new file and `{action: context, id: view, value: {...}}` merges into. A preview's closure sees the record without `views` and with its own part laid over as `$ctx`, and runs again when any key it sees changes. A second `tui context` merges over the first."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui context")
            .category(Category::Viewers)
            .required(
                "context",
                SyntaxShape::Record(vec![].into()),
                "What the TUI starts with.",
            )
            .input_output_types(builder_io_types())
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["state", "shared", "hook"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "A directory the hooks list, kept outside the rows",
                example: "ls | tui table | tui context {dir: (pwd)} | tui bind backspace {|s| let up = $s.context.dir | path dirname; [{action: context, value: {dir: $up}} {action: data, value: (ls $up)}] } | tui run",
                result: None,
            },
            Example {
                description: "Shared keys a preview closure reads beside its own `--on-change` part",
                example: "ls | tui split [(tui table) (tui preview --id view --on-change {|| {page: 1} } {|row, ctx| pdftoppm -png -singlefile -f $ctx.page -scale-to $ctx.width $row.name | complete | get stdout })] | tui context {width: 1600} | tui run",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let context: Value = call.req(engine_state, stack, 0)?;
        with_app(call, input, |app| {
            app.context = merge_context(&app.context, context);
            Ok(())
        })
    }
}
