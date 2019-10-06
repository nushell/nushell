use crate::parser::hir::syntax_shape::flat_shape::FlatShape;
use crate::{Tag, Tagged, TaggedItem};
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

impl Tagged<Flag> {
    pub fn color(&self) -> Tagged<FlatShape> {
        match self.item.kind {
            FlagKind::Longhand => FlatShape::Flag.tagged(self.tag),
            FlagKind::Shorthand => FlatShape::ShorthandFlag.tagged(self.tag),
        }
    }
}
