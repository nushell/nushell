//! A `print` command for the embedded engine.
//!
//! Nushell's real `print` lives in nu-cli, so an engine built from the
//! library crates alone parses `print foo` as an *external* command, which
//! fails to spawn and can't even coerce a record argument to argv.
//!
//! Ours renders through the `table` command as nu's does, but writes to the
//! DAP client as `output` events: process stdout belongs to the protocol.

use crate::dap::protocol::DapWriter;
use crate::dap::types::DapEvent;
use crate::state::DebugState;
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct DapPrint {
    pub writer: DapWriter,
}

/// How one `print` invocation renders: its flags plus the DAP `output`
/// category they resolved to.
#[derive(Clone, Copy)]
struct PrintOpts<'a> {
    no_newline: bool,
    /// `--raw`: no table pass, and binary goes out as its own bytes.
    raw: bool,
    category: &'a str,
}

impl Command for DapPrint {
    fn name(&self) -> &str {
        "print"
    }

    fn description(&self) -> &str {
        "Print the given values to the debug console."
    }

    fn signature(&self) -> Signature {
        Signature::build("print")
            .input_output_types(vec![
                (Type::Nothing, Type::Nothing),
                (Type::Any, Type::Nothing),
            ])
            .allow_variants_without_examples(true)
            .rest("rest", SyntaxShape::Any, "The values to print.")
            .switch(
                "no-newline",
                "print without inserting a newline for the line ending",
                Some('n'),
            )
            .switch("stderr", "print to stderr instead of stdout", Some('e'))
            .switch(
                "raw",
                "print without formatting (including binary data)",
                Some('r'),
            )
            .category(Category::Strings)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let args: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let no_newline = call.has_flag(engine_state, stack, "no-newline")?;
        let to_stderr = call.has_flag(engine_state, stack, "stderr")?;
        let opts = PrintOpts {
            no_newline,
            raw: call.has_flag(engine_state, stack, "raw")?,
            category: if to_stderr { "stderr" } else { "stdout" },
        };

        if args.is_empty() {
            if !matches!(input, PipelineData::Empty) {
                self.emit(engine_state, stack, call, input, opts)?;
            }
        } else {
            for arg in args {
                self.emit(engine_state, stack, call, arg.into_pipeline_data(), opts)?;
            }
        }
        Ok(PipelineData::empty())
    }
}

// Prompts have no terminal under the debugger (stdin is NUL), but there is a
// UI on the wire: `input`/`input list` emit a `nuDapUi` event, the extension
// answers with `nuDapUiReply`, and the eval thread blocks in between.

const UI_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

/// Block until the client answers UI request `id` (or cancel/terminate).
fn wait_ui_reply(
    state: &DebugState,
    id: u64,
    span: nu_protocol::Span,
) -> Result<crate::state::UiReply, ShellError> {
    let deadline = nu_utils::time::Instant::now() + UI_TIMEOUT;
    let mut replies = state.ui.replies.lock();
    loop {
        if let Some(reply) = replies.remove(&id) {
            return Ok(reply);
        }
        if state
            .terminate_flag
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return Err(ShellError::Generic(GenericError::new(
                "debug session is terminating".to_string(),
                "prompt abandoned".to_string(),
                span,
            )));
        }
        if nu_utils::time::Instant::now() >= deadline {
            return Err(ShellError::Generic(GenericError::new(
                "no answer from the debugger UI".to_string(),
                "the prompt dialog timed out".to_string(),
                span,
            )));
        }
        state
            .ui
            .cv
            .wait_for(&mut replies, std::time::Duration::from_millis(500));
    }
}

/// Render one `input list` choice as the line the user reads in the picker.
///
/// Strings pass through bare, matching upstream `input list`: `[apple banana]
/// | input list` must read `apple`, not the Variables row's quoted `"apple"`.
/// Everything else borrows [`crate::variables::short_render`], the only
/// renderer that keeps a container to one bounded line. See
/// `docs/value-rendering.md`.
fn pick_label(value: &Value, ctx: crate::variables::RenderCtx<'_>) -> String {
    match value {
        Value::String { val, .. } => val.clone(),
        other => crate::variables::short_render(other, ctx),
    }
}

/// `input [prompt]` → VS Code input box.
#[derive(Clone)]
pub(crate) struct DapInput {
    pub state: Arc<DebugState>,
    pub writer: DapWriter,
}

impl std::fmt::Debug for DapInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DapInput")
    }
}

impl Command for DapInput {
    fn name(&self) -> &str {
        "input"
    }

    fn description(&self) -> &str {
        "Prompt for text via the debugger UI (input box)."
    }

    fn signature(&self) -> Signature {
        Signature::build("input")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .allow_variants_without_examples(true)
            .optional("prompt", SyntaxShape::String, "Prompt to show.")
            .named(
                "default",
                SyntaxShape::String,
                "default value if nothing is entered",
                Some('d'),
            )
            .switch("suppress-output", "ignored under the debugger", Some('s'))
            .category(Category::Platform)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let prompt: Option<String> = call.opt(engine_state, stack, 0)?;
        let default: Option<String> = call.get_flag(engine_state, stack, "default")?;
        let id = self
            .state
            .ui
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        self.writer.event(DapEvent::NuDapUi {
            id,
            kind: "text",
            prompt: prompt.unwrap_or_else(|| "Input".into()),
            default,
            items: None,
            multi: None,
            truncated: None,
        });

        let reply = wait_ui_reply(&self.state, id, call.head)?;
        if reply.cancelled {
            return Ok(Value::nothing(call.head).into_pipeline_data());
        }
        Ok(Value::string(reply.value.unwrap_or_default(), call.head).into_pipeline_data())
    }
}

/// `input list [prompt] --multi --index --fuzzy` → VS Code quick pick.
#[derive(Clone)]
pub(crate) struct DapInputList {
    pub state: Arc<DebugState>,
    pub writer: DapWriter,
}

impl std::fmt::Debug for DapInputList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DapInputList")
    }
}

impl Command for DapInputList {
    fn name(&self) -> &str {
        "input list"
    }

    fn description(&self) -> &str {
        "Pick from a list via the debugger UI (quick pick)."
    }

    fn signature(&self) -> Signature {
        Signature::build("input list")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .allow_variants_without_examples(true)
            .optional("prompt", SyntaxShape::String, "Prompt to show.")
            .switch("multi", "allow multiple selections", Some('m'))
            .switch("index", "return the index instead of the item", Some('i'))
            .switch(
                "fuzzy",
                "ignored (the quick pick filters natively)",
                Some('f'),
            )
            .category(Category::Platform)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let prompt: Option<String> = call.opt(engine_state, stack, 0)?;
        let multi = call.has_flag(engine_state, stack, "multi")?;
        let as_index = call.has_flag(engine_state, stack, "index")?;

        let items: Vec<Value> = match input.into_value(call.head)? {
            Value::List { vals, .. } => vals.to_vec(),
            Value::Nothing { .. } => Vec::new(),
            single => vec![single],
        };
        if items.is_empty() {
            return Err(ShellError::Generic(GenericError::new(
                "no options to choose from".to_string(),
                "`input list` received an empty list".to_string(),
                call.head,
            )));
        }
        const MAX_ITEMS: usize = 1000;
        let config = engine_state.get_config();
        let cache = self.state.cache.lock().clone();
        let ctx = crate::variables::RenderCtx {
            config,
            cache: &cache,
        };
        let labels: Vec<String> = items
            .iter()
            .take(MAX_ITEMS)
            .map(|v| pick_label(v, ctx))
            .collect();

        let id = self
            .state
            .ui
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        self.writer.event(DapEvent::NuDapUi {
            id,
            kind: "list",
            prompt: prompt.unwrap_or_else(|| "Select an item".into()),
            default: None,
            items: Some(labels),
            multi: Some(multi),
            truncated: Some(items.len() > MAX_ITEMS),
        });

        let reply = wait_ui_reply(&self.state, id, call.head)?;
        if reply.cancelled {
            return Ok(Value::nothing(call.head).into_pipeline_data());
        }

        let value = if multi {
            let picked = reply.indices.unwrap_or_default();
            if as_index {
                Value::list(
                    picked
                        .iter()
                        .map(|&i| Value::int(i as i64, call.head))
                        .collect(),
                    call.head,
                )
            } else {
                Value::list(
                    picked
                        .iter()
                        .filter_map(|&i| items.get(i).cloned())
                        .collect(),
                    call.head,
                )
            }
        } else {
            let i = reply.index.unwrap_or(0);
            if as_index {
                Value::int(i as i64, call.head)
            } else {
                items.get(i).cloned().unwrap_or(Value::nothing(call.head))
            }
        };
        Ok(value.into_pipeline_data())
    }
}

/// Commands that genuinely can't work without a terminal.
#[derive(Clone, Debug)]
pub(crate) struct DapInputUnsupported {
    pub name: &'static str,
}

impl Command for DapInputUnsupported {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "Not available under the debugger."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name)
            .input_output_types(vec![(Type::Any, Type::Any)])
            .allow_variants_without_examples(true)
            .rest("rest", SyntaxShape::Any, "Ignored.")
            .category(Category::Platform)
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Err(ShellError::Generic(
            GenericError::new(
                format!("`{}` is not supported under nu-dap", self.name),
                "raw key events need a real terminal".to_string(),
                call.head,
            )
            .with_help("run the script with `nu` directly"),
        ))
    }
}

impl DapPrint {
    /// Render data like nu's `print` (through `table` when registered) and
    /// forward the text to the DAP client.
    fn emit(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        data: PipelineData,
        opts: PrintOpts<'_>,
    ) -> Result<(), ShellError> {
        // `--raw`, as upstream: no table pass, and binary is its own bytes
        // rather than a hex dump. The DAP wire carries text, so the bytes are
        // decoded lossily on the way out.
        if opts.raw {
            if let PipelineData::Value(Value::Binary { val: bytes, .. }, _) = &data {
                self.writer
                    .output(opts.category, String::from_utf8_lossy(bytes).into_owned());
                return Ok(());
            }
            return self.emit_values(engine_state, data, opts);
        }

        let data = match engine_state.table_decl_id {
            Some(decl_id) => {
                let table_call = nu_protocol::ast::Call::new(call.head);
                engine_state.get_decl(decl_id).run(
                    engine_state,
                    stack,
                    &(&table_call).into(),
                    data,
                )?
            }
            None => data,
        };

        self.emit_values(engine_state, data, opts)
    }

    /// Render each value in `data` and send it as an `output` event.
    fn emit_values(
        &self,
        engine_state: &EngineState,
        data: PipelineData,
        opts: PrintOpts<'_>,
    ) -> Result<(), ShellError> {
        let config = engine_state.get_config();
        for value in data {
            let mut out = match value {
                Value::Error { error, .. } => return Err(*error),
                v => v.to_expanded_string("\n", config),
            };
            if !opts.no_newline {
                out.push('\n');
            }
            self.writer.output(opts.category, out);
        }
        Ok(())
    }
}
