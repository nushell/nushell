//! The record handed to user completers, in two shapes; see [`InputShape`].
//!
//! A completer asks for the larger one by declaring a `--full` flag in its own parameter
//! list. That reads the same on a `def` and on a closure, which matters because the external
//! completer is configured as a closure and so has nowhere to hang an attribute:
//!
//! ```nu
//! def "nu-complete deep" [input: record, --full] { ... }
//! $env.config.completions.external.completer = {|input, --full| ... }
//! ```
//!
//! # `{token, place}`, the default
//!
//! `token` is the one token being completed — `{text, kind, span}` — and `place` is where in
//! the line that is: `cursor`, a byte offset; `target`, the `{start, end}` byte range a
//! suggestion replaces; and the resolution (`kind`, plus `flag`/`index`). `target` is worth
//! having separately from `token.span`, because the two differ wherever a completion spans
//! more than one token: a multiword command head, or a cell path.
//!
//! This is deliberately the floor. Of 421 completers in `nu_scripts`, 405 read no input at
//! all, so almost nothing pays for more than this.
//!
//! # `{contexts, place}`, with `--full`
//!
//! `contexts` is the command the cursor is in, as `{tokens}`. A token is
//! `{text, kind, span, nested}`; row 0 is the command name. Where the cursor is inside a
//! closure or subexpression, the token that *is* that expression carries the command within
//! it as `nested` — a context of its own, tagged with the slot it fills in its parent
//! (`each { ⌶ }` nests under `{kind: positional, index: 0}`). So `contexts` nests exactly as
//! far as the cursor does and no further: a command beside the cursor, whether an earlier
//! statement or an upstream pipe, never appears, and no text is repeated between levels.
//!
//! `place` keeps the same three fields, only as walks of the tree rather than byte offsets:
//! `cursor` is where the cursor is, and `target.start`/`target.end` bound what a suggestion
//! replaces. The resolution stays where it is by default, so `$input.place.kind` reads the
//! same either way and only the coordinates change.
//!
//! A walk is `{path, byte}`: `path` indexes a token per level, descending through `nested`,
//! and `byte` is the offset within the token the last index names. A target may start or end
//! mid-token, so a completer replacing part of one need not round out to a token boundary. A
//! walk is null where the offset falls on no token at all, rather than naming one it isn't on.
//!
//! Everything is post-resolution: aliases are expanded, and a token the expansion produced
//! has a null `span`, because it is not on the line. Byte offsets index the commandline; a
//! completer never sees text past the cursor.
//!
//! Menu sources receive the same record, and declare `--full` the same way.
//!
//! # What comes back
//!
//! A list of suggestions is the whole answer; a record of `{completions, options?}` carries
//! settings beside them; `null` declines, letting the next source answer. A lone suggestion
//! record works too — a record is the envelope only when it names `completions` or `options`.
//!
//! It is read part by part, and a part that does not fit costs that part alone: a malformed
//! `options` keeps the completions returned beside it, a malformed `span` keeps the
//! suggestion around it, and one bad suggestion keeps the rest of the list. Every one of
//! those is reported. Unknown keys are reported rather than ignored, so a misspelling is not
//! silence.

use crate::completions::{
    Completer, CompletionOptions, Context, Fetched, MatchAlgorithm, NuMatcher, SemanticSuggestion,
    completer::{CompletionContext, is_flag_text},
    to_reedline_span,
};
use nu_color_config::{color_record_to_nustyle, lookup_ansi_color_style};
use nu_engine::compile;
use nu_parser::{FlatShape, flatten_expression};
use nu_protocol::{
    BlockId, DeclId, Flag, FromValue, PipelineData, Record, ShellError, Signature, Span,
    SuggestionKind, Type, Value, VarId,
    debugger::WithoutDebug,
    engine::{Closure, Command, EngineState, StateWorkingSet},
    shell_error::generic::GenericError,
};
use nu_utils::strip_ansi_string_unlikely;
use reedline::Suggestion;
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

/// A `{start, end}` record of byte offsets into the commandline.
fn span_record(start: usize, end: usize, span: Span) -> Value {
    Value::record(
        Record::from_iter([
            ("start".into(), Value::int(start as i64, span)),
            ("end".into(), Value::int(end as i64, span)),
        ]),
        span,
    )
}

/// One token of a context's command.
struct Token {
    /// The typed text; a token the cursor is inside is cut there. Absent for a nesting
    /// token, whose text is the command it holds as `nested`.
    text: Option<String>,
    /// `head` (the command name), `flag` (`--x`, `-x`), `block` (a nesting expression), or
    /// `value`.
    kind: &'static str,
    /// Byte range the token occupies on the line. Absent for alias-expanded tokens, which
    /// are not on it.
    at: Option<(usize, usize)>,
}

impl Token {
    /// A token spanning `start..end` of the line.
    fn on_line(buffer: &str, (start, end): (usize, usize), first: bool, flag_shape: bool) -> Self {
        let text = buffer.get(start..end).unwrap_or_default().to_string();
        Self {
            kind: Self::classify(&text, first, flag_shape),
            text: Some(text),
            at: Some((start, end)),
        }
    }

    /// A token an alias expansion produced: part of the command, but not on the line.
    fn expanded(text: String, first: bool, flag_shape: bool) -> Self {
        Self {
            kind: Self::classify(&text, first, flag_shape),
            text: Some(text),
            at: None,
        }
    }

    /// The closure or subexpression the cursor descends into. Its text is left out: it is
    /// the command carried in `nested`, which would otherwise be repeated at every level.
    fn nesting((start, end): (usize, usize)) -> Self {
        Self {
            text: None,
            kind: "block",
            at: Some((start, end)),
        }
    }

    /// `head` for the command name, else `flag` for a dash-led token and `value` otherwise.
    fn classify(text: &str, first: bool, flag_shape: bool) -> &'static str {
        if first {
            "head"
        } else if flag_shape || is_flag_text(text) {
            "flag"
        } else {
            "value"
        }
    }

    /// The columns every token has. Each is always present: a ragged table errors on the
    /// rows that have nothing, where `null` reads fine.
    fn columns(&self, span: Span) -> Record {
        Record::from_iter([
            (
                "text".into(),
                self.text
                    .as_ref()
                    .map_or_else(|| Value::nothing(span), |text| Value::string(text, span)),
            ),
            ("kind".into(), Value::string(self.kind, span)),
            (
                "span".into(),
                self.at.map_or_else(
                    || Value::nothing(span),
                    |(start, end)| span_record(start, end, span),
                ),
            ),
        ])
    }

    /// A row of the default `tokens` table.
    fn to_value(&self, span: Span) -> Value {
        Value::record(self.columns(span), span)
    }

    /// A row of the full form's table, which adds the context this token holds, if any.
    fn to_nested_value(&self, span: Span, nested: Option<Value>) -> Value {
        let mut record = self.columns(span);
        record.insert("nested", nested.unwrap_or_else(|| Value::nothing(span)));
        Value::record(record, span)
    }
}

/// The tokens of one context's command. A context the cursor only passes through stops at
/// the nesting expression it descends into, which becomes the last token; the innermost one
/// runs to the cursor and always carries a token at `replacing`.
fn context_tokens(ctx: &Context, level: &CompletionContext) -> Vec<Token> {
    let cursor = ctx.buffer.len();
    let element = level.element.map(|element| element.span);
    let mut tokens: Vec<Token> = Vec::new();

    let flattened = level
        .element
        .map(|element| flatten_expression(ctx.working_set, element))
        .unwrap_or_default();

    // The span of the token flattening last yielded, to drop the repeats below.
    let mut previous = None;

    for (token_span, shape) in &flattened {
        // Everything from the descent on belongs to the contexts nested inside this one.
        if level
            .descent
            .is_some_and(|descent| token_span.start >= descent.start)
        {
            break;
        }

        // Flattening yields the nodes the parser synthesized alongside the ones that were
        // typed: a missing argument is a zero-width span, and a row condition's implicit row
        // variable repeats the span of the member beside it. Neither is a token to complete.
        if token_span.start >= token_span.end || previous == Some(*token_span) {
            continue;
        }
        previous = Some(*token_span);

        // Nor is whitespace. A closure flattens to its delimiters and the padding around
        // them (`{ `, and the gap before `}`), so without this the token under a cursor
        // sitting in `each { ls ⌶` is the space itself.
        if ctx
            .working_set
            .get_span_contents(*token_span)
            .iter()
            .all(u8::is_ascii_whitespace)
        {
            continue;
        }

        // On the line only if inside this command's span; alias tokens point at the definition.
        let on_line = token_span.start >= ctx.offset
            && element.is_some_and(|element| {
                token_span.start >= element.start && token_span.end <= element.end
            });

        let first = tokens.is_empty();
        let flag_shape = matches!(shape, FlatShape::Flag);

        tokens.push(if on_line {
            let start = token_span.start - ctx.offset;
            // Never text past the cursor, though the LSP parses whole files.
            if start >= cursor {
                continue;
            }
            let end = (token_span.end - ctx.offset).min(cursor);
            Token::on_line(ctx.buffer, (start, end), first, flag_shape)
        } else {
            let text = ctx.working_set.get_span_contents(*token_span);
            Token::expanded(
                String::from_utf8_lossy(text).into_owned(),
                first,
                flag_shape,
            )
        });
    }

    // The nesting expression closes an enclosing context: it is the last token, and the
    // context below hangs off it.
    if let Some(descent) = level.descent {
        let start = descent.start.saturating_sub(ctx.offset);
        let end = descent.end.saturating_sub(ctx.offset).min(cursor);
        tokens.push(Token::nesting((start, end)));
        return tokens;
    }

    // The cursor can rest on no token at all: an empty slot (a trailing argument, a bare
    // `--`), or punctuation the flattener emits nothing for (the dot of `$env.⌶`). Add one
    // running to the cursor, so a walk always lands on a real token.
    if locate(&tokens, cursor, Side::Trailing).is_none() {
        let last_end = tokens
            .iter()
            .filter_map(|token| token.at)
            .map(|(_, end)| end)
            .max()
            .unwrap_or(0);

        // Never back over a token already listed: one that overlapped would put the same
        // bytes on the line twice, and out of order at that.
        let replacing = to_reedline_span(ctx.span, ctx.offset);
        let start = replacing.start.max(last_end).min(cursor);
        let first = tokens.is_empty();
        tokens.push(Token::on_line(ctx.buffer, (start, cursor), first, false));
    }

    tokens
}

/// Which side of a token boundary a byte offset belongs to when it falls exactly between
/// two tokens.
#[derive(Clone, Copy)]
enum Side {
    /// The token starting there: a range's start.
    Leading,
    /// The token ending there: a range's end, and the cursor.
    Trailing,
}

/// The token `offset` falls in, and its byte offset within that token.
fn locate(tokens: &[Token], offset: usize, side: Side) -> Option<(usize, usize)> {
    let holds = |token: &Token| {
        let (start, end) = token.at?;
        (start <= offset && offset <= end).then_some((start, end))
    };

    let mut candidates = tokens.iter().enumerate();
    let found = match side {
        Side::Leading => candidates.find_map(|(index, token)| Some((index, holds(token)?))),
        Side::Trailing => candidates
            .rev()
            .find_map(|(index, token)| Some((index, holds(token)?))),
    };

    found.map(|(index, (start, _))| (index, offset - start))
}

/// A `{path, byte}` walk: `path` indexes one token per level, descending through `nested`,
/// and `byte` is the offset within the token the last index names. `descents` is the path
/// down to the context `at` is in.
///
/// Null where the offset falls on no token — `$env.⌶`, whose dot belongs to no token of its
/// own. A completer can then tell "nowhere" from a real position, which a walk pointing at
/// some token it isn't on would hide.
fn walk_value(descents: &[usize], at: Option<(usize, usize)>, span: Span) -> Value {
    let Some((token, byte)) = at else {
        return Value::nothing(span);
    };

    let path = descents
        .iter()
        .copied()
        .chain([token])
        .map(|index| Value::int(index as i64, span))
        .collect();

    Value::record(
        Record::from_iter([
            ("path".into(), Value::list(path, span)),
            ("byte".into(), Value::int(byte as i64, span)),
        ]),
        span,
    )
}

/// Nest each level's tokens into the one above, building from the innermost out. A level's
/// nesting token is its last, and the context hanging off it is tagged with the slot it
/// fills in that level.
fn contexts_value(ctx: &Context, levels: &[Vec<Token>]) -> Value {
    let span = ctx.span;
    let mut nested: Option<Value> = None;

    for (level, tokens) in levels.iter().enumerate().rev() {
        let last = tokens.len().saturating_sub(1);
        let mut inner = nested.take();
        let rows = tokens
            .iter()
            .enumerate()
            .map(|(index, token)| {
                token.to_nested_value(span, (index == last).then(|| inner.take()).flatten())
            })
            .collect();

        // The outermost context fills no slot: nothing encloses it.
        let mut record = match level.checked_sub(1) {
            Some(parent) => ctx.contexts[parent].cursor.into_record(span),
            None => Record::new(),
        };
        record.insert("tokens", Value::list(rows, span));
        nested = Some(Value::record(record, span));
    }

    nested.unwrap_or_else(|| Value::record(Record::new(), span))
}

/// Where the completion happens: the cursor, the range a suggestion replaces, and what the
/// cursor resolved to. Both shapes build it here, so the resolution sits in the same place
/// in each and only `cursor`/`target` change coordinates. See the [module docs](self).
fn place_value(ctx: &Context, cursor: Value, target: Value) -> Value {
    let span = ctx.span;
    let mut place = Record::from_iter([("cursor".into(), cursor), ("target".into(), target)]);

    if let Some(level) = ctx.contexts.last() {
        place.extend(level.cursor.into_record(span));
    }

    Value::record(place, span)
}

/// Which record a completer receives. Almost every completer reads no input at all, so the
/// default is the floor and everything else is opt-in; see the [module docs](self).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputShape {
    /// `{token, place}`: the one token being completed, and where it is.
    Token,
    /// `{contexts, place}`: the nesting tree, and walks into it.
    Full,
}

impl InputShape {
    /// The shape a `--full` opt-in selects, however that opt-in was expressed: a flag a
    /// completer declares, or one typed at `commandline complete`.
    pub fn from_full(full: bool) -> Self {
        if full { Self::Full } else { Self::Token }
    }
}

/// Build the record a user completer receives; see the [module docs](self).
pub(crate) fn completer_input(ctx: &Context, shape: InputShape) -> Value {
    match shape {
        InputShape::Token => token_input(ctx),
        InputShape::Full => full_input(ctx),
    }
}

/// The default `{token, place}` record. Only the command the cursor is in is flattened, and
/// only the token under the cursor survives; the nesting around it is never walked.
fn token_input(ctx: &Context) -> Value {
    let span = ctx.span;
    let replacing = to_reedline_span(ctx.span, ctx.offset);
    let tokens = ctx
        .contexts
        .last()
        .map(|level| context_tokens(ctx, level))
        .unwrap_or_default();

    // The same token `--full` would land `place.cursor` on. The cursor can sit past every
    // token — `$env.⌶`, where the dot is no token of its own — so fall back to the last.
    let token = locate(&tokens, ctx.buffer.len(), Side::Trailing)
        .map(|(index, _)| index)
        .or_else(|| tokens.len().checked_sub(1))
        .and_then(|index| tokens.get(index))
        .map_or_else(|| Value::nothing(span), |token| token.to_value(span));

    Value::record(
        Record::from_iter([
            ("token".into(), token),
            (
                "place".into(),
                place_value(
                    ctx,
                    Value::int(ctx.buffer.len() as i64, span),
                    span_record(replacing.start, replacing.end, span),
                ),
            ),
        ]),
        span,
    )
}

/// The `--full` `{contexts, place}` record; see the [module docs](self).
fn full_input(ctx: &Context) -> Value {
    let span = ctx.span;
    let levels: Vec<Vec<Token>> = ctx
        .contexts
        .iter()
        .map(|level| context_tokens(ctx, level))
        .collect();

    // Every level but the last nests through its own last token.
    let descents: Vec<usize> = levels
        .iter()
        .rev()
        .skip(1)
        .rev()
        .map(|tokens| tokens.len().saturating_sub(1))
        .collect();

    let innermost = levels.last().map_or(&[][..], Vec::as_slice);
    let replacing = to_reedline_span(ctx.span, ctx.offset);
    let walk = |offset, side| walk_value(&descents, locate(innermost, offset, side), span);

    let target = Value::record(
        Record::from_iter([
            ("start".into(), walk(replacing.start, Side::Leading)),
            ("end".into(), walk(replacing.end, Side::Trailing)),
        ]),
        span,
    );

    Value::record(
        Record::from_iter([
            ("contexts".into(), contexts_value(ctx, &levels)),
            (
                "place".into(),
                place_value(ctx, walk(ctx.buffer.len(), Side::Trailing), target),
            ),
        ]),
        span,
    )
}

/// Which record a block asked for, and the `--full` flag to bind so it can read `$full`.
/// Read off the block's own signature, so a closure — a menu source, or the external
/// completer — opts in exactly as a `def` does.
pub(crate) fn declared_shape(signature: &Signature) -> (InputShape, Option<&Flag>) {
    let full = signature
        .named
        .iter()
        .find(|flag| flag.long == "full" && flag.arg.is_none());

    (InputShape::from_full(full.is_some()), full)
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

        // Declaring `--full` both selects the larger record and binds the flag, so the
        // completer can read `$full` like any other parameter.
        let (shape, full) = declared_shape(&block.signature);
        if let Some(var_id) = full.and_then(|flag| flag.var_id) {
            callee_stack.add_var(var_id, Value::bool(true, ctx.span));
        }

        if let Some(var_id) = block
            .signature
            .get_positional(0)
            .and_then(|positional| positional.var_id)
        {
            callee_stack.add_var(var_id, completer_input(ctx, shape));
        }

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
                log::error!(
                    "{}",
                    ShellError::Generic(
                        GenericError::new_internal(
                            "nu::shell::completion",
                            "failed to eval completer block",
                        )
                        .with_inner([err]),
                    )
                );
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

/// A completer's return value, after both accepted shapes are normalized.
struct CompleterOutput {
    suggestions: Vec<SemanticSuggestion>,
    options: CompletionOptions,
    sort: bool,
    filter: bool,
}

/// Report something a completer returned that could not be used. Nothing here is fatal —
/// the rest of the return still stands — so the message is the only way a completer author
/// learns of it.
fn report(message: impl Into<String>) {
    log::error!(
        "{}",
        ShellError::Generic(GenericError::new_internal(
            "nu::shell::completion",
            message.into(),
        ))
    );
}

/// Read one part of what a completer returned, naming it when it does not fit. A part that
/// fails costs that part alone: a bad `options` keeps the completions returned beside it, a
/// bad `span` keeps the suggestion around it.
fn read_part<T: FromValue>(value: Value, what: &str) -> Option<T> {
    T::from_value(value)
        .map_err(|err| {
            log::error!(
                "{}",
                ShellError::Generic(
                    GenericError::new_internal(
                        "nu::shell::completion",
                        format!("a completer's {what} is not usable"),
                    )
                    .with_inner([err]),
                )
            );
        })
        .ok()
}

/// A string field read leniently: anything that coerces to a string becomes one, so
/// `{description: 5}` reads as `"5"` rather than costing the suggestion it describes.
struct Text(String);

impl FromValue for Text {
    fn from_value(value: Value) -> Result<Self, ShellError> {
        value.coerce_into_string().map(Self)
    }

    fn expected_type() -> Type {
        Type::String
    }
}

impl From<Text> for String {
    fn from(text: Text) -> Self {
        text.0
    }
}

/// A record a completer returns, read strictly: only the keys it declares are accepted, so
/// a misspelled one is reported rather than silently doing nothing. Implemented solely by
/// [`returned_record!`], which derives [`Self::KEYS`] from the same fields [`FromValue`]
/// reads, so the two cannot drift apart.
trait ReturnedRecord: FromValue {
    /// What to call it in a message.
    const LABEL: &'static str;
    /// Every key accepted, whether read into a field or merely tolerated.
    const KEYS: &'static [&'static str];

    /// Read one, reporting an unknown key or a value that does not fit.
    fn read(value: Value) -> Option<Self> {
        if let Ok(record) = value.as_record()
            && let Some(unknown) = record
                .columns()
                .find(|column| !Self::KEYS.contains(&column.as_str()))
        {
            report(format!(
                "a {} has no `{unknown}` field; expected one of {}",
                Self::LABEL,
                Self::KEYS.join(", ")
            ));
            return None;
        }

        read_part(value, Self::LABEL)
    }
}

/// Declare a record a completer returns: the struct [`FromValue`] reads, and the exact set
/// of keys it accepts. `accepting` names keys tolerated but not read.
macro_rules! returned_record {
    (
        $(#[$doc:meta])*
        struct $name:ident is $label:literal $(, accepting [$($extra:literal),* $(,)?])? {
            $( $(#[$field_doc:meta])* $field:ident : $ty:ty ),* $(,)?
        }
    ) => {
        $(#[$doc])*
        #[derive(FromValue)]
        #[nu_value(type_name = $label)]
        struct $name {
            $( $(#[$field_doc])* $field: $ty, )*
        }

        impl ReturnedRecord for $name {
            const LABEL: &'static str = $label;
            const KEYS: &'static [&'static str] =
                &[$(stringify!($field),)* $($($extra),*)?];
        }
    };
}

returned_record! {
    /// The `options` a completer may return beside its completions. Every key is optional;
    /// one left out keeps the engine's configured setting.
    struct ReturnedOptions is "completion options" {
        /// Whether the engine narrows the list against the typed text. Defaults by completer
        /// kind; see [`Narrowing`].
        filter: Option<bool>,
        sort: Option<bool>,
        case_sensitive: Option<bool>,
        match_description: Option<bool>,
        completion_algorithm: Option<String>,
        /// Deprecated; use the substring match algorithm.
        positional: Option<bool>,
    }
}

returned_record! {
    /// One suggestion, as every completer and menu source returns it. Declared rather than
    /// read key by key, so a mistyped field is reported, not a silent default.
    struct ReturnedSuggestion is "suggestion", accepting ["kind", "type"] {
        /// Coerced to a string to insert; its type is reported back as the suggestion's kind.
        value: Value,
        display_override: Option<Text>,
        description: Option<Text>,
        /// A colour name, or a colour record. Read on its own, so an unusable one costs the
        /// colour rather than the suggestion.
        style: Option<Value>,
        /// Read on its own for the same reason as `style`.
        span: Option<Value>,
        /// Extra columns beside the value, in description menus.
        extra: Option<Vec<Text>>,
    }
}

returned_record! {
    /// A `span` a suggestion returned. Each end is optional and falls back to the span the
    /// engine would have replaced, so `{end: 20}` moves only the end (#tested by
    /// `custom_completions_override_span`).
    struct ReturnedSpan is "suggestion span" {
        start: Option<i64>,
        end: Option<i64>,
    }
}

/// What a completer or menu source returned, before any options are applied. See the
/// [module docs](self) for the shapes accepted.
pub(crate) struct Returned {
    pub(crate) completions: Vec<Value>,
    options: Option<ReturnedOptions>,
}

impl Returned {
    /// `None` where the source declined with `null`, letting the next one answer.
    pub(crate) fn read(value: Value) -> Option<Self> {
        let bare = |completions| {
            Some(Self {
                completions,
                options: None,
            })
        };

        // A record is the envelope only when it names one of the two keys; any other record
        // is a lone suggestion, which is what a menu source returning one has always meant.
        let is_envelope = match &value {
            Value::Record { val, .. } => val.contains("completions") || val.contains("options"),
            _ => false,
        };

        if is_envelope {
            let mut record = value.into_record().unwrap_or_default();
            // Read the halves apart, so a bad `options` costs the settings it names and not
            // every completion returned beside it.
            return Some(Self {
                completions: record
                    .remove("completions")
                    .and_then(|completions| read_part(completions, "`completions`"))
                    .unwrap_or_default(),
                options: record.remove("options").and_then(ReturnedOptions::read),
            });
        }

        match value {
            Value::Nothing { .. } => None,
            Value::List { vals, .. } => bare(vals.into_owned()),
            record @ Value::Record { .. } => bare(vec![record]),
            // A list is many suggestions and a record is one, so a bare value is neither —
            // a mistake, not a shorthand. Its elements would have coerced inside a list.
            other => {
                report(format!(
                    "a completer must return a list of completions, a record of \
                     {{completions, options?}} or one suggestion, or null to decline; got {}",
                    other.get_type()
                ));
                bare(Vec::new())
            }
        }
    }
}

impl CompleterOutput {
    /// Read what a completer returned; `None` when it declined with `null`.
    fn read(value: Value, ctx: &Context, narrowing: Narrowing) -> Option<Self> {
        let returned = Returned::read(value)?;

        let mut output = Self {
            suggestions: map_value_completions(
                returned.completions.iter(),
                to_reedline_span(ctx.span, ctx.offset),
                SpanClamp::within(ctx.buffer),
            ),
            options: ctx.options.clone(),
            sort: true,
            filter: narrowing.filters_by_default(),
        };

        if let Some(options) = returned.options {
            output.read_options(options);
        }

        Some(output)
    }

    /// Apply the `options` record a completer returned alongside its completions.
    fn read_options(&mut self, options: ReturnedOptions) {
        if let Some(filter) = options.filter {
            self.filter = filter;
        }

        if let Some(sort) = options.sort {
            self.sort = sort;
            if self.sort && !self.filter {
                log::warn!("Sorting won't happen because filtering is disabled.");
            }
        }

        if let Some(case_sensitive) = options.case_sensitive {
            self.options.case_sensitive = case_sensitive;
        }

        if let Some(match_description) = options.match_description {
            self.options.match_description = match_description;
        }

        if options.positional.is_some() {
            log::warn!(
                "Use of the positional option is deprecated. Use the substring match algorithm instead."
            );
        }

        if let Some(algorithm) = options
            .completion_algorithm
            .and_then(|algorithm| MatchAlgorithm::try_from(algorithm).ok())
        {
            self.options.match_algorithm = algorithm;
            if let Some(false) = options.positional
                && self.options.match_algorithm == MatchAlgorithm::Prefix
            {
                self.options.match_algorithm = MatchAlgorithm::Substring;
            }
        }
    }

    /// Narrow the suggestions against the typed prefix, unless the completer already did.
    fn into_fetched(self, ctx: &Context) -> Fetched {
        if !self.filter {
            return Fetched::Cacheable(self.suggestions);
        }

        let prefix = ctx.prefix_str();
        let mut matcher = NuMatcher::new(prefix.as_ref(), &self.options, self.sort);

        for suggestion in self.suggestions {
            let value =
                strip_ansi_string_unlikely(suggestion.suggestion.display_value().to_string());
            if matcher.check_match(&value).is_some() {
                matcher.add(value, suggestion);
            } else if self.options.match_description
                && let Some(description) = suggestion.suggestion.description.as_deref()
                && matcher.check_match(description).is_some()
            {
                let description = description.to_string();
                matcher.add(description, suggestion);
            }
        }

        Fetched::Cacheable(matcher.suggestion_results())
    }
}

/// The colour a suggestion asked for: a name (`"green"`, `"bg_red"`), or a record of
/// `{fg, bg, attr}`.
fn style_of(style: Value) -> Option<nu_ansi_term::Style> {
    match style {
        Value::String { val, .. } => Some(lookup_ansi_color_style(&val)),
        Value::Record { .. } => Some(color_record_to_nustyle(&style)),
        other => {
            report(format!(
                "a suggestion's `style` must be a colour name or a record of {{fg, bg, attr}}; \
                 got {}",
                other.get_type()
            ));
            None
        }
    }
}

impl ReturnedSuggestion {
    /// Resolve against the span a suggestion replaces when it names none.
    fn into_semantic(
        self,
        default_span: reedline::Span,
        clamp: SpanClamp<'_>,
    ) -> SemanticSuggestion {
        let kind = SuggestionKind::Value(self.value.get_type());

        SemanticSuggestion {
            suggestion: Suggestion {
                value: self
                    .value
                    .coerce_string()
                    .map(strip_ansi_string_unlikely)
                    .unwrap_or_default(),
                display_override: self.display_override.map(Into::into),
                description: self.description.map(Into::into),
                style: self.style.and_then(style_of),
                span: self
                    .span
                    .and_then(ReturnedSpan::read)
                    .map_or(default_span, |span| span.resolve(default_span, clamp)),
                extra: self
                    .extra
                    .map(|extra| extra.into_iter().map(Into::into).collect()),
                ..Suggestion::default()
            },
            kind: Some(kind),
        }
    }
}

impl ReturnedSpan {
    /// Clamp so a returned span can't index out of bounds or mid-character (#5127), nor
    /// replace text the completer was never shown.
    fn resolve(self, default: reedline::Span, clamp: SpanClamp<'_>) -> reedline::Span {
        let offset = |value: Option<i64>, fallback: usize| match value {
            // A negative offset names no position on the line. Keeping the default is the
            // safe reading; clamping it to `0` would silently replace from the line's start.
            Some(value) if value < 0 => {
                report(format!("a suggestion's span offset ({value}) is negative"));
                fallback
            }
            Some(value) => clamp.apply(value as usize),
            None => fallback,
        };

        let start = offset(self.start, default.start);
        let end = offset(self.end, default.end);

        if start > end {
            report(format!(
                "a suggestion's span start ({start}) is greater than its end ({end})"
            ));
            return reedline::Span::new(end, end);
        }

        reedline::Span::new(start, end)
    }
}

/// Convert a completer's values into suggestions. A record is a [`ReturnedSuggestion`];
/// anything else is the value itself, so `[a b]` and `[1 2 3]` both work. `default_span` is
/// what a suggestion naming no `span` replaces. Takes the span and buffer directly rather
/// than a [`Context`] so menu sources share it.
pub(crate) fn map_value_completions<'a>(
    list: impl Iterator<Item = &'a Value>,
    default_span: reedline::Span,
    clamp: SpanClamp<'_>,
) -> Vec<SemanticSuggestion> {
    list.filter_map(move |value| {
        if matches!(value, Value::Record { .. }) {
            return ReturnedSuggestion::read(value.clone())
                .map(|suggestion| suggestion.into_semantic(default_span, clamp));
        }

        let kind = SuggestionKind::Value(value.get_type());
        value.coerce_string().ok().map(|string| SemanticSuggestion {
            suggestion: Suggestion {
                value: strip_ansi_string_unlikely(string),
                span: default_span,
                ..Suggestion::default()
            },
            kind: Some(kind),
        })
    })
    .collect()
}

/// How far a returned span may reach, and against which text. Completers clamp to the
/// buffer they saw; menu sources to the cursor, with no text to snap to a char boundary.
#[derive(Clone, Copy)]
pub(crate) struct SpanClamp<'a> {
    pub limit: usize,
    pub text: Option<&'a str>,
}

impl<'a> SpanClamp<'a> {
    /// Clamped to `text`, whose length is the bound.
    pub(crate) fn within(text: &'a str) -> Self {
        Self {
            limit: text.len(),
            text: Some(text),
        }
    }

    /// Bounded by an offset with no text to align against.
    pub(crate) fn upto(limit: usize) -> Self {
        Self { limit, text: None }
    }

    pub(crate) fn apply(&self, value: usize) -> usize {
        let value = value.min(self.limit);
        self.text.map_or(value, |text| {
            text.floor_char_boundary(value.min(text.len()))
        })
    }
}
