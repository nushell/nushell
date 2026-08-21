/// A result type with the sole purpose of collecting multiple errors from [`Iterator::collect`].
///
/// `Iterator::collect<MultiResult<Vec<T>, Vec<E>>>` is similar to
/// `Iterator::collect<Result<Vec<T>, E>>`, but instead of stopping at the first error, it keeps
/// collecting errors until the iterator is exhausted.
///
/// ```
/// # use nu_utils::MultiResult;
/// let outcome = [Ok(1), Err(2), Ok(3), Err(4), Ok(5), Err(6)]
///     .into_iter()
///     .collect::<MultiResult<Vec<_>, Vec<_>>>()
///     .result();
///
/// assert_eq!(outcome, Err(vec![2, 4, 6]))
/// ```
///
/// To collect both `Ok` and `Err` variants separately see [`Iterator::partition`].
#[derive(Debug, PartialEq, Eq)]
pub enum MultiResult<Ts, Es> {
    Ok(Ts),
    Err(Es),
}

impl<Ts, Es> MultiResult<Ts, Es> {
    pub fn result(self) -> Result<Ts, Es> {
        match self {
            Self::Ok(ok) => Ok(ok),
            Self::Err(err) => Err(err),
        }
    }
}

impl<Ts, Es> From<MultiResult<Ts, Es>> for Result<Ts, Es> {
    fn from(value: MultiResult<Ts, Es>) -> Self {
        value.result()
    }
}

impl<T, E, Ts, Es> FromIterator<Result<T, E>> for MultiResult<Ts, Es>
where
    Ts: FromIterator<T>,
    Es: FromIterator<E>,
{
    fn from_iter<Iter: IntoIterator<Item = Result<T, E>>>(iter: Iter) -> Self {
        let mut iter = iter.into_iter();
        match iter.by_ref().collect::<Result<Ts, E>>() {
            Ok(oks) => Self::Ok(oks),
            Err(err) => Self::Err(
                std::iter::once(err)
                    .chain(iter.filter_map(Result::err))
                    .collect::<Es>(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_ok() {
        let outcome = [1, 2, 3]
            .into_iter()
            .map(Ok::<i32, ()>)
            .collect::<MultiResult<Vec<_>, Vec<_>>>()
            .result();

        assert_eq!(outcome, Ok(vec![1, 2, 3]))
    }

    #[test]
    fn all_err() {
        let outcome = [1, 2, 3]
            .into_iter()
            .map(Err::<(), i32>)
            .collect::<MultiResult<Vec<_>, Vec<_>>>()
            .result();

        assert_eq!(outcome, Err(vec![1, 2, 3]))
    }

    #[test]
    fn mixed_ok_err() {
        let outcome = [Ok(1), Err(2), Ok(3), Err(4), Ok(5), Err(6)]
            .into_iter()
            .collect::<MultiResult<Vec<_>, Vec<_>>>()
            .result();

        assert_eq!(outcome, Err(vec![2, 4, 6]))
    }

    #[test]
    fn nested_all_ok_ok() {
        let outcome = [Ok(Ok(1)), Ok(Ok(2)), Ok(Ok(3))]
            .into_iter()
            .collect::<MultiResult<MultiResult<Vec<i32>, Vec<i32>>, Vec<i32>>>();

        assert_eq!(outcome, MultiResult::Ok(MultiResult::Ok(vec![1, 2, 3])))
    }

    #[test]
    fn nested_ok_err() {
        let outcome = [Ok(Ok(1)), Ok(Err(2)), Ok(Ok(3)), Ok(Err(4))]
            .into_iter()
            .collect::<MultiResult<MultiResult<Vec<i32>, Vec<i32>>, Vec<i32>>>();

        assert_eq!(outcome, MultiResult::Ok(MultiResult::Err(vec![2, 4])))
    }

    #[test]
    fn nested_err() {
        let outcome = [Ok(Ok(1)), Err(2), Ok(Ok(3)), Err(4)]
            .into_iter()
            .collect::<MultiResult<MultiResult<Vec<i32>, Vec<i32>>, Vec<i32>>>();

        assert_eq!(outcome, MultiResult::Err(vec![2, 4]))
    }
}
