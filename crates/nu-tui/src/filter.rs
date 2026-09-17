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

    /// Rows that match, in their original order.
    pub fn apply(&self, rows: Vec<Value>) -> Vec<Value> {
        if self.is_empty() {
            return rows;
        }
        let mut matcher = self.matcher();
        rows.into_iter()
            .filter(|row| self.matches(&mut matcher, row))
            .collect()
    }

    pub fn matcher(&self) -> Matcher {
        Matcher::new({
            let mut config = NucleoConfig::DEFAULT;
            config.prefer_prefix = true;
            config
        })
    }

    /// Whether `value` matches. `matcher` comes from [`Filter::matcher`].
    pub fn matches(&self, matcher: &mut Matcher, value: &Value) -> bool {
        if self.is_empty() {
            return true;
        }
        self.texts(value)
            .into_iter()
            .any(|text| self.matches_text(matcher, &text))
    }

    /// Whether one piece of text matches the query.
    pub fn matches_text(&self, matcher: &mut Matcher, text: &str) -> bool {
        if self.fuzzy {
            let case = if self.case_sensitive {
                CaseMatching::Respect
            } else {
                CaseMatching::Smart
            };
            let atom = Atom::new(
                &self.query,
                case,
                Normalization::Smart,
                AtomKind::Fuzzy,
                false,
            );
            let mut buf = Vec::new();
            atom.score(Utf32Str::new(text, &mut buf), matcher).is_some()
        } else if self.case_sensitive {
            text.contains(&self.query)
        } else {
            text.to_lowercase().contains(&self.query.to_lowercase())
        }
    }

    /// The strings a value is matched on: the chosen columns of a record, or
    /// every field, or the value's text.
    fn texts(&self, value: &Value) -> Vec<String> {
        match value {
            Value::Record { val, .. } => {
                let mut out = Vec::new();
                if !self.columns.is_empty() {
                    for col in &self.columns {
                        if let Some(v) = val.get(col) {
                            out.push(value_text(v));
                        }
                    }
                    return out;
                }
                let key = formatted_key(val);
                if !key.is_empty() {
                    out.push(key);
                }
                for (k, v) in val.iter() {
                    out.push(k.clone());
                    out.push(value_text(v));
                }
                out
            }
            Value::List { vals, .. } => vals.iter().map(value_text).collect(),
            other => vec![value_text(other)],
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
        assert_eq!(f.apply(rows()).len(), 1);
        let f = Filter {
            query: "ALP".into(),
            case_sensitive: true,
            ..Default::default()
        };
        assert!(f.apply(rows()).is_empty());
    }

    #[test]
    fn fuzzy_matches_subsequence() {
        let f = Filter {
            query: "gma".into(),
            fuzzy: true,
            ..Default::default()
        };
        assert_eq!(f.apply(rows()).len(), 1);
        let f = Filter {
            query: "gma".into(),
            ..Default::default()
        };
        assert!(f.apply(rows()).is_empty());
    }

    #[test]
    fn columns_restrict_the_match() {
        let f = Filter {
            query: "name".into(),
            columns: vec!["name".into()],
            ..Default::default()
        };
        assert!(
            f.apply(rows()).is_empty(),
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
        assert_eq!(f.apply(vec![Value::test_record(r)]).len(), 1);
    }
}
