//! The engine's declarations, as the winnow parser asks about them.

use std::{cell::RefCell, rc::Rc};

use nu_protocol::{
    DeclId, SyntaxShape,
    ast::Expr,
    engine::{CommandType, OverlayFrame, ScopeFrame, StateWorkingSet, longest_decl_name},
};
use nu_winnow_parser::{CommandLookup, DeclKind};

/// Answers the winnow parser's questions about commands from the working set.
///
/// The working set sits in a [`RefCell`] because the driver changes it between statements (a
/// `use` adds commands) while the parser, which only reads it, holds this lookup for the whole
/// block. The two never overlap: the parser asks while it parses a statement, the driver changes
/// the working set when it is handed the statement.
pub(super) struct EngineLookup<'c, 'w, 'e> {
    pub(super) working_set: &'c RefCell<&'w mut StateWorkingSet<'e>>,
    /// The kind of each declaration asked about so far, by id: finding it takes the command's
    /// signature, and a declaration keeps its kind while a block is parsed (a predeclaration
    /// and the definition replacing it take external arguments alike).
    kinds: RefCell<Vec<Option<DeclKind>>>,
}

impl<'c, 'w, 'e> EngineLookup<'c, 'w, 'e> {
    pub(super) fn new(working_set: &'c RefCell<&'w mut StateWorkingSet<'e>>) -> Self {
        Self {
            working_set,
            kinds: RefCell::new(Vec::new()),
        }
    }
}

impl CommandLookup for EngineLookup<'_, '_, '_> {
    fn find_decl(&self, name: &str) -> Option<DeclKind> {
        let working_set = self.working_set.borrow();
        let decl_id = working_set.find_decl(name.as_bytes())?;
        Some(cached_kind(&self.kinds, decl_id, || {
            decl_kind(&working_set, decl_id)
        }))
    }

    /// Always `true`: the working set keeps no index of first words, so the parser tries the
    /// longer names first, as the classic parser's `find_longest_decl` does.
    fn is_decl_name_prefix(&self, _word: &str) -> bool {
        true
    }

    /// No declaration in any engine has a longer name (`longest_decl_name`).
    fn longest_name(&self) -> usize {
        longest_decl_name()
    }

    fn is_builtin_decl(&self, name: &str) -> bool {
        let working_set = self.working_set.borrow();
        working_set
            .find_decl(name.as_bytes())
            .is_some_and(|decl_id| working_set.get_decl(decl_id).is_builtin())
    }
}

/// The kind of `decl_id` in `kinds` (by declaration id), worked out with `kind` the first time.
fn cached_kind(
    kinds: &RefCell<Vec<Option<DeclKind>>>,
    decl_id: DeclId,
    kind: impl FnOnce() -> DeclKind,
) -> DeclKind {
    let index = decl_id.get();
    if let Some(Some(known)) = kinds.borrow().get(index) {
        return *known;
    }
    let kind = kind();
    let mut kinds = kinds.borrow_mut();
    if kinds.len() <= index {
        kinds.resize(index + 1, None);
    }
    kinds[index] = Some(kind);
    kind
}

/// The engine's command names as a working set resolves them at one moment, for the thread that
/// parses a block's statements ahead of the lowering (see [`super::driver`]). It owns copies of
/// what resolving a name reads, so it can go to that thread while the working set goes on
/// changing. The thread asks it through an [`AskedLookup`], which writes every answer down for
/// the driver to check against the live working set before the statement is lowered.
pub(super) struct NamesSnapshot<'e> {
    /// A working set over the same engine whose scope frames copy the original's names
    /// (declarations, predeclarations, visibility, overlays), so that its `find_decl` answers
    /// as the original's did when the copy was made. It holds no declarations of its own: it is
    /// asked only about names, and about permanent declarations.
    names: StateWorkingSet<'e>,
    /// The kinds the original lookup had worked out, by declaration id. A delta declaration not
    /// among them is taken to be `Declared`, which most are.
    kinds: RefCell<Vec<Option<DeclKind>>>,
    /// The bound on the length of command names when the copy was made.
    pub(super) longest_name: usize,
}

impl<'e> NamesSnapshot<'e> {
    /// The names `lookup`'s working set resolves now.
    pub(super) fn new(lookup: &EngineLookup<'_, '_, 'e>) -> Self {
        let working_set = lookup.working_set.borrow();
        let mut names = StateWorkingSet::new(working_set.permanent_state);
        names.delta.scope = working_set.delta.scope.iter().map(names_only).collect();
        names.search_predecls = working_set.search_predecls;
        Self {
            names,
            kinds: RefCell::new(lookup.kinds.borrow().clone()),
            longest_name: longest_decl_name(),
        }
    }

    /// Whether `decl_id` is a declaration of the engine rather than of the working set.
    fn is_permanent(&self, decl_id: DeclId) -> bool {
        decl_id.get() < self.names.permanent_state.num_decls()
    }
}

/// A [`NamesSnapshot`] answering the winnow parser, every answer written down in `asked`.
pub(super) struct AskedLookup<'e> {
    pub(super) names: NamesSnapshot<'e>,
    pub(super) asked: Rc<RefCell<Vec<Asked>>>,
}

/// A question the winnow parser asked an [`AskedLookup`], with the answer it got.
pub(super) enum Asked {
    Decl(Box<str>, Option<DeclKind>),
    Builtin(Box<str>, bool),
}

impl CommandLookup for AskedLookup<'_> {
    fn find_decl(&self, name: &str) -> Option<DeclKind> {
        let names = &self.names;
        let kind = names.names.find_decl(name.as_bytes()).map(|decl_id| {
            cached_kind(&names.kinds, decl_id, || {
                if names.is_permanent(decl_id) {
                    decl_kind(&names.names, decl_id)
                } else {
                    DeclKind::Declared
                }
            })
        });
        self.asked
            .borrow_mut()
            .push(Asked::Decl(Box::from(name), kind));
        kind
    }

    fn is_decl_name_prefix(&self, _word: &str) -> bool {
        true
    }

    fn longest_name(&self) -> usize {
        self.names.longest_name
    }

    /// Only an engine declaration is built in.
    fn is_builtin_decl(&self, name: &str) -> bool {
        let names = &self.names;
        let builtin = names
            .names
            .find_decl(name.as_bytes())
            .is_some_and(|decl_id| {
                names.is_permanent(decl_id) && names.names.get_decl(decl_id).is_builtin()
            });
        self.asked
            .borrow_mut()
            .push(Asked::Builtin(Box::from(name), builtin));
        builtin
    }
}

/// What `lookup` (the live working set's) answers differently from `asked`, or a changed bound
/// on name lengths (it was `longest_name`); `None` when nothing changed, and parsing with
/// `lookup` would then have given the same tree.
pub(super) fn changed_answer(
    lookup: &EngineLookup,
    asked: &[Asked],
    longest_name: usize,
) -> Option<String> {
    let longest = longest_decl_name();
    if longest != longest_name {
        return Some(format!(
            "longest command name {longest} instead of {longest_name}"
        ));
    }
    asked.iter().find_map(|asked| match asked {
        Asked::Decl(name, kind) => {
            let live = lookup.find_decl(name);
            (live != *kind).then(|| format!("`{name}` is {live:?}, not {kind:?}"))
        }
        Asked::Builtin(name, builtin) => {
            let live = lookup.is_builtin_decl(name);
            (live != *builtin).then(|| format!("`{name}` built in: {live}, not {builtin}"))
        }
    })
}

/// `frame` with only what resolving a command name reads: no variables or modules.
fn names_only(frame: &ScopeFrame) -> ScopeFrame {
    ScopeFrame {
        overlays: frame
            .overlays
            .iter()
            .map(|(name, overlay)| {
                let names = OverlayFrame {
                    predecls: overlay.predecls.clone(),
                    decls: overlay.decls.clone(),
                    visibility: overlay.visibility.clone(),
                    ..OverlayFrame::from_origin(overlay.origin, overlay.prefixed)
                };
                (name.clone(), names)
            })
            .collect(),
        active_overlays: frame.active_overlays.clone(),
        removed_overlays: frame.removed_overlays.clone(),
        predecls: frame.predecls.clone(),
    }
}

/// How the parser treats calls to `decl_id`: an alias of an external command makes an external
/// call; a command whose rest parameter takes external arguments (an untyped `def --wrapped`),
/// or an alias of one, has its arguments parsed like an external command's.
fn decl_kind(working_set: &StateWorkingSet, decl_id: DeclId) -> DeclKind {
    let decl = working_set.get_decl(decl_id);
    if let Some(alias) = decl.as_alias() {
        return match &alias.wrapped_call.expr {
            Expr::ExternalCall(..) => DeclKind::ExternalAlias,
            Expr::Call(call) if takes_external_arguments(working_set, call.decl_id) => {
                DeclKind::Wrapped
            }
            _ => DeclKind::Declared,
        };
    }
    if takes_external_arguments(working_set, decl_id) {
        DeclKind::Wrapped
    } else if decl.command_type() == CommandType::Builtin {
        DeclKind::Builtin
    } else {
        DeclKind::Declared
    }
}

fn takes_external_arguments(working_set: &StateWorkingSet, decl_id: DeclId) -> bool {
    working_set
        .get_signature_shared(decl_id)
        .rest_positional
        .as_ref()
        .is_some_and(|rest| rest.shape == SyntaxShape::ExternalArgument)
}
