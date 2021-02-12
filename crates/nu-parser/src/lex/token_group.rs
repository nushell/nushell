use smart_default::SmartDefault;
use std::iter::FromIterator;

use derive_new::new;
use nu_source::{HasSpan, Span};

#[derive(Debug, Clone, SmartDefault, new)]
pub struct TokenBuilder<T: HasSpan> {
    #[default(None)]
    contents: Option<Vec<T>>,
}

impl<T> From<TokenBuilder<T>> for Vec<T>
where
    T: HasSpan,
{
    fn from(x: TokenBuilder<T>) -> Self {
        x.contents.unwrap_or_else(Vec::new)
    }
}

impl<T> HasSpan for TokenBuilder<T>
where
    T: HasSpan,
{
    fn span(&self) -> Span {
        match &self.contents {
            Some(vec) => {
                let mut iter = vec.iter();
                let head = iter.next();
                let last = iter.last().or(head);

                match (head, last) {
                    (Some(head), Some(last)) => Span::new(head.span().start(), last.span().end()),
                    _ => Span::default(),
                }
            }
            None => Span::new(0, 0),
        }
    }
}

impl<T> TokenBuilder<T>
where
    T: HasSpan,
{
    pub fn is_empty(&self) -> bool {
        self.contents.is_none()
    }

    pub fn take(&mut self) -> Option<TokenBuilder<T>> {
        self.contents.take().map(|c| TokenBuilder::new(Some(c)))
    }

    pub fn map<I, U>(self, mapper: impl Fn(T) -> U) -> I
    where
        I: FromIterator<U>,
    {
        match self.contents {
            Some(contents) => contents.into_iter().map(mapper).collect(),
            None => I::from_iter(None),
        }
    }

    pub fn push(&mut self, item: T) {
        let contents = match self.contents.take() {
            Some(mut contents) => {
                contents.push(item);
                contents
            }
            None => vec![item],
        };

        self.contents.replace(contents);
    }
}
