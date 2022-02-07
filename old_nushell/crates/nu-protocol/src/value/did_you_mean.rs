use crate::Value;

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

// Borrowed from here https://github.com/wooorm/levenshtein-rs
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let mut result = 0;

    /* Shortcut optimizations / degenerate cases. */
    if a == b {
        return result;
    }

    let length_a = a.chars().count();
    let length_b = b.chars().count();

    if length_a == 0 {
        return length_b;
    }

    if length_b == 0 {
        return length_a;
    }

    /* Initialize the vector.
     *
     * This is why it’s fast, normally a matrix is used,
     * here we use a single vector. */
    let mut cache: Vec<usize> = (1..).take(length_a).collect();
    let mut distance_a;
    let mut distance_b;

    /* Loop. */
    for (index_b, code_b) in b.chars().enumerate() {
        result = index_b;
        distance_a = index_b;

        for (index_a, code_a) in a.chars().enumerate() {
            distance_b = if code_a == code_b {
                distance_a
            } else {
                distance_a + 1
            };

            distance_a = cache[index_a];

            result = if distance_a > result {
                if distance_b > result {
                    result + 1
                } else {
                    distance_b
                }
            } else if distance_b > distance_a {
                distance_a + 1
            } else {
                distance_b
            };

            cache[index_a] = result;
        }
    }

    result
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

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(super::levenshtein_distance("hello world", "hello world"), 0);
        assert_eq!(super::levenshtein_distance("hello", "hello world"), 6);
        assert_eq!(super::levenshtein_distance("°C", "°C"), 0);
        assert_eq!(super::levenshtein_distance("°", "°C"), 1);
    }
}
