use rstest::rstest;
use rstest_reuse::{apply, template};

use nu_protocol::{Filesize, FilesizeUnit};
use nu_test_support::prelude::*;

#[template]
#[rstest]
#[case(Filesize::ZERO, FilesizeUnit::B.as_str(), "0 B")]
#[case(Filesize::ZERO, FilesizeUnit::KB.as_str(), "0 kB")]
#[case(FilesizeUnit::KiB.as_filesize(), FilesizeUnit::B.as_str(), "1024 B")]
#[case(Filesize::from_unit(2442, FilesizeUnit::KB).unwrap(), FilesizeUnit::B.as_str(), "2442000 B")]
fn filesize_cases(
    #[case] size: Filesize,
    #[case] unit_str: &str,
    #[case] expected: &str,
) -> Result {
}

#[apply(filesize_cases)]
fn format_filesize_works(
    #[case] size: Filesize,
    #[case] unit_str: &str,
    #[case] expected: &str,
) -> Result {
    let mut tester = test();
    let () = tester.run_with_data("let unit = $in", unit_str)?;
    tester
        .run_with_data("format filesize $unit", size)
        .expect_value_eq(expected)
}

#[apply(filesize_cases)]
fn format_filesize_works_with_records(
    #[case] size: Filesize,
    #[case] unit_str: &str,
    #[case] expected: &str,
) -> Result {
    let mut tester = test();
    let () = tester.run_with_data("let unit = $in", unit_str)?;
    tester
        .run_with_data("format filesize $unit foo", test_value!({ foo: size }))
        .expect_value_eq(test_value!({foo: expected}))
}

#[test]
fn format_filesize_works_with_tables() -> Result {
    let input = test_table![
        ["name", "type", "size"];
        ["yehuda.txt", "file", Filesize::ZERO],
        ["jt.txt", "file", Filesize::ZERO],
        ["andres.txt", "file", Filesize::ZERO],
    ];

    let expected = test_table![
        ["name", "type", "size"];
        ["yehuda.txt", "file", "0 kB"],
        ["jt.txt", "file", "0 kB"],
        ["andres.txt", "file", "0 kB"],
    ];

    test()
        .run_with_data("format filesize kB size", input)
        .expect_value_eq(expected)
}

#[test]
fn format_filesize_without_fraction_keeps_old_output() -> Result {
    let code = "1MB | format filesize kB";
    test().run(code).expect_value_eq("1000 kB")
}

#[test]
fn format_filesize_respects_float_precision_for_fractional_values() -> Result {
    let code = "
        $env.config = ($env.config | upsert float_precision 5)
        1024B | format filesize kB
    ";

    test().run(code).expect_value_eq("1.02400 kB")
}

#[test]
fn format_filesize_with_invalid_unit() -> Result {
    let code = "1MB | format filesize sec";
    let err = test().run(code).expect_error()?;
    assert!(matches!(err, ShellError::InvalidUnit { .. }));
    Ok(())
}
