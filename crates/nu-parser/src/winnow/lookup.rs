//! The engine's declarations, as the winnow parser asks about them.

use std::cell::RefCell;

use nu_protocol::{
    DeclId, SyntaxShape,
    ast::Expr,
    engine::{CommandType, StateWorkingSet},
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
        let mut kinds = self.kinds.borrow_mut();
        let index = decl_id.get();
        if let Some(Some(kind)) = kinds.get(index) {
            return Some(*kind);
        }
        let kind = decl_kind(&working_set, decl_id);
        if kinds.len() <= index {
            kinds.resize(index + 1, None);
        }
        kinds[index] = Some(kind);
        Some(kind)
    }

    /// Always `true`: the working set keeps no index of first words, so the parser tries the
    /// longer names first, as the classic parser's `find_longest_decl` does.
    fn is_decl_name_prefix(&self, _word: &str) -> bool {
        true
    }

    fn is_builtin_decl(&self, name: &str) -> bool {
        let working_set = self.working_set.borrow();
        working_set
            .find_decl(name.as_bytes())
            .is_some_and(|decl_id| working_set.get_decl(decl_id).is_builtin())
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
