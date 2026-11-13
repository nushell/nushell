use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case::filesize("1kb", 1000)]
#[case::binary_filesize("1kib", 1024)]
#[case::int("1024", 1024)]
#[case::binary("0x[01010101]", 16843009)]
#[case::empty_binary("0x[]", 0)]
fn into_int_converts_value(#[case] input: &str, #[case] expected: i64) -> Result {
    test()
        .run(format!("{input} | into int"))
        .expect_value_eq(expected)
}

#[rstest]
#[case::binary("0x[f0]", -16)]
#[case::empty_binary("0x[]", 0)]
fn into_int_converts_signed_value(#[case] input: &str, #[case] expected: i64) -> Result {
    test()
        .run(format!("{input} | into int --signed"))
        .expect_value_eq(expected)
}

#[test]
fn into_int_binary_back_and_forth() -> Result {
    test()
        .run("0x[f0] | into int | into binary | to nuon")
        .expect_value_eq("0x[F000000000000000]")
}

#[test]
fn into_int_binary_signed_back_and_forth() -> Result {
    test()
        .run("0x[f0] | into int --signed | into binary | to nuon")
        .expect_value_eq("0x[F0FFFFFFFFFFFFFF]")
}

#[rstest]
#[case("1983-04-13T12:09:14.123456789-05:00", 419101754123456789)] // full precision
#[case("1983-04-13T12:09:14.456789-05:00", 419101754456789000)] // microsec
#[case("1983-04-13T12:09:14-05:00", 419101754000000000)] // sec
#[case("2052-04-13T12:09:14.123456789-05:00", 2596640954123456789)] // future date > 2038 epoch
#[case("1902-04-13T12:09:14.123456789-05:00", -2137042245876543211)] // past date < 1970
fn into_int_datetime(#[case] time_in: &str, #[case] int_out: i64) -> Result {
    test()
        .run_with_data("$in | into datetime --format '%+' | into int", time_in)
        .expect_value_eq(int_out)
}
