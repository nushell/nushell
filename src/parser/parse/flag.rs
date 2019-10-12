use crate::parser::hir::syntax_shape::flat_shape::FlatShape;
use crate::{Spanned, SpannedItem, Tag};
use derive_new::new;
use getset::Getters;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum FlagKind {
    Shorthand,
    Longhand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Getters, new)]
#[get = "pub(crate)"]
pub struct Flag {
    pub(crate) kind: FlagKind,
    pub(crate) name: Tag,
}

impl Spanned<Flag> {
    pub fn color(&self) -> Spanned<FlatShape> {
        match self.item.kind {
            FlagKind::Longhand => FlatShape::Flag.spanned(self.span),
            FlagKind::Shorthand => FlatShape::ShorthandFlag.spanned(self.span),
        }
    }
}
