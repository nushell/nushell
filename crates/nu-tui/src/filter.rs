//! Row filtering for search boxes and `--capture-keys`. Substring by
//! default, or fuzzy with the same nucleo matcher `input list --fuzzy` uses.

use crate::keys::normalize_binding;
use nu_protocol::{Record, Value};
use nucleo_matcher::{
    Config as NucleoConfig, Matcher, Utf32Str,
    pattern::{Atom, AtomKind, CaseMatching, Normalization},
};

/// A filter as configured by a `tui search` box.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub query: String,
    pub fuzzy: bool,
    pub case_sensitive: bool,
    /// Record columns to match against. Empty means every field.
    pub columns: Vec<String>,
}

impl Filter {
    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    /// The rows that match, cloned, in their original order.
    pub fn apply(&self, rows: &[Value]) -> Vec<Value> {
        if self.is_empty() {
            return rows.to_vec();
        }
        let mut matcher = self.matcher();
        rows.iter()
            .filter(|row| matcher.matches(row))
            .cloned()
            .collect()
    }

    /// A matcher with the pattern parsed once, for a whole pass over the rows.
    pub fn matcher(&self) -> Prepared<'_> {
        let atom = self.fuzzy.then(|| {
            let case = if self.case_sensitive {
                CaseMatching::Respect
            } else {
                CaseMatching::Smart
            };
            Atom::new(
                &self.query,
                case,
                Normalization::Smart,
                AtomKind::Fuzzy,
                false,
            )
        });
        Prepared {
            filter: self,
            matcher: Matcher::new({
                let mut config = NucleoConfig::DEFAULT;
                config.prefer_prefix = true;
                config
            }),
            atom,
            needle: if self.case_sensitive {
                self.query.clone()
            } else {
                self.query.to_lowercase()
            },
            buf: Vec::new(),
        }
    }

    /// Visit the strings a value is matched on: the chosen columns of a
    /// record, or every key and field, or the value's text. Stops at the
    /// first visit that returns `true`. String fields are borrowed, so a
    /// pass over a long list allocates only for non-string values.
    fn any_text(&self, value: &Value, f: &mut dyn FnMut(&str) -> bool) -> bool {
        fn hit(v: &Value, f: &mut dyn FnMut(&str) -> bool) -> bool {
            match v {
                Value::String { val, .. } => f(val),
                Value::Nothing { .. } => f(""),
                other => f(&value_text(other)),
            }
        }
        match value {
            Value::Record { val, .. } => {
                if !self.columns.is_empty() {
                    return self
                        .columns
                        .iter()
                        .filter_map(|col| val.get(col))
                        .any(|v| hit(v, f));
                }
                let key = formatted_key(val);
                if !key.is_empty() && f(&key) {
                    return true;
                }
                val.iter().any(|(k, v)| f(k) || hit(v, f))
            }
            Value::List { vals, .. } => vals.iter().any(|v| hit(v, f)),
            other => hit(other, f),
        }
    }
}

/// A [`Filter`] ready to test many values: the fuzzy pattern is parsed once
/// and the scratch buffer is reused.
pub struct Prepared<'a> {
    filter: &'a Filter,
    matcher: Matcher,
    atom: Option<Atom>,
    needle: String,
    buf: Vec<char>,
}

impl Prepared<'_> {
    /// Whether `value` matches.
    pub fn matches(&mut self, value: &Value) -> bool {
        if self.filter.is_empty() {
            return true;
        }
        let filter = self.filter;
        filter.any_text(value, &mut |text| self.matches_text(text))
    }

    /// Whether one piece of text matches the query.
    pub fn matches_text(&mut self, text: &str) -> bool {
        match &self.atom {
            Some(atom) => atom
                .score(Utf32Str::new(text, &mut self.buf), &mut self.matcher)
                .is_some(),
            None if self.filter.case_sensitive => text.contains(&self.needle),
            None => text.to_lowercase().contains(&self.needle),
        }
    }
}

/// Flat text of a value for matching.
pub fn value_text(value: &Value) -> String {
    match value {
        Value::String { val, .. } => val.clone(),
        Value::Nothing { .. } => String::new(),
        other => other.to_expanded_string(", ", &nu_protocol::Config::default()),
    }
}

/// `ctrl+r` for a keybinding row `{modifier: control, keycode: char_r}`, so
/// a captured chord finds the row that binds it. Empty for other records.
pub fn formatted_key(record: &Record) -> String {
    let modifier = record
        .get("modifier")
        .and_then(|v| v.as_str().ok())
        .unwrap_or("");
    let keycode = record
        .get("keycode")
        .and_then(|v| v.as_str().ok())
        .unwrap_or("");
    if keycode.is_empty() {
        return String::new();
    }
    normalize_binding(modifier, keycode)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Value> {
        ["alpha", "beta", "gamma"]
            .into_iter()
            .map(|n| {
                let mut r = Record::new();
                r.insert("name", Value::test_string(n));
                Value::test_record(r)
            })
            .collect()
    }

    #[test]
    fn substring_is_case_insensitive_by_default() {
        let f = Filter {
            query: "ALP".into(),
            ..Default::default()
        };
        assert_eq!(f.apply(&rows()).len(), 1);
        let f = Filter {
            query: "ALP".into(),
            case_sensitive: true,
            ..Default::default()
        };
        assert!(f.apply(&rows()).is_empty());
    }

    #[test]
    fn fuzzy_matches_subsequence() {
        let f = Filter {
            query: "gma".into(),
            fuzzy: true,
            ..Default::default()
        };
        assert_eq!(f.apply(&rows()).len(), 1);
        let f = Filter {
            query: "gma".into(),
            ..Default::default()
        };
        assert!(f.apply(&rows()).is_empty());
    }

    #[test]
    fn columns_restrict_the_match() {
        let f = Filter {
            query: "name".into(),
            columns: vec!["name".into()],
            ..Default::default()
        };
        assert!(
            f.apply(&rows()).is_empty(),
            "keys are not matched when columns are set"
        );
    }

    #[test]
    fn keybinding_rows_match_their_chord() {
        let mut r = Record::new();
        r.insert("modifier", Value::test_string("control"));
        r.insert("keycode", Value::test_string("char_r"));
        let f = Filter {
            query: "ctrl+r".into(),
            ..Default::default()
        };
        assert_eq!(f.apply(&[Value::test_record(r)]).len(), 1);
    }
}
