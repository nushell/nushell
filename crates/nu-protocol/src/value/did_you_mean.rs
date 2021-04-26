use crate::Value;
use std::cmp;

/// Prepares a list of "sounds like" matches (using edit distance) for the string you're trying to find
pub fn did_you_mean(obj_source: &Value, field_tried: String) -> Option<Vec<String>> {
    let possibilities = obj_source.data_descriptors();

    let mut possible_matches: Vec<_> = possibilities
        .into_iter()
        .map(|word| {
            let edit_distance = levenshtein_distance(&word, &field_tried);
            (edit_distance, word)
        })
        .collect();

    if !possible_matches.is_empty() {
        possible_matches.sort();
        let words_matched: Vec<String> = possible_matches.into_iter().map(|m| m.1).collect();
        Some(words_matched)
    } else {
        None
    }
}

/// Borrowed from https://crates.io/crates/natural
fn levenshtein_distance(str1: &str, str2: &str) -> usize {
    let mut current: Vec<usize> = (0..str1.len() + 1).collect();
    let str1_chars: Vec<char> = str1.chars().collect();
    let str2_chars: Vec<char> = str2.chars().collect();

    let str1_len = str1_chars.len();
    let str2_len = str2_chars.len();

    for str2_index in 1..str2_len + 1 {
        let previous = current;
        current = vec![0; str1_len + 1];
        current[0] = str2_index;
        for str1_index in 1..str1_len + 1 {
            let add = previous[str1_index] + 1;
            let delete = current[str1_index - 1] + 1;
            let mut change = previous[str1_index - 1];
            if str1_chars[str1_index - 1] != str2_chars[str2_index - 1] {
                change += 1
            }
            current[str1_index] = min3(add, delete, change);
        }
    }
    current[str1_len]
}

fn min3<T: Ord>(a: T, b: T, c: T) -> T {
    cmp::min(a, cmp::min(b, c))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::UntaggedValue;
    use indexmap::indexmap;
    use nu_source::Tag;

    #[test]
    fn did_you_mean_returns_possible_column_matches() {
        let value = UntaggedValue::row(indexmap! {
           "dog".to_string() => UntaggedValue::int(1).into(),
           "cat".to_string() => UntaggedValue::int(1).into(),
           "alt".to_string() => UntaggedValue::int(1).into(),
        });

        let source = Value {
            tag: Tag::unknown(),
            value,
        };

        assert_eq!(
            Some(vec![
                "cat".to_string(),
                "alt".to_string(),
                "dog".to_string()
            ]),
            did_you_mean(&source, "hat".to_string())
        )
    }

    #[test]
    fn did_you_mean_returns_no_matches_when_empty() {
        let empty_source = Value {
            tag: Tag::unknown(),
            value: UntaggedValue::row(indexmap! {}),
        };

        assert_eq!(None, did_you_mean(&empty_source, "hat".to_string()))
    }
}
