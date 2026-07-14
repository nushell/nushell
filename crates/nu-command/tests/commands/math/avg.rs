use nu_test_support::prelude::*;

#[test]
fn can_average_numbers() -> Result {
    let code = "
        open sgml_description.json
        | get glossary.GlossDiv.GlossList.GlossEntry.Sections
        | math avg
     ";
    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(101.5)
}

#[test]
fn can_average_bytes() -> Result {
    test()
        .run("[100kb, 10b, 100mib] | math avg | to json -r")
        .expect_value_eq("34985870")
}

#[test]
fn can_average_range() -> Result {
    test().run("0..5 | math avg").expect_value_eq(2.5)
}

#[test]
fn cannot_average_infinite_range() -> Result {
    let outcome = test().run("0.. | math avg").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}

#[test]
fn const_avg() -> Result {
    test()
        .run("const AVG = [1 3 5] | math avg; $AVG")
        .expect_value_eq(3.0)
}

#[test]
fn overflow_error_is_consistent_between_list_and_table() -> Result {
    // Large durations that sum beyond i64::MAX nanoseconds
    let durations = "[618019200000000000ns, 650422800000000000ns, 652579200000000000ns, 657849600000000000ns, 660873600000000000ns, 662342400000000000ns, 664416000000000000ns, 667782000000000000ns, 669855600000000000ns, 673311600000000000ns, 675903600000000000ns, 677462400000000000ns, 681609600000000000ns, 683766000000000000ns]";

    let list_err = test()
        .run(format!("{durations} | math avg"))
        .expect_shell_error()?;

    let table_err = test()
        .run(format!("{durations} | wrap d | math avg"))
        .expect_shell_error()?;

    assert!(
        std::mem::discriminant(&list_err) == std::mem::discriminant(&table_err),
        "list and table paths produced different error variants:\n  list:  {list_err:?}\n  table: {table_err:?}"
    );
    Ok(())
}
