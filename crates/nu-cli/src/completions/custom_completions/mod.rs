mod input;
mod output;

pub use input::InputShape;
pub(crate) use input::declared_shape;
use output::report;
pub(crate) use output::{Returned, SpanClamp, map_value_completions};

use crate::completions::{Completer, Context, Fetched};
pub(crate) use input::completer_input;
use nu_engine::compile;
use nu_protocol::{
    BlockId, DeclId, PipelineData, ShellError, Signature, Value, VarId,
    debugger::WithoutDebug,
    engine::{Closure, Command, EngineState, Stack, StateWorkingSet},
};
pub(crate) use output::CompleterOutput;
use std::{borrow::Cow, sync::Arc};

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
pub(crate) const INPUT_FIELDS: [&str; 3] = ["token", "place", "contexts"];

/// Bind declared positional names to matching fields in the input record.
pub(crate) fn bind_declared_inputs(stack: &mut Stack, signature: &Signature, input: Value) {
    let span = input.span();
    let Ok(record) = input.into_record() else {
        return;
    };

    for positional in signature
        .required_positional
        .iter()
        .chain(&signature.optional_positional)
    {
        if let Some(var_id) = positional.var_id {
            if !INPUT_FIELDS.contains(&positional.name.as_str()) {
                log::warn!(
                    "a completer declares `{}`, which is not a completion input; expected one \
                     of {} — it will receive nothing",
                    positional.name,
                    INPUT_FIELDS.join(", ")
                );
            }

            let value = record
                .get(positional.name.as_str())
                .cloned()
                .unwrap_or_else(|| Value::nothing(span));
            stack.add_var(var_id, value);
        }
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
        // declares: each is bound by name to the like-named field of the input record
        // (`token`, `place`, or `contexts`). Order is free and an unrecognized name simply
        // receives nothing. Declaring a `contexts` parameter selects the larger record.
        let shape = declared_shape(&block.signature);
        bind_declared_inputs(
            &mut callee_stack,
            &block.signature,
            completer_input(ctx, shape),
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
