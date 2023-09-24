use crate::Value;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Record {
    pub cols: Vec<String>,
    pub vals: Vec<Value>,
}

impl Record {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cols: Vec::with_capacity(capacity),
            vals: Vec::with_capacity(capacity),
        }
    }

    pub fn iter(&self) -> Iter {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> IterMut {
        self.into_iter()
    }

    pub fn is_empty(&self) -> bool {
        self.cols.is_empty() || self.vals.is_empty()
    }

    pub fn len(&self) -> usize {
        usize::min(self.cols.len(), self.vals.len())
    }

    pub fn push(&mut self, col: impl Into<String>, val: Value) {
        self.cols.push(col.into());
        self.vals.push(val);
    }
}

impl FromIterator<(String, Value)> for Record {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(iter: T) -> Self {
        let (cols, vals) = iter.into_iter().unzip();
        Self { cols, vals }
    }
}

pub type IntoIter = std::iter::Zip<std::vec::IntoIter<String>, std::vec::IntoIter<Value>>;

impl IntoIterator for Record {
    type Item = (String, Value);

    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.cols.into_iter().zip(self.vals)
    }
}

pub type Iter<'a> = std::iter::Zip<std::slice::Iter<'a, String>, std::slice::Iter<'a, Value>>;

impl<'a> IntoIterator for &'a Record {
    type Item = (&'a String, &'a Value);

    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.cols.iter().zip(&self.vals)
    }
}

pub type IterMut<'a> = std::iter::Zip<std::slice::Iter<'a, String>, std::slice::IterMut<'a, Value>>;

impl<'a> IntoIterator for &'a mut Record {
    type Item = (&'a String, &'a mut Value);

    type IntoIter = IterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.cols.iter().zip(&mut self.vals)
    }
}

#[macro_export]
macro_rules! record {
    {$col:expr => $val:expr, $($tail:tt)*} => {
        record!(@top $col => $val, $($tail)*)
    };

    {$val:ident, $($tail:tt)*} => {
        record!(@top $val, $($tail)*)
    };

    {$col:expr => $val:expr} => {
        record!($col => $val,)
    };

    {$val:ident} => {
        record!($val,)
    };

    {@top $($tokens:tt)+} => {
        {
            let mut cols = Vec::new();

            #[allow(clippy::vec_init_then_push)]
            {
                record!(@col cols, $($tokens)+);
            }

            let mut vals = Vec::new();

            #[allow(clippy::vec_init_then_push)]
            {
                record!(@val vals, $($tokens)+);
            }

            $crate::Record {
                cols,
                vals,
            }
        }
    };

    {@col $cols:ident, $col:expr => $val:expr, $($tail:tt)*} => {
        $cols.push($col.into());
        record!(@col $cols, $($tail)*)
    };
    {@col $cols:ident, $col:expr => $val:expr} => {
        record!(@col $cols, $col => $val,)
    };
    {@col $cols:ident, $val:ident, $($tail:tt)* } => {
        $cols.push(stringify!($val).into());
        record!(@col $cols, $($tail)*)
    };
    {@col $cols:ident, $val:ident } => {
        record!(@col $cols, $val,)
    };
    {@col $cols:ident, $(,)?} => {};

    {@val $vals:ident, $col:expr => $val:expr, $($tail:tt)* } => {
        $vals.push($val);
        record!(@val $vals, $($tail)*)
    };
    {@val $vals:ident, $col:expr => $val:expr} => {
        record!(@val $vals, $col => $val,)
    };
    {@val $vals:ident, $val:ident, $($tail:tt)* } => {
        $vals.push($val);
        record!(@val $vals, $($tail)*)
    };
    {@val $cols:ident, $val:ident } => {
        record!(@val $cols, $val,)
    };
    {@val $vals:ident, $(,)?} => {};

    {} => {};
}

#[cfg(test)]
mod test {
    use crate::{Record, Span, Value};

    #[test]
    fn record_macro_arrows() {
        assert_eq!(
            Record::from_iter([(
                "foo".into(),
                Value::string("bar".to_owned(), Span::unknown())
            ),]),
            record!("foo" => Value::string("bar".to_owned(), Span::unknown())) // no trailing comma
        );

        assert_eq!(
            Record::from_iter([
                (
                    "foo".into(),
                    Value::string("bar".to_owned(), Span::unknown())
                ),
                (
                    "baz".into(),
                    Value::string("quux".to_owned(), Span::unknown())
                ),
            ]),
            record!(
                "foo" => Value::string("bar".to_owned(), Span::unknown()),
                "baz" => Value::string("quux".to_owned(), Span::unknown()) // no trailing comma
            )
        );

        assert_eq!(
            Record::from_iter([
                ("foo".into(), Value::int(0, Span::unknown())),
                ("baz".into(), Value::int(1, Span::unknown())),
            ]),
            record!(
                "foo" => Value::int(0, Span::unknown()),
                "baz" => Value::int(1, Span::unknown()), // with trailing comma
            )
        );
    }

    #[test]
    fn record_macro_identifier() {
        let foo = Value::bool(false, Span::unknown());
        let bar = Value::int(1, Span::unknown());

        assert_eq!(
            Record::from_iter([
                ("foo".into(), Value::bool(false, Span::unknown())),
                ("bar".into(), Value::int(1, Span::unknown())),
            ]),
            record!(foo, bar)
        );

        let foo = Value::bool(true, Span::unknown());
        let bar = Value::int(2, Span::unknown());

        assert_eq!(
            Record::from_iter([
                ("foo".into(), Value::bool(true, Span::unknown())),
                ("bar".into(), Value::int(2, Span::unknown())) // no trailing comma
            ]),
            record!(foo, bar)
        );
    }

    #[test]
    fn record_macro_identifier_arrow_mix() {
        let foo = Value::bool(false, Span::unknown());

        assert_eq!(
            Record::from_iter([
                ("foo".into(), Value::bool(false, Span::unknown())),
                ("bar".into(), Value::int(1, Span::unknown()))
            ]),
            record!(foo, "bar".to_owned() => Value::int(1, Span::unknown()))
        );

        let bar = Value::int(2, Span::unknown());

        assert_eq!(
            Record::from_iter([
                ("foo".into(), Value::bool(false, Span::unknown())),
                ("bar".into(), Value::int(2, Span::unknown())) // no trailing comma
            ]),
            record!("foo".to_owned() => Value::bool(false, Span::unknown()), bar)
        );
    }
}
