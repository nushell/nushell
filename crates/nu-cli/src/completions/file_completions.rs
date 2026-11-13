use crate::completions::{Completer, Context, Fetched, completion_common::complete_paths};

pub struct FileCompletion;

impl Completer for FileCompletion {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        complete_paths(ctx, false)
    }
}
