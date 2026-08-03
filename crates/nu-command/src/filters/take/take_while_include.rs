pub(super) fn take_while_include_n<T>(
    iter: impl IntoIterator<Item = T>,
    mut predicate: impl FnMut(&T) -> bool,
    mut n: usize,
) -> impl Iterator<Item = T> {
    let mut done = false;
    let mut peekable = iter.into_iter().peekable();

    std::iter::from_fn(move || {
        match (done, n) {
            (true, 0) => None,
            (true, _) => {
                n -= 1;
                peekable.next()
            }
            (false, _) => {
                match peekable.next_if(&mut predicate) {
                    Some(e) => Some(e),
                    None => {
                        done = true;
                        // very unlikely to be false, just in case
                        if n > 0 {
                            n -= 1;
                            peekable.next()
                        } else {
                            None
                        }
                    }
                }
            }
        }
    })
}
