use crate::prelude::*;
use crate::{EntriesListView, GenericView, TreeView};
use futures::stream::{self, StreamExt};
use std::sync::{Arc, Mutex};

pub(crate) fn format(input: Vec<Value>, host: &mut dyn Host) {
    let last = input.len() - 1;
    for (i, item) in input.iter().enumerate() {
        let view = GenericView::new(item);
        crate::format::print_view(&view, &mut *host);

        if last != i {
            outln!("");
        }
    }
}
