//! Relocation of engine-state ids while a parse delta is serialized and deserialized.
//!
//! Ids such as [`DeclId`](crate::DeclId) or [`BlockId`](crate::BlockId) and every [`Span`] are
//! indexes into tables of the [`EngineState`] that created them. Parsed programs refer to those
//! tables from hundreds of places (AST expressions, IR instructions, values, signatures, module
//! maps), so persisting a program means rewriting every one of them when it is loaded into a
//! different engine state.
//!
//! Instead of walking all of those types by hand, the serde impls of [`Id`](crate::Id) and
//! [`Span`] consult a thread-local context installed by this module:
//!
//! - [`collect_imports`] runs a serialization with an [`ImportCollector`], which records every id
//!   that is not local to the delta being written (the delta's *imports*) without changing the
//!   output.
//! - [`relocate`] runs a deserialization with an [`IdRelocator`], which shifts local ids and spans
//!   to their new position and resolves imports through a table the loader built beforehand. An
//!   import that was not resolved turns into a deserialization error, never into a guessed index.
//!
//! With no context installed (plugin protocol, `view ir --json`, ...), the hooks do nothing, and
//! while no thread has one they return after reading a single atomic.
//!
//! An id is *local* when it is at or above the size its table had before the delta was parsed
//! (see [`IdBases`]). Local ids keep their order, so relocating them is a constant shift.

use crate::{
    Span,
    engine::{EngineState, UNKNOWN_SPAN_ID},
};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::BTreeSet,
    sync::atomic::{AtomicUsize, Ordering},
};

/// The engine-state tables whose indexes a serialized parse delta can contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IdKind {
    Var,
    Decl,
    Block,
    Module,
    File,
    /// [`SpanId`](crate::SpanId), an index into the engine's span table (not a [`Span`]).
    ///
    /// Relocated programs don't carry that table: every span id maps to the unknown span id, and
    /// such an expression reports its `span` instead (see `Expression::span`).
    Span,
}

/// Connects an [`Id`](crate::Id) marker type to the table its ids index.
pub trait IdMarker {
    /// `None` for ids that are only meaningful inside their container, like the overlays of a
    /// scope frame.
    const KIND: Option<IdKind> = None;
}

/// Sizes of the engine-state tables before a delta was parsed into them.
///
/// Ids below these values refer to state outside the delta, ids at or above them to state the
/// delta created.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdBases {
    pub vars: usize,
    pub decls: usize,
    pub blocks: usize,
    pub modules: usize,
    pub files: usize,
    /// First global [`Span`] offset after the files of the engine state.
    pub span_start: usize,
}

impl IdBases {
    /// The table sizes of `engine_state`, i.e. the bases of a delta parsed on top of it.
    pub fn of(engine_state: &EngineState) -> Self {
        Self {
            vars: engine_state.num_vars(),
            decls: engine_state.num_decls(),
            blocks: engine_state.num_blocks(),
            modules: engine_state.num_modules(),
            files: engine_state.num_files(),
            span_start: engine_state.next_span_start(),
        }
    }

    /// The size of the table `kind` indexes, `None` for span ids, which aren't relocated.
    pub fn get(&self, kind: IdKind) -> Option<usize> {
        match kind {
            IdKind::Var => Some(self.vars),
            IdKind::Decl => Some(self.decls),
            IdKind::Block => Some(self.blocks),
            IdKind::Module => Some(self.modules),
            IdKind::File => Some(self.files),
            IdKind::Span => None,
        }
    }
}

/// Records what a delta refers to outside of itself while it is serialized.
#[derive(Debug)]
pub struct ImportCollector {
    bases: IdBases,
    ids: BTreeSet<(IdKind, usize)>,
}

impl ImportCollector {
    pub fn new(bases: IdBases) -> Self {
        Self {
            bases,
            ids: BTreeSet::new(),
        }
    }

    /// Record `id` if it is not local to the delta.
    pub fn note_id(&mut self, kind: IdKind, id: usize) {
        if self.bases.get(kind).is_some_and(|base| id < base) {
            self.ids.insert((kind, id));
        }
    }

    /// Every non-local id that was serialized, in `(kind, id)` order.
    pub fn imports(&self) -> impl Iterator<Item = (IdKind, usize)> + '_ {
        self.ids.iter().copied()
    }
}

/// Maps the ids of a serialized delta onto the engine state it is being loaded into.
#[derive(Debug)]
pub struct IdRelocator {
    from: IdBases,
    to: IdBases,
    /// The resolved imports, indexed by kind and then by old id. Imports are below the bases, so
    /// the tables stay as small as the engine state the delta was produced on.
    imports: Vec<Vec<Option<usize>>>,
}

impl IdRelocator {
    /// Relocate a delta produced on top of tables of size `from` onto tables of size `to`.
    pub fn new(from: IdBases, to: IdBases) -> Self {
        Self {
            from,
            to,
            imports: vec![],
        }
    }

    /// Resolve the imported id `old` to `new`.
    pub fn bind(&mut self, kind: IdKind, old: usize, new: usize) {
        let kind = kind as usize;
        if self.imports.len() <= kind {
            self.imports.resize(kind + 1, vec![]);
        }
        let table = &mut self.imports[kind];
        if table.len() <= old {
            table.resize(old + 1, None);
        }
        table[old] = Some(new);
    }

    /// The id in the target engine state for `id`, or an error if it is an unresolved import.
    pub fn map_id(&self, kind: IdKind, id: usize) -> Result<usize, String> {
        let (Some(from), Some(to)) = (self.from.get(kind), self.to.get(kind)) else {
            return Ok(UNKNOWN_SPAN_ID.get());
        };
        if id >= from {
            return to
                .checked_add(id - from)
                .ok_or_else(|| format!("{kind:?} id {id} is out of range"));
        }
        self.imports
            .get(kind as usize)
            .and_then(|table| table.get(id).copied().flatten())
            .ok_or_else(|| {
                format!("{kind:?} id {id} is outside the serialized program and was not linked")
            })
    }

    /// The span in the target engine state for `span`.
    ///
    /// The delta's own spans move with its files. Spans into files outside of it become
    /// [`Span::unknown()`].
    pub fn map_span(&self, span: Span) -> Span {
        let shift = |offset: usize| offset.checked_add(self.to.span_start);
        let local = (span.start >= self.from.span_start).then(|| {
            Some(Span {
                start: shift(span.start - self.from.span_start)?,
                end: shift(span.end.checked_sub(self.from.span_start)?)?,
            })
        });
        local.flatten().unwrap_or(Span::unknown())
    }
}

enum Context {
    Collect(ImportCollector),
    Relocate(IdRelocator),
}

thread_local! {
    static CONTEXT: RefCell<Option<Context>> = const { RefCell::new(None) };
}

/// How many contexts are installed, on all threads.
static CONTEXTS: AtomicUsize = AtomicUsize::new(0);

/// Install `context` for the duration of `f`, restoring whatever was installed before, even if
/// `f` panics. Returns the context as `f` left it.
fn with_context<T>(context: Context, f: impl FnOnce() -> T) -> (T, Option<Context>) {
    struct Restore(Option<Context>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            CONTEXT.with(|current| *current.borrow_mut() = previous);
            CONTEXTS.fetch_sub(1, Ordering::Relaxed);
        }
    }

    CONTEXTS.fetch_add(1, Ordering::Relaxed);
    let _restore = Restore(CONTEXT.with(|current| current.replace(Some(context))));
    let value = f();
    let context = CONTEXT.with(|current| current.borrow_mut().take());
    (value, context)
}

/// Run `f`, recording in `collector` every non-local id that gets serialized on this thread
/// meanwhile.
pub fn collect_imports<T>(
    collector: ImportCollector,
    f: impl FnOnce() -> T,
) -> (T, ImportCollector) {
    match with_context(Context::Collect(collector), f) {
        (value, Some(Context::Collect(collector))) => (value, collector),
        _ => unreachable!("relocation context was replaced while collecting imports"),
    }
}

/// Run `f`, relocating every id and span that gets deserialized on this thread meanwhile.
pub fn relocate<T>(relocator: IdRelocator, f: impl FnOnce() -> T) -> (T, IdRelocator) {
    match with_context(Context::Relocate(relocator), f) {
        (value, Some(Context::Relocate(relocator))) => (value, relocator),
        _ => unreachable!("relocation context was replaced while relocating"),
    }
}

/// Serialization hook: record `id` if an [`ImportCollector`] is active.
///
/// Also used for ids that are stored untyped (as plain integers), which the serde hooks cannot
/// see.
pub fn note_id(kind: IdKind, id: usize) {
    if CONTEXTS.load(Ordering::Relaxed) == 0 {
        return;
    }
    CONTEXT.with(|context| {
        if let Ok(mut context) = context.try_borrow_mut()
            && let Some(Context::Collect(collector)) = context.as_mut()
        {
            collector.note_id(kind, id);
        }
    });
}

/// Deserialization hook: relocate `id` if an [`IdRelocator`] is active.
pub(crate) fn map_id(kind: IdKind, id: usize) -> Result<usize, String> {
    if CONTEXTS.load(Ordering::Relaxed) == 0 {
        return Ok(id);
    }
    CONTEXT.with(|context| match context.try_borrow().as_deref() {
        Ok(Some(Context::Relocate(relocator))) => relocator.map_id(kind, id),
        _ => Ok(id),
    })
}

/// Deserialization hook for [`Span`], see [`map_id`].
pub(crate) fn map_span(span: Span) -> Span {
    if CONTEXTS.load(Ordering::Relaxed) == 0 {
        return span;
    }
    CONTEXT.with(|context| match context.try_borrow().as_deref() {
        Ok(Some(Context::Relocate(relocator))) => relocator.map_span(span),
        _ => span,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlockId, DeclId, RegId, VarId};

    fn bases(n: usize, span_start: usize) -> IdBases {
        IdBases {
            vars: n,
            decls: n,
            blocks: n,
            modules: n,
            files: n,
            span_start,
        }
    }

    #[test]
    fn collects_only_non_local_ids() {
        let data = (
            DeclId::new(3),
            DeclId::new(12),
            VarId::new(1),
            RegId::new(2),
            Span::new(12, 14),
        );
        let (json, collector) = collect_imports(ImportCollector::new(bases(10, 30)), || {
            serde_json::to_string(&data)
        });
        let imports: Vec<_> = collector.imports().collect();
        assert_eq!(imports, vec![(IdKind::Var, 1), (IdKind::Decl, 3)]);
        // collecting never changes the output
        assert_eq!(json.unwrap(), serde_json::to_string(&data).unwrap());
    }

    #[test]
    fn relocates_local_ids_and_resolves_imports() {
        let json = serde_json::to_string(&(
            DeclId::new(3),
            DeclId::new(12),
            BlockId::new(10),
            RegId::new(12),
            Span::new(12, 14),
            Span::new(40, 45),
        ))
        .unwrap();

        let mut relocator = IdRelocator::new(bases(10, 30), bases(20, 100));
        relocator.bind(IdKind::Decl, 3, 7);
        let (value, _) = relocate(relocator, || {
            serde_json::from_str::<(DeclId, DeclId, BlockId, RegId, Span, Span)>(&json)
        });
        assert_eq!(
            value.unwrap(),
            (
                DeclId::new(7),
                DeclId::new(22),
                BlockId::new(20),
                RegId::new(12),
                Span::unknown(),
                Span::new(110, 115)
            )
        );
    }

    #[test]
    fn unresolved_import_is_an_error() {
        let json = serde_json::to_string(&DeclId::new(3)).unwrap();
        let (value, _) = relocate(IdRelocator::new(bases(10, 0), bases(20, 0)), || {
            serde_json::from_str::<DeclId>(&json)
        });
        assert!(value.is_err());
        // outside of `relocate`, ids deserialize unchanged
        assert_eq!(
            serde_json::from_str::<DeclId>(&json).unwrap(),
            DeclId::new(3)
        );
    }
}
