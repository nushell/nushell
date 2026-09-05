mod input;
mod output;

pub(crate) use output::{Returned, SpanClamp, map_value_completions};
pub(crate) use output::{report, warn};

use crate::completions::{Completer, Context, Fetched};
pub use input::DeclaredInputs;
pub(crate) use input::{completer_input, legacy_context, legacy_pos, legacy_spans};
use nu_engine::compile;
use nu_protocol::{
    BlockId, DeclId, PipelineData, ShellError, Signature, Value, VarId,
    debugger::WithoutDebug,
    engine::{Closure, Command, EngineState, Stack, StateWorkingSet},
};
pub(crate) use output::CompleterOutput;
use std::{
    borrow::Cow,
    collections::HashSet,
    sync::{Arc, Mutex, OnceLock},
};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    warning_key: usize,
}

impl LegacyInputs {
    /// Construct a bridge only when the signature actually needs it, avoiding legacy parsing
    /// and allocations for completers that already use named inputs.
    fn when_needed(
        kind: LegacyInputKind,
        signature: &Signature,
        warning_key: usize,
        values: impl FnOnce() -> [Value; 2],
    ) -> Self {
        let needs_legacy = signature
            .required_positional
            .iter()
            .chain(&signature.optional_positional)
            .take(2)
            .any(|positional| !INPUT_FIELDS.contains(&positional.name.as_str()));

        let values = needs_legacy
            .then(values)
            .map(|[first, second]| [Some(first), Some(second)])
            .unwrap_or([None, None]);
        Self {
            kind,
            values,
            warning_key,
        }
    }

    pub(crate) fn parameter(ctx: &Context, signature: &Signature, block_id: BlockId) -> Self {
        Self::when_needed(
            LegacyInputKind::Parameter,
            signature,
            block_id.get(),
            || [legacy_context(ctx), legacy_pos(ctx)],
        )
    }

    pub(crate) fn command(ctx: &Context, signature: &Signature, block_id: BlockId) -> Self {
        Self::when_needed(LegacyInputKind::Command, signature, block_id.get(), || {
            // Command-wide and external completers historically received only `$spans`.
            [legacy_spans(ctx), Value::nothing(ctx.span)]
        })
    }

    pub(crate) fn menu(
        signature: &Signature,
        block_id: BlockId,
        buffer: &str,
        position: usize,
        span: nu_protocol::Span,
    ) -> Self {
        Self::when_needed(LegacyInputKind::Menu, signature, block_id.get(), || {
            [
                Value::string(buffer, span),
                Value::int(position as i64, span),
            ]
        })
    }

    fn value(&self, index: usize) -> Option<Value> {
        self.values.get(index).and_then(Clone::clone)
    }

    fn warn_once(&self, names: &[String]) {
        static WARNED: OnceLock<Mutex<HashSet<(LegacyInputKind, usize)>>> = OnceLock::new();

        let warned = WARNED.get_or_init(|| Mutex::new(HashSet::new()));
        let Ok(mut warned) = warned.lock() else {
            // A poisoned diagnostics lock must never interfere with completion.
            return;
        };
        if !warned.insert((self.kind, self.warning_key)) {
            return;
        }

        warn(format!(
            "a {} completer uses deprecated positional input{} `{}`; it still receives the \
             legacy value for compatibility, but {}",
            self.kind.name(),
            if names.len() == 1 { "" } else { "s" },
            names.join("`, `"),
            self.kind.migration(),
        ));
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
        legacy.warn_once(&legacy_names);
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
    pub(crate) fn closure(closure: &Closure) -> Self {
        Self {
            block_id: closure.block_id,
            captures: closure.captures.clone(),
            narrowing: Narrowing::Completer,
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
        })
    }

    /// Call the completer with the record it asked for.
    pub(crate) fn eval(&self, ctx: &Context) -> Result<Value, ShellError> {
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
            Narrowing::Engine => LegacyInputs::parameter(ctx, &block.signature, self.block_id),
            Narrowing::Completer => LegacyInputs::command(ctx, &block.signature, self.block_id),
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

        nu_engine::eval_block_with_early_return::<WithoutDebug>(
            engine_state.as_ref(),
            &mut callee_stack,
            &block,
            PipelineData::empty(),
        )
        .and_then(|data| data.body.into_value(ctx.span))
    }
}

impl Completer for UserCompletion {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        let value = match self.eval(ctx) {
            Ok(value) => value,
            Err(err) => {
                report(format!("failed to eval completer block: {err}"));
                // Not an empty success: an external completer failing still lets file
                // completion answer; a parameter completer failing must not dump the
                // whole directory in place of its argument.
                return match self.narrowing {
                    Narrowing::Engine => Fetched::Cacheable(vec![]),
                    Narrowing::Completer => Fetched::Declined,
                };
            }
        };

        match CompleterOutput::read(value, ctx, self.narrowing) {
            // `null` declines, letting the next source answer.
            None => Fetched::Declined,
            Some(output) => output.into_fetched(ctx),
        }
    }
}
