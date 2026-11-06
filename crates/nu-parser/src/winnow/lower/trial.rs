//! Trial reads: reading something one way, looking at what that reported, and keeping or dropping
//! it, as nu's parser does wherever it tries a shape or a form and falls back.
//!
//! What a read reports goes on lists of the working set whose order is output: its errors, warnings
//! and compile errors, and it can leave scopes open (an unclosed closure's). A leftover error also
//! changes what gets compiled, as a closure compiles only while there are no errors at all. So a
//! read that is given up must leave exactly what nu's leaves.
//!
//! [`Trial`] holds the marks and does the restoring. Which lists to restore stays with each caller,
//! because nu's functions differ: `parse_oneof` drops everything a failed shape reported,
//! `parse_alias` drops only errors. The method a site calls is its policy.
//!
//! [`Lower::read_oneof`] is the one port of `parse_oneof`.

use nu_protocol::{
    BlockId, CompileError, ParseError, ParseWarning, Span, SyntaxShape,
    ast::Expression,
    engine::{ScopeFrame, StateWorkingSet},
};

use super::{Lower, Lowered, Unlowered};
use crate::parse_calls::accepts_any_bareword;
use crate::winnow::syntax::Span as WSpan;

/// How far the working set's lists had got when a read began.
#[derive(Clone, Copy)]
pub(super) struct Trial {
    errors: usize,
    warnings: usize,
    compile_errors: usize,
    scopes: usize,
}

impl Trial {
    /// Marks the lists as they stand, before the read.
    #[inline]
    pub(super) fn new(working_set: &StateWorkingSet) -> Self {
        Self {
            errors: working_set.parse_errors.len(),
            warnings: working_set.parse_warnings.len(),
            compile_errors: working_set.compile_errors.len(),
            scopes: working_set.delta.scope.len(),
        }
    }

    /// The errors reported since the mark.
    #[inline]
    pub(super) fn new_errors<'a>(&self, working_set: &'a StateWorkingSet) -> &'a [ParseError] {
        working_set
            .parse_errors
            .get(self.errors..)
            .unwrap_or_default()
    }

    /// Whether the read reported an error.
    #[inline]
    pub(super) fn failed(&self, working_set: &StateWorkingSet) -> bool {
        working_set.parse_errors.len() > self.errors
    }

    /// Where the earliest error since the mark starts, which is how `parse_oneof` measures how far
    /// a read got. Errors are not reported in source order, so it is not always the first one.
    pub(super) fn first_error(&self, working_set: &StateWorkingSet) -> Option<usize> {
        self.new_errors(working_set)
            .iter()
            .map(|error| error.span().start)
            .min()
    }

    /// Drops the errors since the mark and nothing else, as `parse_alias` and most of nu's readers
    /// do. The warnings and compile errors stay, and so does a scope left open.
    #[inline]
    pub(super) fn discard_errors(&self, working_set: &mut StateWorkingSet) {
        working_set.parse_errors.truncate(self.errors);
    }

    /// Takes off the scopes entered since the mark and left open.
    pub(super) fn close_scopes(&self, working_set: &mut StateWorkingSet) {
        working_set.delta.scope.truncate(self.scopes);
    }

    /// Drops everything the read reported and every scope it left open: it leaves no trace on these
    /// lists, as a shape `parse_oneof` gives up on. What else it added to the working set stays
    /// (see the module documentation of `lower`).
    pub(super) fn discard(&self, working_set: &mut StateWorkingSet) {
        working_set.parse_errors.truncate(self.errors);
        working_set.parse_warnings.truncate(self.warnings);
        working_set.compile_errors.truncate(self.compile_errors);
        self.close_scopes(working_set);
    }

    /// Takes what a best guess keeps off the lists, for [`Reported::restore`] if it stays the best
    /// guess: its errors, warnings and open scopes. Its compile errors go, as in `parse_oneof`, so
    /// the lists stand as at the mark and the next reading can't resolve a name through those
    /// scopes.
    pub(super) fn take(&self, working_set: &mut StateWorkingSet) -> Reported {
        let reported = Reported {
            errors: split_off_from(&mut working_set.parse_errors, self.errors),
            warnings: split_off_from(&mut working_set.parse_warnings, self.warnings),
            scopes: split_off_from(&mut working_set.delta.scope, self.scopes),
        };
        working_set.compile_errors.truncate(self.compile_errors);
        reported
    }

    /// Forgives a `break` or `continue` in `try`'s handler `closure`: compiled on its own, it is
    /// outside any loop, and `parse_internal_call` drops that error when it is the only one since
    /// the mark and the closure has no IR.
    pub(super) fn forgive_handler_break(
        &self,
        working_set: &mut StateWorkingSet,
        closure: BlockId,
    ) {
        if let Some([CompileError::NotInALoop { .. }]) =
            working_set.compile_errors.get(self.compile_errors..)
            && working_set.get_block(closure).ir_block.is_none()
        {
            working_set.compile_errors.truncate(self.compile_errors);
        }
    }
}

/// `list` from `at` on, or nothing when it is shorter than that.
fn split_off_from<T>(list: &mut Vec<T>, at: usize) -> Vec<T> {
    list.split_off(at.min(list.len()))
}

/// What a read reported and left open, off the lists ([`Trial::take`]).
pub(super) struct Reported {
    errors: Vec<ParseError>,
    warnings: Vec<ParseWarning>,
    scopes: Vec<ScopeFrame>,
}

impl Reported {
    /// Puts it all back at the end of the lists, in the order it was reported.
    pub(super) fn restore(self, working_set: &mut StateWorkingSet) {
        working_set.parse_errors.extend(self.errors);
        working_set.parse_warnings.extend(self.warnings);
        working_set.delta.scope.extend(self.scopes);
    }
}

/// Whether the errors of a `oneof` shape's best guess, whose last error is `last`, are kept even
/// when the guess got no further than the item's start (`parse_oneof`): an expected value is dropped
/// unless the shape is a block, a closure or an expression and the error is inside the item, `item`
/// the span of the item the shape read last; any other error is kept.
fn best_guess_propagates(
    last: Option<&ParseError>,
    shape: &SyntaxShape,
    item: impl FnOnce() -> Span,
) -> bool {
    match last {
        Some(
            ParseError::Expected(_, error_span) | ParseError::ExpectedWithStringMsg(_, error_span),
        ) => {
            matches!(
                shape,
                SyntaxShape::Block | SyntaxShape::Closure(_) | SyntaxShape::Expression
            ) && *error_span != item()
        }
        _ => true,
    }
}

impl Lower<'_, '_, '_> {
    /// `parse_oneof`: the first of `shapes` that `read` reads without an error, whose value and
    /// items read are the result, starting at the item `first_item`. `read` gives a value and the
    /// index of the item after those it read.
    ///
    /// As there, the shapes that take nearly any bare word go last unless the item starts with `$`,
    /// `(` or `{`, and what each shape reported is dropped before the next is tried. When none reads
    /// it, the value is the guess whose first error is the furthest along, with its errors, warnings
    /// and open scopes, if that error is past the item's start or its last error is not just an
    /// expected value ([`best_guess_propagates`], `last_item` naming the item a shape read last
    /// from the index it read to). Otherwise the item is garbage, with an error that lists the
    /// shapes. Either way the items read are those of the last shape that read any, or `next` if
    /// none did.
    ///
    /// A shape the lowering gives back for an error is one nu's value parser refuses too, so the
    /// next is tried, and with no clean reading the statement is given back: the lowering can't
    /// rank a guess it didn't make. A shape it doesn't support gives the statement back at once,
    /// because nu may take that shape, which a later one's clean reading must not hide.
    pub(super) fn read_oneof(
        &mut self,
        first_item: WSpan,
        shapes: &[SyntaxShape],
        mut next: usize,
        mut read: impl FnMut(&mut Self, &SyntaxShape) -> Lowered<(Expression, usize)>,
        last_item: impl Fn(&Self, usize) -> Span,
    ) -> Lowered<(Expression, usize)> {
        let defer_barewords = !matches!(
            self.text(first_item).as_bytes().first(),
            Some(b'$' | b'(' | b'{')
        );
        let deferred = |shape: &&SyntaxShape| defer_barewords && accepts_any_bareword(shape);
        let ordered = shapes
            .iter()
            .filter(|shape| !deferred(shape))
            .chain(shapes.iter().filter(|shape| deferred(shape)));
        let start = self.span(first_item);
        let trial = Trial::new(self.working_set);
        let mut best_guess = None;
        let mut furthest = 0;
        let mut propagate = false;
        let mut given_back = false;
        for shape in ordered {
            match read(self, shape) {
                Ok((value, after)) => {
                    let Some(first) = trial.first_error(self.working_set) else {
                        return Ok((value, after));
                    };
                    next = after;
                    let reported = trial.take(self.working_set);
                    if first > furthest {
                        let last = || last_item(self, after);
                        propagate = best_guess_propagates(reported.errors.last(), shape, last);
                        furthest = first;
                        best_guess = Some((value, reported));
                    }
                }
                Err(unsupported @ Unlowered::Unsupported(_)) => {
                    trial.discard(self.working_set);
                    return Err(unsupported);
                }
                Err(Unlowered::Error) => {
                    given_back = true;
                    trial.discard(self.working_set);
                }
            }
        }
        if given_back {
            return Err(Unlowered::Error);
        }
        match best_guess {
            Some((value, reported)) if furthest > start.start || propagate => {
                reported.restore(self.working_set);
                Ok((value, next))
            }
            _ => {
                self.working_set.error(ParseError::ExpectedWithStringMsg(
                    format!("one of a list of accepted shapes: {shapes:?}"),
                    start,
                ));
                Ok((Expression::garbage(self.working_set, start), next))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nu_protocol::{
        ReportMode, Type,
        ast::{Block, Expr},
        engine::EngineState,
        ir::IrBlock,
    };

    use super::*;

    fn error(start: usize) -> ParseError {
        ParseError::UnknownState("trial".into(), Span::new(start, start + 1))
    }

    fn warning() -> ParseWarning {
        ParseWarning::Deprecated {
            dep_type: "trial".into(),
            label: "trial".into(),
            span: Span::unknown(),
            help: None,
            report_mode: ReportMode::FirstUse,
        }
    }

    fn compile_error() -> CompileError {
        CompileError::NotInALoop {
            msg: "trial".into(),
            span: None,
        }
    }

    /// One of each on every list, and a scope left open, as a read with errors reports them.
    fn report_all(working_set: &mut StateWorkingSet, error_start: usize) {
        working_set.error(error(error_start));
        working_set.warning(warning());
        working_set.compile_errors.push(compile_error());
        working_set.enter_scope();
    }

    /// How long each list is: errors, warnings, compile errors, scopes.
    fn lengths(working_set: &StateWorkingSet) -> [usize; 4] {
        [
            working_set.parse_errors.len(),
            working_set.parse_warnings.len(),
            working_set.compile_errors.len(),
            working_set.delta.scope.len(),
        ]
    }

    /// Runs `test` on a working set that already reported something, so a trial that rolled back
    /// too far would show. Gives the lengths it started with.
    fn with_working_set(test: impl FnOnce(&mut StateWorkingSet, [usize; 4])) {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);
        report_all(&mut working_set, 1000);
        let before = lengths(&working_set);
        test(&mut working_set, before);
    }

    #[test]
    fn a_new_trial_has_nothing_to_report() {
        with_working_set(|working_set, _| {
            let trial = Trial::new(working_set);
            assert!(!trial.failed(working_set));
            assert!(trial.new_errors(working_set).is_empty());
            assert_eq!(trial.first_error(working_set), None);
        });
    }

    #[test]
    fn first_error_is_the_earliest_not_the_first_reported() {
        with_working_set(|working_set, _| {
            let trial = Trial::new(working_set);
            working_set.error(error(20));
            working_set.error(error(5));
            assert!(trial.failed(working_set));
            assert_eq!(trial.new_errors(working_set).len(), 2);
            assert_eq!(trial.first_error(working_set), Some(5));
        });
    }

    #[test]
    fn a_failed_try_leaves_no_trace() {
        with_working_set(|working_set, before| {
            let trial = Trial::new(working_set);
            report_all(working_set, 1);
            report_all(working_set, 2);
            trial.discard(working_set);
            assert_eq!(lengths(working_set), before);
        });
    }

    #[test]
    fn discard_errors_leaves_the_other_lists() {
        with_working_set(|working_set, [errors, warnings, compile_errors, scopes]| {
            let trial = Trial::new(working_set);
            report_all(working_set, 1);
            trial.discard_errors(working_set);
            assert_eq!(
                lengths(working_set),
                [errors, warnings + 1, compile_errors + 1, scopes + 1]
            );
        });
    }

    #[test]
    fn close_scopes_leaves_the_reports() {
        with_working_set(|working_set, [errors, warnings, compile_errors, scopes]| {
            let trial = Trial::new(working_set);
            report_all(working_set, 1);
            trial.close_scopes(working_set);
            assert_eq!(
                lengths(working_set),
                [errors + 1, warnings + 1, compile_errors + 1, scopes]
            );
        });
    }

    #[test]
    fn take_leaves_the_lists_as_at_the_mark() {
        with_working_set(|working_set, before| {
            let trial = Trial::new(working_set);
            report_all(working_set, 1);
            let reported = trial.take(working_set);
            assert_eq!(lengths(working_set), before);
            assert!(!trial.failed(working_set));
            drop(reported);
        });
    }

    #[test]
    fn restore_gives_back_all_but_the_compile_errors_in_order() {
        with_working_set(|working_set, [errors, warnings, compile_errors, scopes]| {
            let trial = Trial::new(working_set);
            working_set.error(error(7));
            report_all(working_set, 3);
            let reported = trial.take(working_set);
            reported.restore(working_set);
            assert_eq!(
                lengths(working_set),
                [errors + 2, warnings + 1, compile_errors, scopes + 1]
            );
            let starts: Vec<_> = trial
                .new_errors(working_set)
                .iter()
                .map(|error| error.span().start)
                .collect();
            assert_eq!(starts, [7, 3]);
        });
    }

    #[test]
    fn marks_past_the_lists_read_as_nothing() {
        with_working_set(|working_set, _| {
            let trial = Trial::new(working_set);
            working_set.parse_errors.clear();
            working_set.parse_warnings.clear();
            working_set.delta.scope.clear();
            assert!(!trial.failed(working_set));
            assert!(trial.new_errors(working_set).is_empty());
            let reported = trial.take(working_set);
            assert!(reported.errors.is_empty());
            assert!(reported.warnings.is_empty());
            assert!(reported.scopes.is_empty());
            trial.discard(working_set);
        });
    }

    #[test]
    fn a_handler_break_is_forgiven_only_alone_and_without_ir() {
        with_working_set(|working_set, [_, _, compile_errors, _]| {
            let closure = working_set.add_block(Arc::new(Block::new()));

            let trial = Trial::new(working_set);
            working_set.compile_errors.push(compile_error());
            trial.forgive_handler_break(working_set, closure);
            assert_eq!(working_set.compile_errors.len(), compile_errors);

            working_set.compile_errors.push(compile_error());
            working_set.compile_errors.push(compile_error());
            trial.forgive_handler_break(working_set, closure);
            assert_eq!(working_set.compile_errors.len(), compile_errors + 2);
            working_set.compile_errors.truncate(compile_errors);

            let mut compiled = Block::new();
            compiled.ir_block = Some(IrBlock {
                instructions: Vec::new(),
                spans: Vec::new(),
                data: Arc::new([]),
                comments: Vec::new(),
                register_count: 0,
                file_count: 0,
                scope_regions: Vec::new(),
            });
            let compiled = working_set.add_block(Arc::new(compiled));
            working_set.compile_errors.push(compile_error());
            trial.forgive_handler_break(working_set, compiled);
            assert_eq!(working_set.compile_errors.len(), compile_errors + 1);
        });
    }

    /// What a fake reader does with a shape, for [`read_shapes`].
    #[derive(Clone, Copy)]
    enum Reading {
        Clean,
        /// A read that reports one of everything, its error at this offset.
        Fails(usize),
        /// As `Fails`, twice over, so its guess shows in how many errors it reported.
        FailsTwice(usize),
        /// As `Fails`, its last error `Expected` over the whole item.
        ExpectedOverItem(usize),
        /// As `Fails`, its last error `Expected` over a part of the item.
        ExpectedInside(usize),
        GivesBack,
        Unsupported,
    }

    /// [`Lower::read_oneof`] over `shapes` for the item in `source`, which leading blanks move off
    /// offset 0, each shape read as `reading` says, on a working set that already reported
    /// something. `check` gets the lengths it started with, the result, and how many shapes were
    /// read.
    fn read_shapes(
        source: &str,
        shapes: &[SyntaxShape],
        reading: impl Fn(&SyntaxShape) -> Reading,
        check: impl FnOnce(&mut StateWorkingSet, [usize; 4], Lowered<(Expression, usize)>, usize),
    ) {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);
        report_all(&mut working_set, 1000);
        let before = lengths(&working_set);
        let start = source.len() - source.trim_start().len();
        let item = Span::new(start, source.len());
        let mut reads = 0;
        let result = Lower::new(&mut working_set, source, 0).read_oneof(
            WSpan::new(start, source.len()),
            shapes,
            1,
            |this, shape| {
                reads += 1;
                let value = Expression::new(
                    this.working_set,
                    Expr::Nothing,
                    Span::unknown(),
                    Type::Nothing,
                );
                match reading(shape) {
                    Reading::Clean => Ok((value, 10)),
                    Reading::Fails(start) => {
                        report_all(this.working_set, start);
                        Ok((value, 20))
                    }
                    Reading::FailsTwice(start) => {
                        report_all(this.working_set, start);
                        report_all(this.working_set, start);
                        Ok((value, 20))
                    }
                    Reading::ExpectedOverItem(start) => {
                        report_all(this.working_set, start);
                        this.working_set.error(ParseError::Expected("trial", item));
                        Ok((value, 30))
                    }
                    Reading::ExpectedInside(start) => {
                        report_all(this.working_set, start);
                        let inside = Span::new(item.start + 1, item.end);
                        this.working_set
                            .error(ParseError::Expected("trial", inside));
                        Ok((value, 30))
                    }
                    Reading::GivesBack => {
                        report_all(this.working_set, 2);
                        Err(Unlowered::Error)
                    }
                    Reading::Unsupported => {
                        report_all(this.working_set, 2);
                        Err(Unlowered::Unsupported("trial"))
                    }
                }
            },
            |_, _| item,
        );
        check(&mut working_set, before, result, reads);
    }

    fn is_garbage(result: &Lowered<(Expression, usize)>) -> bool {
        matches!(result, Ok((value, _)) if matches!(value.expr, Expr::Garbage))
    }

    const INT_THEN_FLOAT: [SyntaxShape; 2] = [SyntaxShape::Int, SyntaxShape::Float];

    #[test]
    fn oneof_takes_the_first_clean_shape_and_forgets_the_others() {
        read_shapes(
            "word",
            &INT_THEN_FLOAT,
            |shape| match shape {
                SyntaxShape::Int => Reading::Fails(1),
                _ => Reading::Clean,
            },
            |working_set, before, result, reads| {
                assert_eq!(reads, 2);
                assert!(matches!(result, Ok((_, 10))));
                assert_eq!(lengths(working_set), before);
            },
        );
    }

    #[test]
    fn oneof_stops_at_the_first_clean_shape() {
        read_shapes(
            "word",
            &INT_THEN_FLOAT,
            |_| Reading::Clean,
            |_, _, _, reads| assert_eq!(reads, 1),
        );
    }

    #[test]
    fn oneof_keeps_the_furthest_guess_with_its_reports_and_scopes() {
        read_shapes(
            "word",
            &INT_THEN_FLOAT,
            |shape| match shape {
                SyntaxShape::Int => Reading::Fails(1),
                _ => Reading::Fails(3),
            },
            |working_set, [errors, warnings, compile_errors, scopes], result, reads| {
                assert_eq!(reads, 2);
                assert!(matches!(result, Ok((_, 20))));
                // One shape's reports, not both, and no compile errors of either.
                assert_eq!(
                    lengths(working_set),
                    [errors + 1, warnings + 1, compile_errors, scopes + 1]
                );
                assert_eq!(
                    working_set.parse_errors.last().map(|e| e.span().start),
                    Some(3)
                );
            },
        );
    }

    #[test]
    fn oneof_keeps_the_first_of_two_equally_far_guesses() {
        read_shapes(
            "word",
            &INT_THEN_FLOAT,
            |shape| match shape {
                SyntaxShape::Int => Reading::Fails(2),
                _ => Reading::FailsTwice(2),
            },
            |working_set, [errors, ..], _, _| {
                assert_eq!(working_set.parse_errors.len(), errors + 1);
            },
        );
    }

    /// Garbage, the error listing the shapes the only thing reported.
    fn assert_garbage(
        working_set: &StateWorkingSet,
        [errors, warnings, compile_errors, scopes]: [usize; 4],
        result: &Lowered<(Expression, usize)>,
    ) {
        assert!(is_garbage(result));
        assert_eq!(
            lengths(working_set),
            [errors + 1, warnings, compile_errors, scopes]
        );
        assert!(matches!(
            working_set.parse_errors.last(),
            Some(ParseError::ExpectedWithStringMsg(..))
        ));
    }

    #[test]
    fn oneof_reads_as_garbage_when_every_read_fails_at_offset_zero() {
        read_shapes(
            "word",
            &INT_THEN_FLOAT,
            |_| Reading::Fails(0),
            |working_set, before, result, _| assert_garbage(working_set, before, &result),
        );
    }

    #[test]
    fn oneof_reads_as_garbage_when_the_guess_only_expected_a_value_at_the_item() {
        read_shapes(
            "  word",
            &INT_THEN_FLOAT,
            |_| Reading::ExpectedOverItem(2),
            |working_set, before, result, _| assert_garbage(working_set, before, &result),
        );
    }

    #[test]
    fn oneof_reads_as_garbage_for_an_expected_value_inside_the_item_of_a_plain_shape() {
        read_shapes(
            "  word",
            &INT_THEN_FLOAT,
            |_| Reading::ExpectedInside(2),
            |working_set, before, result, _| assert_garbage(working_set, before, &result),
        );
    }

    #[test]
    fn oneof_keeps_an_expected_value_inside_the_item_of_a_block() {
        read_shapes(
            "  word",
            &[SyntaxShape::Block, SyntaxShape::Float],
            |shape| match shape {
                SyntaxShape::Block => Reading::ExpectedInside(2),
                _ => Reading::Fails(1),
            },
            |working_set, _, result, _| {
                assert!(matches!(result, Ok((_, 20))));
                assert!(matches!(
                    working_set.parse_errors.last(),
                    Some(ParseError::Expected(..))
                ));
            },
        );
    }

    #[test]
    fn oneof_keeps_an_expected_value_past_the_item_start() {
        read_shapes(
            "word",
            &INT_THEN_FLOAT,
            |_| Reading::ExpectedInside(3),
            |working_set, [errors, ..], result, _| {
                assert!(matches!(result, Ok((_, 30))));
                assert!(!is_garbage(&result));
                assert_eq!(working_set.parse_errors.len(), errors + 2);
            },
        );
    }

    #[test]
    fn oneof_keeps_any_other_error_at_the_item_start() {
        read_shapes(
            "  word",
            &INT_THEN_FLOAT,
            |_| Reading::Fails(2),
            |working_set, [errors, warnings, compile_errors, scopes], result, _| {
                assert!(matches!(result, Ok((_, 20))));
                assert_eq!(
                    lengths(working_set),
                    [errors + 1, warnings + 1, compile_errors, scopes + 1]
                );
            },
        );
    }

    #[test]
    fn oneof_goes_on_after_the_items_of_the_last_shape_that_read_any() {
        read_shapes(
            "word",
            &INT_THEN_FLOAT,
            |shape| match shape {
                SyntaxShape::Int => Reading::Fails(3),
                _ => Reading::ExpectedOverItem(1),
            },
            |_, _, result, _| assert!(matches!(result, Ok((_, 30)))),
        );
    }

    #[test]
    fn oneof_goes_on_after_the_first_item_when_there_are_no_shapes() {
        read_shapes(
            "word",
            &[],
            |_| Reading::Clean,
            |working_set, before, result, reads| {
                assert_eq!(reads, 0);
                assert!(matches!(result, Ok((_, 1))));
                assert_garbage(working_set, before, &result);
            },
        );
    }

    #[test]
    fn oneof_stops_at_an_unsupported_shape_and_leaves_no_trace() {
        read_shapes(
            "word",
            &INT_THEN_FLOAT,
            |shape| match shape {
                SyntaxShape::Int => Reading::Unsupported,
                _ => Reading::Clean,
            },
            |working_set, before, result, reads| {
                assert_eq!(reads, 1);
                assert!(matches!(result, Err(Unlowered::Unsupported("trial"))));
                assert_eq!(lengths(working_set), before);
            },
        );
    }

    #[test]
    fn oneof_goes_on_after_a_shape_given_back_for_an_error() {
        read_shapes(
            "word",
            &INT_THEN_FLOAT,
            |shape| match shape {
                SyntaxShape::Int => Reading::GivesBack,
                _ => Reading::Clean,
            },
            |working_set, before, result, reads| {
                assert_eq!(reads, 2);
                assert!(matches!(result, Ok((_, 10))));
                assert_eq!(lengths(working_set), before);
            },
        );
    }

    #[test]
    fn oneof_gives_back_when_a_shape_gave_back_and_none_read_clean() {
        read_shapes(
            "word",
            &INT_THEN_FLOAT,
            |shape| match shape {
                SyntaxShape::Int => Reading::GivesBack,
                _ => Reading::Fails(3),
            },
            |working_set, before, result, _| {
                assert!(matches!(result, Err(Unlowered::Error)));
                assert_eq!(lengths(working_set), before);
            },
        );
    }

    /// Which shape comes first shows in the result, `String` reading clean and `Int` not.
    fn string_read_first(source: &str) -> bool {
        let shapes = [SyntaxShape::String, SyntaxShape::Int];
        let mut string_first = false;
        read_shapes(
            source,
            &shapes,
            |shape| match shape {
                SyntaxShape::String => Reading::Clean,
                _ => Reading::Fails(1),
            },
            |_, _, _, reads| string_first = reads == 1,
        );
        string_first
    }

    #[test]
    fn oneof_tries_a_bareword_shape_last() {
        assert!(!string_read_first("word"));
    }

    #[test]
    fn oneof_keeps_the_written_order_for_an_item_that_starts_a_value() {
        assert!(string_read_first("$word"));
        assert!(string_read_first("(word)"));
        assert!(string_read_first("{word}"));
    }
}
