mod input;
mod output;

pub(crate) use output::report;
pub(crate) use output::{Returned, SpanClamp, map_value_completions};

use crate::completions::{
    Completer, Context, Fetched,
    completer::{closure_is_interactive, decl_is_interactive},
};
pub use input::DeclaredInputs;
pub(crate) use input::{completer_input, legacy_context, legacy_pos, legacy_spans};
use nu_engine::compile;
use nu_protocol::{
    BlockId, DeclId, PipelineData, ReportMode, ShellError, ShellWarning, Signature, Span, Value,
    VarId,
    ast::Block,
    debugger::WithoutDebug,
    engine::{Closure, Command, EngineState, Stack, StateWorkingSet},
    report_shell_warning,
};
pub(crate) use output::CompleterOutput;
use std::{
    borrow::Cow,
    hash::{DefaultHasher, Hash, Hasher},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex},
};

thread_local! {
    static COMPLETION_PANIC_ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether the current thread is evaluating a completion source. The main panic hook uses this
/// to avoid printing a panic that completion already converted into a `ShellError`.
pub fn completion_panic_is_active() -> bool {
    COMPLETION_PANIC_ACTIVE.with(std::cell::Cell::get)
}

/// Catch a completion-source panic while marking it for the process panic hook.
pub(crate) fn catch_completion_panic<T>(
    f: impl FnOnce() -> T,
) -> Result<T, Box<dyn std::any::Any + Send>> {
    COMPLETION_PANIC_ACTIVE.with(|active| {
        let previous = active.replace(true);
        let result = catch_unwind(AssertUnwindSafe(f));
        active.set(previous);
        result
    })
}

/// Who filters the candidates against the typed prefix; overridable via `options.filter`.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Narrowing {
    /// The engine filters; parameter completers list every candidate.
    Engine,
    /// The completer narrowed its own list; command-wide/external completers see the typed
    /// text and may match fuzzily (e.g. carapace).
    Completer,
}

impl Narrowing {
    /// Whether the engine filters when the completer expresses no `options.filter`.
    fn filters_by_default(self) -> bool {
        matches!(self, Self::Engine)
    }
}

/// Borrow the permanent engine state when the completer lives in it (no per-keystroke
/// clone); otherwise clone it and merge the working-set delta.
fn engine_state_for_completion<'a>(
    working_set: &'a StateWorkingSet<'_>,
    is_permanent: bool,
) -> Cow<'a, EngineState> {
    if is_permanent {
        Cow::Borrowed(working_set.permanent_state)
    } else {
        let mut engine_state = working_set.permanent_state.clone();
        let _ = engine_state.merge_delta(working_set.delta.clone());
        Cow::Owned(engine_state)
    }
}

/// Fields available to a custom completer.
pub(crate) const INPUT_FIELDS: [&str; 3] = ["token", "place", "buffer"];

/// A legacy positional interface which predates named completion inputs.
#[derive(Debug, Clone, Copy)]
pub(crate) enum LegacyInputKind {
    /// A custom completer attached to one parameter.
    Parameter,
    /// A command-wide or configured external completer.
    Command,
    /// A menu source.
    Menu,
}

impl LegacyInputKind {
    fn migration(self) -> &'static str {
        match self {
            Self::Parameter => {
                "use `[buffer, place]`; `$buffer` replaces the old context and \
                 `$place.cursor` replaces its position"
            }
            Self::Command => {
                "use `[buffer]`; split or parse `$buffer` if the old token list is needed"
            }
            Self::Menu => "use `[buffer, place]`; `$place.cursor` replaces the old position",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Parameter => "parameter",
            Self::Command => "command or external",
            Self::Menu => "menu",
        }
    }
}

/// Values supplied to an old positional completer interface.
///
/// New-style inputs are always selected by name. Only the first two unrecognised positionals
/// can opt into this bridge, mirroring the two values the old interfaces accepted.
pub(crate) struct LegacyInputs {
    kind: LegacyInputKind,
    values: [Option<Value>; 2],
    /// The completer's definition, which the deprecation points at.
    definition: Span,
}

impl LegacyInputs {
    /// Construct a bridge only when the signature actually needs it, avoiding legacy parsing
    /// and allocations for completers that already use named inputs.
    ///
    /// `site` locates the completer when its block carries no span of its own.
    fn when_needed(
        kind: LegacyInputKind,
        block: &Block,
        site: Span,
        values: impl FnOnce() -> [Value; 2],
    ) -> Self {
        let needs_legacy = block
            .signature
            .required_positional
            .iter()
            .chain(&block.signature.optional_positional)
            .take(2)
            .any(|positional| !INPUT_FIELDS.contains(&positional.name.as_str()));

        let values = needs_legacy
            .then(values)
            .map(|[first, second]| [Some(first), Some(second)])
            .unwrap_or([None, None]);
        Self {
            kind,
            values,
            definition: block.span.unwrap_or(site),
        }
    }

    pub(crate) fn parameter(ctx: &Context, block: &Block) -> Self {
        Self::when_needed(LegacyInputKind::Parameter, block, ctx.span, || {
            [legacy_context(ctx), legacy_pos(ctx)]
        })
    }

    pub(crate) fn command(ctx: &Context, block: &Block) -> Self {
        Self::when_needed(LegacyInputKind::Command, block, ctx.span, || {
            // Command-wide and external completers historically received only `$spans`.
            [legacy_spans(ctx), Value::nothing(ctx.span)]
        })
    }

    pub(crate) fn menu(block: &Block, buffer: &str, position: usize, span: Span) -> Self {
        Self::when_needed(LegacyInputKind::Menu, block, span, || {
            [
                Value::string(buffer, span),
                Value::int(position as i64, span),
            ]
        })
    }

    fn value(&self, index: usize) -> Option<Value> {
        self.values.get(index).and_then(Clone::clone)
    }

    /// Warn that this completer runs on the bridge. Unlike the mistakes [`report`] logs,
    /// this one costs nothing today and everything once the bridge is removed, so it is
    /// raised where a user will actually see it: queued for [`flush_completion_warnings`].
    fn deprecate(&self, names: &[String]) {
        let (plural, still) = match names.len() {
            1 => ("", "still receives its old positional value"),
            _ => ("s", "still receive their old positional values"),
        };
        let names = names.join("`, `");

        queue(ShellWarning::Deprecated {
            dep_type: "Positional completer input".into(),
            label: format!("declares the legacy positional input{plural} `{names}`"),
            span: self.definition,
            help: Some(format!(
                "A {} completer is handed one record, whose fields bind to the parameters it \
                 names: {}. `{names}` {still} for compatibility, but this bridge will be \
                 removed in a future release. To migrate, {}.",
                self.kind.name(),
                INPUT_FIELDS.join(", "),
                self.kind.migration(),
            )),
            report_mode: ReportMode::FirstUse,
        });
    }
}

/// Deprecations raised while completing, each with the hash that keeps a duplicate out.
///
/// Completion runs while the line editor owns the terminal — for a background query, on
/// another thread — so a warning printed here would land in the middle of the line being
/// typed. They wait here for the next caller that can safely print instead.
static PENDING: Mutex<Vec<(u64, ShellWarning)>> = Mutex::new(Vec::new());

/// Queue a deprecation, unless the same one is already waiting.
fn queue(warning: ShellWarning) {
    let mut hasher = DefaultHasher::new();
    warning.hash(&mut hasher);
    let digest = hasher.finish();

    // A poisoned lock must never interfere with completion.
    if let Ok(mut pending) = PENDING.lock()
        && !pending.iter().any(|(queued, _)| *queued == digest)
    {
        pending.push((digest, warning));
    }
}

/// Print the deprecations completion raised, now that printing is safe: the REPL calls this
/// once the line editor hands back the line, and `commandline complete` when it returns.
/// Each is still shown only once a session, through [`ReportMode::FirstUse`].
pub fn flush_completion_warnings(engine_state: &EngineState, stack: &Stack) {
    // Taken, not printed under the lock: a background completion may be queueing into it.
    let pending = match PENDING.lock() {
        Ok(mut pending) => std::mem::take(&mut *pending),
        Err(_) => return,
    };

    for (_, warning) in pending {
        report_shell_warning(Some(stack), engine_state, &warning);
    }
}

/// Bind declared positional names to matching fields in the input record.
///
/// A non-`token`/`place`/`buffer` name in either of the first two slots receives its old
/// positional value through [`LegacyInputs`] instead of `nothing`. This keeps scripts such as
/// fzf's `{|spans|}` and zoxide's `[context, pos]` working while they migrate. Other unknown
/// inputs still receive `nothing` with a diagnostic.
pub(crate) fn bind_declared_inputs(
    stack: &mut Stack,
    signature: &Signature,
    input: Value,
    legacy: LegacyInputs,
) {
    let span = input.span();
    let Ok(record) = input.into_record() else {
        return;
    };

    let mut legacy_names = Vec::new();
    for (index, positional) in signature
        .required_positional
        .iter()
        .chain(&signature.optional_positional)
        .enumerate()
    {
        if let Some(var_id) = positional.var_id {
            if INPUT_FIELDS.contains(&positional.name.as_str()) {
                let value = record
                    .get(positional.name.as_str())
                    .cloned()
                    .unwrap_or_else(|| Value::nothing(span));
                stack.add_var(var_id, value);
            } else if let Some(old) = legacy.value(index) {
                legacy_names.push(positional.name.clone());
                stack.add_var(var_id, old);
            } else {
                report(format!(
                    "a completer declares `{}`, which is not a completion input; expected one \
                     of {} — it will receive nothing",
                    positional.name,
                    INPUT_FIELDS.join(", ")
                ));
                stack.add_var(var_id, Value::nothing(span));
            }
        }
    }

    if !legacy_names.is_empty() {
        legacy.deprecate(&legacy_names);
    }
}

/// The block a declaration runs, seeing through aliases (a completer named by one).
fn block_of(command: &dyn Command) -> Option<BlockId> {
    command
        .block_id()
        .or_else(|| block_of(command.as_alias()?.command.as_deref()?))
}

/// A user-defined completer: a block called with the input record. Parameter,
/// command-wide, and external completers share this one implementation.
pub(crate) struct UserCompletion {
    block_id: BlockId,
    captures: Vec<(VarId, Value)>,
    narrowing: Narrowing,
    /// Whether this completer is `@interactive`: it takes the terminal and asks the user.
    interactive: bool,
}

impl UserCompletion {
    /// A completer attached to one parameter (`x: string@"nu-complete foo"`); the engine
    /// narrows its results. See [`Narrowing`].
    pub(crate) fn parameter(working_set: &StateWorkingSet<'_>, decl_id: DeclId) -> Option<Self> {
        Self::from_decl(working_set, decl_id, Narrowing::Engine)
    }

    /// A completer attached to a whole command (`@complete "nu-complete foo"`).
    pub(crate) fn command(working_set: &StateWorkingSet<'_>, decl_id: DeclId) -> Option<Self> {
        Self::from_decl(working_set, decl_id, Narrowing::Completer)
    }

    /// The configured external completer closure.
    pub(crate) fn closure(working_set: &StateWorkingSet<'_>, closure: &Closure) -> Self {
        Self {
            block_id: closure.block_id,
            captures: closure.captures.clone(),
            narrowing: Narrowing::Completer,
            interactive: closure_is_interactive(working_set, closure),
        }
    }

    /// A block-backed declaration, seeing through aliases. Builtins and plugin commands
    /// run no block and cannot serve as completers.
    fn from_decl(
        working_set: &StateWorkingSet<'_>,
        decl_id: DeclId,
        narrowing: Narrowing,
    ) -> Option<Self> {
        let block_id = (decl_id.get() < working_set.num_decls())
            .then(|| working_set.get_decl(decl_id))
            .and_then(block_of)?;

        Some(Self {
            block_id,
            captures: vec![],
            narrowing,
            interactive: decl_is_interactive(working_set, decl_id),
        })
    }

    /// Call the completer with the record it asked for; panics become `Err`
    /// so the prompt can recover instead of unwinding the worker/REPL.
    fn eval(&self, ctx: &Context) -> Result<Value, ShellError> {
        let working_set = ctx.working_set;
        let mut block = working_set.get_block(self.block_id).clone();

        // LSP completion, where a custom `def` is parsed but never compiled.
        if block.ir_block.is_none()
            && let Ok(ir_block) = compile(working_set, &block)
        {
            let mut new_block = (*block).clone();
            new_block.ir_block = Some(ir_block);
            block = Arc::new(new_block);
        }

        let mut callee_stack = ctx
            .stack
            .captures_to_stack_preserve_out_dest(self.captures.clone());

        // A completer opts into what it receives through the positional parameters it
        // declares: the input record is overloaded on them, carrying exactly the recognized
        // fields (`token`, `place`, `buffer`) it names, bound by name. Order is free. The
        // compatibility bridge retains old positional inputs for one migration cycle.
        let wanted = DeclaredInputs::from_signature(&block.signature);
        let legacy = match self.narrowing {
            Narrowing::Engine => LegacyInputs::parameter(ctx, &block),
            Narrowing::Completer => LegacyInputs::command(ctx, &block),
        };
        bind_declared_inputs(
            &mut callee_stack,
            &block.signature,
            completer_input(ctx, wanted),
            legacy,
        );

        let engine_state = engine_state_for_completion(
            working_set,
            self.block_id.get() < working_set.permanent_state.num_blocks(),
        );

        let span = ctx.span;
        match catch_completion_panic(|| {
            nu_engine::eval_block_with_early_return::<WithoutDebug>(
                engine_state.as_ref(),
                &mut callee_stack,
                &block,
                PipelineData::empty(),
            )
            .and_then(|data| data.body.into_value(span))
        }) {
            Ok(result) => result,
            Err(payload) => Err(panic_to_shell_error(&payload, span)),
        }
    }
}

/// Convert a `catch_unwind` payload into a recoverable [`ShellError`].
pub(crate) fn panic_to_shell_error(payload: &(dyn std::any::Any + Send), span: Span) -> ShellError {
    let message = if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "Box<dyn Any>".into()
    };
    ShellError::Generic(
        nu_protocol::shell_error::generic::GenericError::new("Completer panicked", message, span)
            .with_help(
                "A custom completer panicked; the panic was caught so the prompt can recover.",
            ),
    )
}

impl UserCompletion {
    /// Empty answer when the block declines (`failed: false`) or errors (`failed: true`):
    /// interactive pickers answer empty on dismiss, and engine-narrowed (parameter)
    /// completers answer empty on error since no fallback can serve their slot —
    /// anything else declines so the next source runs.
    fn empty(&self, failed: bool) -> Fetched {
        if self.interactive || (failed && matches!(self.narrowing, Narrowing::Engine)) {
            Fetched::answering(vec![])
        } else {
            Fetched::declining()
        }
    }
}

impl Completer for UserCompletion {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        match self.eval(ctx) {
            Err(err) => {
                report(format!("failed to eval completer block: {err}"));
                self.empty(true).worth_keeping()
            }
            Ok(value) => match CompleterOutput::read(value, ctx, self.narrowing) {
                None => self.empty(false),
                Some(output) => output.into_fetched(ctx),
            },
        }
    }
}
