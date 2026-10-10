use crate::completions::{Completer, Context, Fetched, completion_common::complete_paths};

pub struct DirectoryCompletion;

impl Completer for DirectoryCompletion {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        complete_paths(ctx, true)
    }
}
