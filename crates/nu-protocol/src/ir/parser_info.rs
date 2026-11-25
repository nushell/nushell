use crate::{
    BlockId, DeclId, ModuleId, VarId,
    ast::{Expr, Expression, ImportPattern},
};
use nu_path::expand_path;
use serde::{Deserialize, Serialize};

/// A value the parser or the compiler attaches to a call under a name, for a keyword command to
/// read instead of an argument. [`Instruction::PushParserInfo`](super::Instruction) carries it in
/// IR, and [`Argument::ParserInfo`](crate::engine::Argument::ParserInfo) at runtime.
///
/// The parser stores parser info as expressions in [`ast::Call`](crate::ast::Call);
/// [`ParserInfo::from_expression`] turns those into this type when they're compiled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParserInfo {
    /// Only the name's presence matters: `noop`, `full_reparse` and `deferred_arguments`.
    Flag,
    /// A block the parser compiled: `block_id` and `main_block_id`.
    BlockId(BlockId),
    /// The declaration that `<cmd> --help` resolved to: `help_decl_id`.
    DeclId(DeclId),
    /// The resolved path of a sourced or run script: `block_id_name`.
    Path(String),
    /// The variable that holds the directory stack: `dirs_var`.
    Var(VarId),
    /// What `use` or `export use` imports: `import_pattern`.
    ImportPattern(Box<ImportPattern>),
    /// The module that `overlay use` was parsed from, when the parser updated it: `overlay_expr`.
    Overlay(Option<ModuleId>),
}

impl ParserInfo {
    /// Turns the parser's expression for the parser info `name` into its typed form, or `None` for
    /// an expression that has none.
    ///
    /// The parser stores block ids as `Expr::Int`, which doesn't say which kind of id it holds, so
    /// an int only becomes a [`ParserInfo::BlockId`] under the names that hold block ids.
    pub fn from_expression(name: &str, expression: &Expression) -> Option<Self> {
        match &expression.expr {
            Expr::Nothing | Expr::Bool(true) => Some(ParserInfo::Flag),
            Expr::Int(id) if matches!(name, "block_id" | "main_block_id") => usize::try_from(*id)
                .ok()
                .map(|id| ParserInfo::BlockId(BlockId::new(id))),
            Expr::Filepath(path, true) => Some(ParserInfo::Path(path.clone())),
            // Expand `~`, `...` and `..` the way evaluating the expression did.
            Expr::Filepath(path, false) => Some(ParserInfo::Path(
                expand_path(path, true).to_string_lossy().into_owned(),
            )),
            Expr::Var(var_id) => Some(ParserInfo::Var(*var_id)),
            Expr::ImportPattern(pattern) => Some(ParserInfo::ImportPattern(pattern.clone())),
            Expr::Overlay(module_id) => Some(ParserInfo::Overlay(*module_id)),
            _ => None,
        }
    }

    /// The block id of a [`ParserInfo::BlockId`].
    pub fn block_id(&self) -> Option<BlockId> {
        match self {
            ParserInfo::BlockId(block_id) => Some(*block_id),
            _ => None,
        }
    }

    /// The path of a [`ParserInfo::Path`].
    pub fn path(&self) -> Option<&str> {
        match self {
            ParserInfo::Path(path) => Some(path),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Span, Type,
        ast::{ImportPatternHead, ImportPatternMember},
    };
    use std::path::Path;

    fn lower(name: &str, expr: Expr) -> Option<ParserInfo> {
        ParserInfo::from_expression(
            name,
            &Expression::new_unknown(expr, Span::test_data(), Type::Any),
        )
    }

    #[test]
    fn presence_only_names_lower_to_flag() {
        assert_eq!(lower("noop", Expr::Nothing), Some(ParserInfo::Flag));
        assert_eq!(lower("noop", Expr::Bool(true)), Some(ParserInfo::Flag));
        assert_eq!(
            lower("full_reparse", Expr::Bool(true)),
            Some(ParserInfo::Flag)
        );
        assert_eq!(
            lower("deferred_arguments", Expr::Bool(true)),
            Some(ParserInfo::Flag)
        );
        assert_eq!(lower("noop", Expr::Bool(false)), None);
    }

    #[test]
    fn ints_lower_only_under_block_id_names() {
        assert_eq!(
            lower("block_id", Expr::Int(5)),
            Some(ParserInfo::BlockId(BlockId::new(5)))
        );
        assert_eq!(
            lower("main_block_id", Expr::Int(7)),
            Some(ParserInfo::BlockId(BlockId::new(7)))
        );
        // `help_decl_id` holds a decl id, and the compiler builds it as a `DeclId` itself.
        assert_eq!(lower("help_decl_id", Expr::Int(5)), None);
        assert_eq!(lower("something_else", Expr::Int(5)), None);
        assert_eq!(lower("block_id", Expr::Int(-1)), None);
    }

    #[test]
    fn unquoted_paths_are_expanded() {
        let Some(ParserInfo::Path(path)) = lower(
            "block_id_name",
            Expr::Filepath("/scripts/sub/../x.nu".into(), false),
        ) else {
            panic!("expected a path");
        };
        let path = Path::new(&path);
        assert!(path.ends_with("scripts/x.nu"), "{path:?}");
        assert!(!path.to_string_lossy().contains(".."), "{path:?}");

        assert_eq!(
            lower(
                "block_id_name",
                Expr::Filepath("/scripts/sub/../x.nu".into(), true)
            ),
            Some(ParserInfo::Path("/scripts/sub/../x.nu".into()))
        );
    }

    #[test]
    fn ids_and_patterns_keep_their_values() {
        assert_eq!(
            lower("dirs_var", Expr::Var(VarId::new(3))),
            Some(ParserInfo::Var(VarId::new(3)))
        );
        assert_eq!(
            lower("overlay_expr", Expr::Overlay(Some(ModuleId::new(2)))),
            Some(ParserInfo::Overlay(Some(ModuleId::new(2))))
        );
        assert_eq!(
            lower("overlay_expr", Expr::Overlay(None)),
            Some(ParserInfo::Overlay(None))
        );

        let pattern = ImportPattern {
            head: ImportPatternHead {
                name: b"assert".to_vec(),
                id: Some(ModuleId::new(1)),
                span: Span::test_data(),
            },
            members: vec![ImportPatternMember::Glob {
                span: Span::test_data(),
            }],
            hidden: Default::default(),
            constants: vec![],
        };
        assert_eq!(
            lower(
                "import_pattern",
                Expr::ImportPattern(Box::new(pattern.clone()))
            ),
            Some(ParserInfo::ImportPattern(Box::new(pattern)))
        );
    }

    #[test]
    fn other_expressions_have_no_typed_form() {
        assert_eq!(lower("noop", Expr::String("x".into())), None);
        assert_eq!(lower("block_id_name", Expr::Int(1)), None);
    }
}
