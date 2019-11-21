use derive_new::new;
use nom_locate::LocatedSpanEx;
use nom_tracable::{HasTracableInfo, TracableInfo};

pub type NomSpan<'a> = LocatedSpanEx<&'a str, TracableContext>;

#[derive(Debug, Clone, Copy, PartialEq, new)]
pub struct TracableContext {
    pub(crate) info: TracableInfo,
}

impl HasTracableInfo for TracableContext {
    fn get_tracable_info(&self) -> TracableInfo {
        self.info
    }

    fn set_tracable_info(self, info: TracableInfo) -> Self {
        TracableContext { info }
    }
}

impl std::ops::Deref for TracableContext {
    type Target = TracableInfo;

    fn deref(&self) -> &TracableInfo {
        &self.info
    }
}

pub fn nom_input(s: &str) -> NomSpan<'_> {
    LocatedSpanEx::new_extra(s, TracableContext::new(TracableInfo::new()))
}
