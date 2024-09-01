use super::prelude::*;
use crate::NuCursorShape;

#[derive(Clone, Copy, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorShape {
    pub emacs: NuCursorShape,
    pub vi_insert: NuCursorShape,
    pub vi_normal: NuCursorShape,
}
