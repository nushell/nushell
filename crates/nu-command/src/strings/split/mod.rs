mod chars;
mod column;
mod command;
mod list;
mod row;
mod words;

pub use chars::SplitChars;
pub use column::SplitColumn;
pub use command::Split;
pub use list::SubCommand as SplitList;
pub use row::SplitRow;
pub use words::SplitWords;

/// Split `s` given the bounds of the separators in `sep_bounds`.
fn rsplitn<I>(s: &str, sep_bounds: I) -> impl Iterator<Item = &str>
where
    I: IntoIterator<Item = (usize, usize)>,
{
    use itertools::Itertools;

    itertools::chain!([(0, 0)], sep_bounds, [(s.len(), 0)])
        .tuple_windows()
        .map(|(a, b)| &s[(a.1)..(b.0)])
}
