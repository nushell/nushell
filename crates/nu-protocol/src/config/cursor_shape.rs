use crate::{IntoValue, NuCursorShape};
use serde::{Deserialize, Serialize};

use crate as nu_protocol;

#[derive(Clone, Copy, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorShape {
    pub emacs: NuCursorShape,
    pub vi_insert: NuCursorShape,
    pub vi_normal: NuCursorShape,
}
