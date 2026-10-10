use nu_parser::lex_with_delimiter_pairs;
use reedline::{HintContext, HintEdit, HintPolicy, HintQuery};

/// Applies Nushell's lexer-backed delimiter mapping to hints shown around auto-pairs.
///
/// The lexer reports matched brackets and ordinary single, double, and backtick quotes, including
/// pairs inside interpolated expressions. Raw string boundaries are not paired because their
/// closer includes `#` bytes. Interpolation candidates containing an unquoted `#` suppress all
/// delimiter mappings, which also rejects raw strings without duplicating lexer rules. A lexical
/// error, unsupported tail character, or
/// ambiguous ordered mapping also suppresses the hint. The source buffer itself may be incomplete
/// after an earlier partial acceptance; the candidate is the syntax input for delimiter matching.
pub(crate) struct AutoPairHintPolicy {
    external_hinter: bool,
    prepared_tail: Option<PreparedTail>,
}

struct PreparedTail {
    // The engine calls partial planning only for the unchanged active hint and source stamp.
    cursor: usize,
    candidate_len: usize,
    targets: Vec<usize>,
}

impl AutoPairHintPolicy {
    pub(crate) fn new(external_hinter: bool) -> Self {
        Self {
            external_hinter,
            prepared_tail: None,
        }
    }
}

impl HintPolicy for AutoPairHintPolicy {
    fn query(&self, context: &HintContext<'_>) -> HintQuery {
        if !self.external_hinter
            && context.selection().is_none()
            && trailing_closers(context.source(), context.cursor(), context.auto_pairs()).is_some()
        {
            HintQuery::new(0..context.cursor(), context.cursor())
        } else {
            HintQuery::whole_buffer(context)
        }
    }

    fn plan(&mut self, context: &HintContext<'_>, candidate: &str) -> Option<HintEdit> {
        self.prepared_tail = None;
        if context.selection().is_some() || candidate.is_empty() {
            return None;
        }

        let Some(tail) = trailing_closers(context.source(), context.cursor(), context.auto_pairs())
        else {
            return if context.at_buffer_end() {
                let end = context.source().len();
                Some(HintEdit::new(end..end, candidate, end + candidate.len()))
            } else {
                None
            };
        };

        let targets = candidate_closer_targets(
            context.source(),
            context.cursor(),
            candidate,
            context.auto_pairs(),
            tail,
        )?;
        self.prepared_tail = Some(PreparedTail {
            cursor: context.cursor(),
            candidate_len: candidate.len(),
            targets,
        });
        let cursor = context.cursor();
        let end = context.source().len();
        Some(HintEdit::new(
            cursor..end,
            candidate,
            cursor + candidate.len(),
        ))
    }

    fn plan_partial(
        &mut self,
        context: &HintContext<'_>,
        candidate: &str,
        next_token: &str,
    ) -> Option<HintEdit> {
        if context.selection().is_some() || next_token.is_empty() {
            return None;
        }

        let Some(_tail) =
            trailing_closers(context.source(), context.cursor(), context.auto_pairs())
        else {
            if context.at_buffer_end() {
                let end = context.source().len();
                return Some(HintEdit::new(end..end, next_token, end + next_token.len()));
            }
            return None;
        };

        if !candidate.starts_with(next_token) {
            return None;
        }

        let prepared = self.prepared_tail.as_ref()?;
        if prepared.cursor != context.cursor() || prepared.candidate_len != candidate.len() {
            return None;
        }
        let accepted_end = context.cursor() + next_token.len();
        let consumed_bytes = prepared
            .targets
            .iter()
            .take_while(|candidate_close| **candidate_close < accepted_end)
            .count();
        let cursor = context.cursor() + next_token.len();
        Some(HintEdit::new(
            context.cursor()..context.cursor() + consumed_bytes,
            next_token,
            cursor,
        ))
    }
}

fn trailing_closers(
    source: &str,
    cursor: usize,
    pairs: impl Iterator<Item = (char, char)>,
) -> Option<&str> {
    if cursor >= source.len() || !source.is_char_boundary(cursor) {
        return None;
    }
    let configured_closers: Vec<char> = pairs.map(|(_, close)| close).collect();
    let tail = source.get(cursor..)?;
    (!configured_closers.is_empty() && tail.chars().all(|ch| configured_closers.contains(&ch)))
        .then_some(tail)
}

/// Map each source tail closer to a unique candidate pair close from an opener in the query prefix.
fn candidate_closer_targets(
    source: &str,
    cursor: usize,
    hint: &str,
    pairs: impl Iterator<Item = (char, char)>,
    tail: &str,
) -> Option<Vec<usize>> {
    let prefix = source.get(..cursor)?;
    let candidate = format!("{prefix}{hint}");
    let (_, error, pairs_found) = lex_with_delimiter_pairs(candidate.as_bytes(), 0, b"", b"", true);
    if error.is_some() {
        return None;
    }

    let configured_pairs: Vec<(char, char)> = pairs.collect();
    let candidate_closes: Vec<(usize, char)> = pairs_found
        .into_iter()
        .filter_map(|(open, close)| {
            if open >= cursor || close < cursor {
                return None;
            }
            let opening = candidate.get(open..)?.chars().next()?;
            let closing = candidate.get(close..)?.chars().next()?;
            configured_pairs
                .contains(&(opening, closing))
                .then_some((close, closing))
        })
        .collect();
    unique_ordered_matches(tail, &candidate_closes)
}

fn unique_ordered_matches(tail: &str, candidate_closes: &[(usize, char)]) -> Option<Vec<usize>> {
    let wanted: Vec<char> = tail.chars().collect();
    if wanted.iter().any(|ch| ch.len_utf8() != 1) {
        return None;
    }

    let mut earliest = Vec::with_capacity(wanted.len());
    let mut candidate_index = 0;
    for wanted_close in &wanted {
        let (matched_index, (position, _)) = candidate_closes
            .iter()
            .enumerate()
            .skip(candidate_index)
            .find(|(_, (_, close))| close == wanted_close)?;
        earliest.push(*position);
        candidate_index = matched_index + 1;
    }

    let mut latest = vec![0; wanted.len()];
    let mut candidate_index = candidate_closes.len();
    for (wanted_index, wanted_close) in wanted.iter().enumerate().rev() {
        let (matched_index, (position, _)) = candidate_closes[..candidate_index]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, (_, close))| close == wanted_close)?;
        latest[wanted_index] = *position;
        candidate_index = matched_index;
    }

    (earliest == latest).then_some(earliest)
}

#[cfg(test)]
mod tests {
    use super::{candidate_closer_targets, unique_ordered_matches};

    const PAIRS: [(char, char); 6] = [
        ('(', ')'),
        ('[', ']'),
        ('{', '}'),
        ('"', '"'),
        ('\'', '\''),
        ('`', '`'),
    ];

    #[test]
    fn candidate_pair_mapping_ignores_string_delimiters() {
        let candidate = r#"f(")")"#;
        let targets = candidate_closer_targets("f()", 2, &candidate[2..], PAIRS.into_iter(), ")")
            .expect("the existing closer maps to the outer call");

        assert_eq!(targets, vec![5]);
        assert_ne!(targets[0], 3);
        assert_eq!(candidate.as_bytes()[targets[0]], b')');

        let targets =
            candidate_closer_targets(r#"f(")"#, 3, &candidate[3..], PAIRS.into_iter(), ")")
                .expect("an incomplete source string still maps through the candidate opener");
        assert_eq!(targets, vec![5]);
    }

    #[test]
    fn nested_tail_closers_map_in_order() {
        let targets = candidate_closer_targets("f(())", 3, "x))", PAIRS.into_iter(), "))")
            .expect("both nested closers map in order");
        assert_eq!(targets, vec![4, 5]);
    }

    #[test]
    fn quote_tail_closers_map_to_candidate_quotes() {
        let cases = [
            (r#"echo "hel""#, 9, "lo\"", "\"", 11),
            ("echo 'hel'", 9, "lo'", "'", 11),
            ("echo `hel`", 9, "lo`", "`", 11),
        ];

        for (source, cursor, hint, tail, expected_close) in cases {
            let targets = candidate_closer_targets(source, cursor, hint, PAIRS.into_iter(), tail)
                .expect("the candidate's quote opener identifies its closer");

            assert_eq!(targets, vec![expected_close], "candidate: {source}{hint}");
        }
    }

    #[test]
    fn escaped_quote_is_not_a_candidate_quote_closer() {
        let source = r#"echo "hel\"""#;
        let targets = candidate_closer_targets(source, 11, "lo\"", PAIRS.into_iter(), "\"")
            .expect("the final unescaped quote closes the candidate string");

        assert_eq!(targets, vec![13]);
        assert_eq!(source.as_bytes()[10], b'"');
    }

    #[test]
    fn quote_and_outer_bracket_tail_closers_map_in_order() {
        let source = r#"f("hel")"#;
        let targets = candidate_closer_targets(source, 6, "lo\")", PAIRS.into_iter(), "\")")
            .expect("the quote and outer paren each have a unique candidate pair");

        assert_eq!(targets, vec![8, 9]);
    }

    #[test]
    fn interpolation_array_and_record_closers_map_in_order() {
        let source = r#"echo $"([1])""#;
        let targets =
            candidate_closer_targets(source, 10, " 2] | length)\"", PAIRS.into_iter(), "])\"")
                .expect("the candidate maps the array, interpolation, and string closers");
        assert_eq!(targets, vec![12, 22, 23]);

        let source = r#"echo $"([{x: 1}])""#;
        let targets = candidate_closer_targets(
            source,
            14,
            ", y: 2}] | length)\"",
            PAIRS.into_iter(),
            "}])\"",
        )
        .expect("the candidate maps record, array, interpolation, and string closers");
        assert_eq!(targets, vec![20, 21, 31, 32]);
    }

    #[test]
    fn interpolation_fake_closers_and_mismatches_are_safe() {
        let source = r#"echo $"([])""#;
        let targets = candidate_closer_targets(
            source,
            9,
            r#"")]} ", 1] | length)""#,
            PAIRS.into_iter(),
            "])\"",
        )
        .expect("closers inside the interpolation string do not match the source tail");
        assert_eq!(targets, vec![18, 28, 29]);

        let targets = candidate_closer_targets(
            r#"echo $"([1])""#,
            10,
            "} | length)\"",
            PAIRS.into_iter(),
            "])\"",
        );
        assert!(targets.is_none());
    }

    #[test]
    fn matching_rejects_non_unique_and_multibyte_closers() {
        assert!(unique_ordered_matches(")", &[(4, ')'), (5, ')')]).is_none());
        assert!(unique_ordered_matches("🎉", &[(4, '🎉')]).is_none());
    }
}
