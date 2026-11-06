//! An item that holds an error (or an `exit`) must not disappear once something has read it, even
//! when the consumer doesn't return that item. See <https://github.com/nushell/nushell/issues/19463>.

use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

/// `failing $n` streams 1, 2, 3 and raises `boom` on item `$n`. `exiting $n` calls `exit 7` there
/// instead. Each item only runs when a consumer pulls it. `errval` returns `boom` as a single value
/// rather than raising it.
const STREAMS: &str = "
    def failing [n: int] { [1 2 3] | each {|x| if $x == $n { error make {msg: boom} }; $x } }
    def exiting [n: int] { [1 2 3] | each {|x| if $x == $n { exit 7 }; $x } }
    def errval [] { [1] | each {|y| error make {msg: boom} } | reduce {|i a| $i } }
";

/// Handles each failed item where it is, which shows what became of the items around it.
const RECOVER: &str = "each {|c| try { $c | ignore; $c } catch { 'caught' } }";

#[rstest]
// Commands that read every item and return some of them.
#[case::last_first_item("failing 1 | last")]
#[case::last_middle_item("failing 2 | last")]
#[case::last_returned_item("failing 3 | last")]
#[case::last_n("failing 1 | last 2")]
#[case::get_earlier_item("failing 1 | get 2")]
#[case::get_later_item("failing 3 | get 0")]
#[case::get_optional("failing 1 | get 2?")]
#[case::get_missing_optional("failing 1 | get 5?")]
#[case::get_nested(
    "[{a: 1} {a: 2}] | each {|r| if $r.a == 1 { error make {msg: boom} }; $r } | get 1.a"
)]
#[case::subexpression_cell_path("(failing 1).2")]
#[case::drop_tail("failing 3 | drop")]
#[case::drop_all("failing 1 | drop 5")]
#[case::slice_relative("failing 1 | slice (-1)..")]
#[case::reverse_then_first("failing 3 | reverse | first")]
#[case::reverse_then_last("failing 3 | reverse | last")]
#[case::shuffle_then_first("failing 3 | shuffle | first")]
#[case::sort_by_then_last("failing 1 | sort-by {|x| 1} | last")]
// Filters that stream on, leaving some items out.
#[case::skip("failing 1 | skip 1")]
#[case::skip_past_the_end("failing 1 | skip 5")]
#[case::skip_then_length("failing 1 | skip 1 | length")]
#[case::skip_while("failing 1 | skip while {|x| true}")]
#[case::skip_until("failing 1 | skip until {|x| false}")]
#[case::take_while("failing 1 | take while {|x| false}")]
#[case::take_until("failing 1 | take until {|x| true}")]
#[case::take_while_include("failing 1 | take while --include 1 {|x| false}")]
#[case::select_row("failing 1 | select 1")]
#[case::drop_nth("failing 1 | drop nth 0")]
#[case::slice("failing 1 | slice 1..2")]
#[case::every("failing 2 | every 2")]
#[case::every_skip("failing 1 | every 2 --skip")]
#[case::update_replaced_row("failing 1 | update 0 9")]
#[case::update_replaced_row_closure("failing 1 | update 0 {|| 9}")]
#[case::upsert_replaced_row("failing 1 | upsert 0 9")]
#[case::upsert_replaced_row_closure("failing 1 | upsert 0 {|| 9}")]
#[case::window_gap("failing 2 | window 1 --stride 2")]
#[case::window_gap_incomplete("failing 3 | window 2 --stride 2")]
#[case::window_overlap_incomplete("[1] | each {|x| error make {msg: boom} } | window 2")]
#[case::split_list_separator("failing 1 | split list {|x| true}")]
// Commands that would otherwise bury the error in a list or record the consumer never opens.
#[case::enumerate("failing 1 | enumerate | last")]
#[case::wrap("failing 1 | wrap a | last")]
#[case::zip("failing 1 | zip [a b c] | last")]
#[case::chunks("failing 1 | chunks 2 | last")]
#[case::chunk_by("failing 1 | chunk-by {|x| $x > 2 } | last")]
#[case::window("failing 1 | window 2 | last")]
#[case::split_list("failing 1 | split list 2 | last")]
#[case::collecting_a_wrapper("let r = (failing 1 | enumerate); 'continued'")]
#[case::group_by("failing 1 | group-by {|x| 1 }")]
#[case::transpose("failing 1 | transpose")]
#[case::rotate("failing 1 | rotate")]
#[case::combinations("failing 1 | combinations 2")]
#[case::permutations("failing 1 | permutations")]
#[case::to_md("failing 1 | to md")]
#[case::to_html("failing 1 | to html")]
#[case::to_text("failing 1 | to text")]
#[case::to_text_no_newline("failing 1 | to text --no-newline")]
#[case::to_text_statement("failing 1 | to text; 'continued'")]
#[case::update_cells("[{a: 1}] | each {|r| error make {msg: boom} } | update cells {|v| 1 }")]
// `;` drains the pipeline before it, and `drain` has to look at a single value as well.
#[case::first_statement("failing 1 | first; 'continued'")]
#[case::last_statement("failing 3 | last; 'continued'")]
#[case::reduce_statement(
    "[1] | each {|x| error make {msg: boom} } | reduce {|i a| $i }; 'continued'"
)]
// A closure's own failure is no less of an error than one in the input.
#[case::take_while_closure("[1 2 3] | take while {|x| error make {msg: boom} }")]
#[case::take_until_closure("[1 2 3] | take until {|x| error make {msg: boom} }")]
#[case::skip_while_closure("[1 2 3] | skip while {|x| error make {msg: boom} }")]
#[case::skip_until_closure("[1 2 3] | skip until {|x| error make {msg: boom} }")]
#[case::chunk_by_closure("[1 2 3] | chunk-by {|x| error make {msg: boom} }")]
#[case::split_list_closure("[1 2 3] | split list {|x| error make {msg: boom} }")]
// A closure that returns an error fails just like one that raises it.
#[case::take_while_returned("[1 2 3] | take while {|x| errval }")]
#[case::take_until_returned("[1 2 3] | take until {|x| errval }")]
#[case::skip_while_returned("[1 2 3] | skip while {|x| errval }")]
#[case::skip_until_returned("[1 2 3] | skip until {|x| errval }")]
#[case::chunk_by_returned("[1 2 3] | chunk-by {|x| errval }")]
#[case::split_list_returned("[1 2 3] | split list {|x| errval }")]
#[case::where_returned("[1 2 3] | where {|x| errval }")]
// A statement in a closure that discards an error input is a `;` like any other.
#[case::each_discarding_param("failing 1 | each {|y| $y; 'ok' }")]
#[case::each_discarding_in("failing 1 | each { $in; 'ok' }")]
// An item with no partner, or that isn't text, is still read.
#[case::zip_other_ended("failing 2 | zip [a]")]
#[case::lines("failing 1 | lines")]
// A byte stream carries the error, and each way of reading it has to unpack it.
#[case::to_text_lines("failing 2 | to text | lines")]
#[case::to_text_no_newline_lines("failing 2 | to text --no-newline | lines")]
#[case::to_text_is_empty("failing 1 | to text | is-empty")]
#[case::to_text_is_not_empty("failing 1 | to text | is-not-empty")]
#[case::to_text_from_csv("failing 2 | to text | from csv")]
#[case::to_text_from_json_objects("failing 2 | to text | from json --objects")]
#[case::to_text_binary_chunks("failing 2 | to text | into binary | each { $in }")]
#[case::to_text_bytes_split("failing 2 | to text | into binary | bytes split 0x[0a]")]
#[case::to_text_binary_first("failing 1 | to text | into binary | first")]
#[case::to_text_binary_last("failing 2 | to text | into binary | last 1")]
#[case::to_text_binary_length("failing 2 | to text | into binary | length")]
#[case::to_text_binary_chunks_n("failing 2 | to text | into binary | chunks 2")]
#[case::to_text_binary_starts_with("failing 1 | to text | into binary | bytes starts-with 0x[31]")]
#[case::to_text_binary_ends_with("failing 2 | to text | into binary | bytes ends-with 0x[31]")]
#[case::to_text_binary_format_bits("failing 2 | to text | into binary | format bits")]
#[case::to_text_binary_msgpack_objects(
    "failing 2 | to text | into binary | from msgpack --objects"
)]
fn error_is_raised(#[case] pipeline: &str) -> Result {
    let err = test()
        .run(format!("{STREAMS}\n{pipeline}"))
        .expect_shell_error()?;
    assert_contains("boom", format!("{err:?}"));
    Ok(())
}

#[rstest]
#[case::first("try { failing 1 | first } catch { 'caught' }")]
#[case::last("try { failing 1 | last } catch { 'caught' }")]
#[case::last_returned_item("try { failing 3 | last } catch { 'caught' }")]
#[case::get("try { failing 1 | get 2 } catch { 'caught' }")]
#[case::skip("try { failing 1 | skip 1 | to json } catch { 'caught' }")]
#[case::take_while_closure(
    "try { [1] | take while {|x| error make {msg: boom} } | to json } catch { 'caught' }"
)]
fn catch_sees_the_error(#[case] pipeline: &str) -> Result {
    test()
        .run(format!("{STREAMS}\n{pipeline}"))
        .expect_value_eq("caught")
}

#[rstest]
#[case::first("failing 3 | first", test_value!(1))]
#[case::first_n("failing 3 | first 2", test_value!([1, 2]))]
#[case::take("failing 3 | take 2", test_value!([1, 2]))]
#[case::take_while("failing 3 | take while {|x| $x < 2 }", test_value!([1]))]
#[case::last_zero("failing 1 | last 0", test_value!([]))]
#[case::skip_then_first("failing 3 | skip 1 | first", test_value!(2))]
#[case::select_row("failing 3 | select 0", test_value!([1]))]
#[case::drop_nth_open_range("failing 3 | drop nth 1..", test_value!([1]))]
#[case::slice("failing 3 | slice 0..0", test_value!([1]))]
#[case::slice_up_to_the_end_of_the_range("failing 3 | slice 0..1", test_value!([1, 2]))]
#[case::take_while_include("failing 3 | take while --include 1 {|x| $x < 2 }", test_value!([1, 2]))]
#[case::zip_other_ended("failing 3 | zip [a]", test_value!([[1, "a"]]))]
// A command that streams doesn't read further than its next output needs, so collecting would run
// item 3 and raise its error.
#[case::streams_skip("failing 3 | skip 0 | first", test_value!(1))]
#[case::streams_skip_while("failing 3 | skip while {|x| false } | first", test_value!(1))]
#[case::streams_skip_until("failing 3 | skip until {|x| true } | first", test_value!(1))]
#[case::streams_take_while("failing 3 | take while {|x| true } | first", test_value!(1))]
#[case::streams_take_until("failing 3 | take until {|x| false } | first", test_value!(1))]
#[case::streams_where("failing 3 | where {|x| true } | first", test_value!(1))]
#[case::streams_every("failing 3 | every 1 | first", test_value!(1))]
#[case::streams_update("failing 3 | update 0 9 | first", test_value!(9))]
#[case::streams_upsert("failing 3 | upsert 0 9 | first", test_value!(9))]
#[case::streams_enumerate("failing 3 | enumerate | first", test_value!({index: 0, item: 1}))]
#[case::streams_wrap("failing 3 | wrap a | first", test_value!({a: 1}))]
#[case::streams_zip("failing 3 | zip [a b c] | first", test_value!([1, "a"]))]
#[case::streams_chunks("failing 3 | chunks 1 | first", test_value!([1]))]
#[case::streams_window("failing 3 | window 1 | first", test_value!([1]))]
#[case::streams_window_gap("failing 3 | window 1 --stride 2 | first", test_value!([1]))]
#[case::streams_chunk_by("failing 3 | chunk-by {|x| $x } | first", test_value!([1]))]
#[case::streams_split_list("failing 3 | split list 2 | first", test_value!([1]))]
#[case::streams_lines("failing 3 | each { into string } | lines | first", test_value!("1"))]
#[case::streams_to_text_lines("failing 3 | to text | lines | first", test_value!("1"))]
#[case::streams_do_ignore_errors("do --ignore-errors { failing 3 } | first", test_value!(1))]
#[case::streams_do_ignore_errors_to_text(
    "do --ignore-errors { failing 3 | to text } | lines | first",
    test_value!("1")
)]
fn item_that_never_ran_stays_unevaluated(
    #[case] pipeline: &str,
    #[case] expected: Value,
) -> Result {
    test()
        .run(format!("{STREAMS}\n{pipeline}"))
        .expect_value_eq(expected)
}

#[rstest]
// An error from the skipped items doesn't take the place of an item that wasn't skipped.
#[case::slice("failing 1 | slice 1..2", test_value!(["caught", 2, 3]))]
#[case::window_gap("failing 2 | window 1 --stride 2", test_value!([[1], "caught", [3]]))]
// An error item doesn't end the skipping.
#[case::skip_while("failing 1 | skip while {|x| true }", test_value!(["caught"]))]
#[case::skip_until("failing 1 | skip until {|x| false }", test_value!(["caught"]))]
// An error kept from the last window isn't reported again at the end.
#[case::window_overlap("failing 3 | window 2", test_value!([[1, 2], "caught"]))]
// Nothing is pulled past the end of the taking, or the exit on the next item would run.
#[case::take_while_failure("exiting 2 | take while {|x| error make {msg: boom} }", test_value!(["caught"]))]
#[case::take_until_failure("exiting 2 | take until {|x| error make {msg: boom} }", test_value!(["caught"]))]
#[case::take_while_failure_include(
    "exiting 3 | take while --include 1 {|x| error make {msg: boom} }",
    test_value!(["caught", 2])
)]
fn recovering_shows_the_items_around_an_error(
    #[case] pipeline: &str,
    #[case] expected: Value,
) -> Result {
    test()
        .run(format!("{STREAMS}\n{pipeline} | {RECOVER}"))
        .expect_value_eq(expected)
}

#[test]
fn closure_can_still_handle_an_error_item() -> Result {
    let pipeline = "failing 1 | each {|y| try { $y | ignore; 'ok' } catch { 'caught' } }";
    test()
        .run(format!("{STREAMS}\n{pipeline}"))
        .expect_value_eq(["caught", "ok", "ok"])
}

#[test]
fn describe_still_shows_an_error_item() -> Result {
    test()
        .run(format!("{STREAMS}\nfailing 1 | describe"))
        .expect_value_eq("list<oneof<error, int>> (stream)")
}

#[rstest]
// Discarded by the consumer.
#[case::last("exiting 1 | last")]
#[case::get("exiting 1 | get 2")]
#[case::subexpression_cell_path("(exiting 1).2")]
#[case::skip("exiting 1 | skip 1")]
#[case::enumerate("exiting 1 | enumerate | last")]
#[case::first_statement("exiting 1 | first; 'continued'")]
#[case::abort("[1 2 3] | each {|x| if $x == 1 { exit --abort 7 }; $x } | last")]
#[case::to_text("exiting 1 | to text")]
#[case::describe("exiting 1 | describe")]
#[case::describe_detailed("exiting 1 | describe --detailed")]
// Handed to a closure that ignores its input: only `exit` skips the closure, since `catch` can't
// handle it anyway.
#[case::each("exiting 1 | each {|x| 1 }")]
#[case::each_no_params("exiting 1 | each { 1 }")]
#[case::each_flatten("exiting 1 | each --flatten {|x| [1] }")]
#[case::par_each("exiting 1 | par-each {|x| 1 }")]
#[case::where_("exiting 1 | where {|x| false }")]
#[case::any("exiting 1 | any {|x| false }")]
#[case::all("exiting 1 | all {|x| true }")]
#[case::reduce("exiting 1 | reduce --fold 0 {|i a| 1 }")]
// Raised by a closure whose failures used to count as `false`.
#[case::take_while("[1 2 3] | take while {|x| exit 7 }")]
#[case::take_until("[1 2 3] | take until {|x| exit 7 }")]
#[case::skip_while("[1 2 3] | skip while {|x| exit 7 }")]
#[case::skip_until("[1 2 3] | skip until {|x| exit 7 }")]
#[case::chunk_by("[1 2 3] | chunk-by {|x| exit 7 }")]
#[case::split_list("[1 2 3] | split list {|x| exit 7 }")]
// `do --ignore-errors` is for errors, and `exit` isn't one.
#[case::do_ignore_errors("do --ignore-errors { exit 7 }; 'continued'")]
#[case::do_ignore_errors_stream("do --ignore-errors { exiting 1 } | last")]
#[case::do_ignore_errors_value(
    "do --ignore-errors { [1] | each {|x| exit 7 } | reduce {|i a| $i } }; 'continued'"
)]
#[case::do_ignore_errors_to_text("do --ignore-errors { exiting 1 | to text }; 'continued'")]
#[case::do_ignore_errors_to_text_piped(
    "do --ignore-errors { exiting 1 | to text } | ignore; 'continued'"
)]
// An item with no partner, or that isn't text, is still read.
#[case::zip_no_other("exiting 1 | zip []")]
#[case::zip_other_ended("exiting 2 | zip [a]")]
#[case::lines("exiting 1 | lines")]
#[case::to_text_lines("[1 2] | each {|x| exit 7 } | to text | lines")]
// A byte stream carries it, and each way of reading that has to unpack it.
#[case::to_text_is_empty("exiting 1 | to text | is-empty")]
#[case::to_text_from_csv("exiting 2 | to text | from csv")]
#[case::to_text_from_tsv("exiting 2 | to text | from tsv")]
#[case::to_text_from_json_objects("exiting 2 | to text | from json --objects")]
#[case::to_text_binary_length("exiting 2 | to text | into binary | length")]
#[case::to_text_binary_last("exiting 2 | to text | into binary | last 1")]
#[case::to_text_binary_msgpack("exiting 1 | to text | into binary | from msgpack")]
// An ordinary error earlier in the group doesn't hide it.
#[case::chunks_after_error(
    "[1 2] | each {|x| if $x == 1 { error make {msg: boom} } else { exit 7 } } | chunks 2"
)]
#[case::window_after_error(
    "[1 2] | each {|x| if $x == 1 { error make {msg: boom} } else { exit 7 } } | window 2"
)]
#[case::window_overlap_tail_after_error(
    "[1 2] | each {|x| if $x == 1 { error make {msg: boom} } else { exit 7 } } | window 3"
)]
#[case::window_gap_tail_after_error(
    "[1 2] | each {|x| if $x == 1 { error make {msg: boom} } else { exit 7 } } | window 3 --stride 4"
)]
fn exit_is_not_lost(#[case] pipeline: &str) -> Result {
    let err = test()
        .run(format!("{STREAMS}\n{pipeline}"))
        .expect_shell_error()?;
    assert_matches!(err, ShellError::Exit { code: 7, .. });
    Ok(())
}

#[rstest]
#[case::to_text_statement("do --ignore-errors { failing 1 | to text }; 'continued'", "continued")]
#[case::to_text_piped(
    "do --ignore-errors { failing 1 | to text } | ignore; 'continued'",
    "continued"
)]
#[case::to_text_ends_at_the_error("do --ignore-errors { failing 2 | to text }", "1\n")]
fn do_ignore_errors_covers_a_byte_stream(#[case] pipeline: &str, #[case] expected: &str) -> Result {
    test()
        .run(format!("{STREAMS}\n{pipeline}"))
        .expect_value_eq(expected)
}

#[rstest]
#[case::to_text("failing 2 | to text | relay; 'continued'")]
#[case::list_stream("failing 2 | relay; 'continued'")]
#[case::collected("let out = (failing 2 | to text | relay); 'continued'")]
#[case::lines("failing 2 | to text | relay | lines")]
#[case::complete("failing 2 | to text | relay | complete")]
#[case::ignore("failing 2 | to text | relay | ignore; 'continued'")]
#[case::two_externals("failing 2 | to text | relay | relay; 'continued'")]
#[nu_test_support::test]
#[deps(TESTBIN_RELAY)]
fn external_reports_an_error_in_its_input(#[case] pipeline: &str) -> Result {
    let err = test()
        .run(format!("{STREAMS}\n{pipeline}"))
        .expect_shell_error()?;
    assert_contains("boom", format!("{err:?}"));
    Ok(())
}

#[rstest]
#[case::to_text("exiting 1 | to text | relay; 'continued'")]
#[case::list_stream("exiting 1 | relay; 'continued'")]
#[case::collected("let out = (exiting 1 | to text | relay); 'continued'")]
#[case::lines("exiting 1 | to text | relay | lines")]
#[case::complete("exiting 1 | to text | relay | complete")]
#[case::two_externals("exiting 1 | to text | relay | relay; 'continued'")]
#[case::do_ignore_errors("do --ignore-errors { exiting 1 | to text | relay }; 'continued'")]
#[case::do_ignore_errors_piped("do --ignore-errors { exiting 1 | to text | relay } | lines")]
#[nu_test_support::test]
#[deps(TESTBIN_RELAY)]
fn external_reports_an_exit_in_its_input(#[case] pipeline: &str) -> Result {
    let err = test()
        .run(format!("{STREAMS}\n{pipeline}"))
        .expect_shell_error()?;
    assert_matches!(err, ShellError::Exit { code: 7, .. });
    Ok(())
}

#[test]
#[deps(TESTBIN_RELAY)]
fn catch_sees_an_error_in_the_input_of_an_external() -> Result {
    let pipeline = "try { failing 1 | to text | relay; 'inner' } catch { 'caught' }";
    test()
        .run(format!("{STREAMS}\n{pipeline}"))
        .expect_value_eq("caught")
}

#[rstest]
#[case::statement("do --ignore-errors { failing 1 | to text | relay }; 'continued'")]
#[case::piped("do --ignore-errors { failing 1 | to text | relay } | lines; 'continued'")]
#[nu_test_support::test]
#[deps(TESTBIN_RELAY)]
fn do_ignore_errors_covers_an_error_in_the_input_of_an_external(#[case] pipeline: &str) -> Result {
    test()
        .run(format!("{STREAMS}\n{pipeline}"))
        .expect_value_eq("continued")
}

#[rstest]
// `nonu` doesn't read its input, so writing to it may fail, which is up to `nonu`.
#[case::stops_reading("1..100000 | to text | nonu; 'continued'")]
// A lone number has no bytes to write, which ends the input without being a failure.
#[case::no_bytes_to_write("1 | relay; 'continued'")]
#[nu_test_support::test]
#[deps(TESTBIN_NONU, TESTBIN_RELAY)]
fn external_input_ending_early_is_no_failure(#[case] pipeline: &str) -> Result {
    test().run(pipeline).expect_value_eq("continued")
}

/// Only pipefail reports an external that fails before the last one, whatever connects the two.
#[rstest]
#[case::fed("'x' | fail | relay; print continued")]
#[case::teed("fail | tee { ignore } | relay; print continued")]
#[nu_test_support::test]
#[deps(NU, TESTBIN_FAIL, TESTBIN_RELAY)]
fn earlier_external_failing_is_no_failure_without_pipefail(#[case] code: &str) -> Result {
    let result: CompleteResult = test().run_with_data(
        "let code; nu -n --experimental-options '[pipefail=false]' -c $code | complete",
        code,
    )?;
    assert_contains("continued", &result.stdout);
    assert_eq!(result.exit_code, 0);
    Ok(())
}

#[rstest]
#[case::last("[1 2 3] | each {|x| if $x == 1 { exit 7 }; $x } | last; print continued")]
#[case::first("[1 2 3] | each {|x| if $x == 1 { exit 7 }; $x } | first; print continued")]
#[case::do_ignore_errors("do --ignore-errors { exit 7 }; print continued")]
#[nu_test_support::test]
#[deps(NU)]
fn exit_ends_the_process(#[case] code: &str) -> Result {
    let result: CompleteResult =
        test().run_with_data("let code; nu -n -c $code | complete", code)?;
    assert_contains_not("continued", &result.stdout);
    assert_eq!(result.stderr, "");
    assert_eq!(result.exit_code, 7);
    Ok(())
}

#[test]
#[deps(NU, TESTBIN_RELAY)]
fn exit_in_the_input_of_an_external_ends_the_process() -> Result {
    let code = "[1 2] | each {|x| exit 7 } | relay; print continued";
    let result: CompleteResult =
        test().run_with_data("let code; nu -n -c $code | complete", code)?;
    assert_contains_not("continued", &result.stdout);
    assert_eq!(result.exit_code, 7);
    Ok(())
}
