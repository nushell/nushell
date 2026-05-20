use std::collections::HashMap;

use nu_test_support::prelude::*;

// Tests against table/structured data
#[test]
fn cal_full_year() -> Result {
    let outcome: HashMap<String, Value> = test().run("cal -t -y --full-year 2010 | first")?;
    assert_eq!(outcome.len(), 8);
    outcome["year"].assert_eq(2010);
    outcome["su"].assert_eq(());
    outcome["mo"].assert_eq(());
    outcome["tu"].assert_eq(());
    outcome["we"].assert_eq(());
    outcome["th"].assert_eq(());
    outcome["fr"].assert_eq(1);
    outcome["sa"].assert_eq(2);
    Ok(())
}

#[test]
fn cal_february_2020_leap_year() -> Result {
    let code = r#"
        cal --as-table -y --full-year 2020 --month-names
        | where month_name == "february"
    "#;

    let outcome: Vec<HashMap<String, Value>> = test().run(code)?;
    assert_eq!(outcome.len(), 5);

    // week 1
    outcome[0]["year"].assert_eq(2020);
    outcome[0]["month_name"].assert_eq("february");
    outcome[0]["su"].assert_eq(());
    outcome[0]["mo"].assert_eq(());
    outcome[0]["tu"].assert_eq(());
    outcome[0]["we"].assert_eq(());
    outcome[0]["th"].assert_eq(());
    outcome[0]["fr"].assert_eq(());
    outcome[0]["sa"].assert_eq(1);

    // week 2
    outcome[1]["su"].assert_eq(2);
    outcome[1]["mo"].assert_eq(3);
    outcome[1]["tu"].assert_eq(4);
    outcome[1]["we"].assert_eq(5);
    outcome[1]["th"].assert_eq(6);
    outcome[1]["fr"].assert_eq(7);
    outcome[1]["sa"].assert_eq(8);

    // week 3
    outcome[2]["su"].assert_eq(9);
    outcome[2]["mo"].assert_eq(10);
    outcome[2]["tu"].assert_eq(11);
    outcome[2]["we"].assert_eq(12);
    outcome[2]["th"].assert_eq(13);
    outcome[2]["fr"].assert_eq(14);
    outcome[2]["sa"].assert_eq(15);

    // week 4
    outcome[3]["su"].assert_eq(16);
    outcome[3]["mo"].assert_eq(17);
    outcome[3]["tu"].assert_eq(18);
    outcome[3]["we"].assert_eq(19);
    outcome[3]["th"].assert_eq(20);
    outcome[3]["fr"].assert_eq(21);
    outcome[3]["sa"].assert_eq(22);

    // week 5
    outcome[4]["su"].assert_eq(23);
    outcome[4]["mo"].assert_eq(24);
    outcome[4]["tu"].assert_eq(25);
    outcome[4]["we"].assert_eq(26);
    outcome[4]["th"].assert_eq(27);
    outcome[4]["fr"].assert_eq(28);
    outcome[4]["sa"].assert_eq(29);

    Ok(())
}

#[test]
fn cal_month_int_matches_name() -> Result {
    let code = r#"
        cal --as-table --full-year 2015 --month --month-names
        | where month_name == "august" and month == 8
    "#;

    let outcome: Vec<HashMap<String, Value>> = test().run(code)?;
    assert_eq!(outcome.len(), 6);

    // week 1
    outcome[0]["month"].assert_eq(8);
    outcome[0]["month_name"].assert_eq("august");
    outcome[0]["su"].assert_eq(());
    outcome[0]["mo"].assert_eq(());
    outcome[0]["tu"].assert_eq(());
    outcome[0]["we"].assert_eq(());
    outcome[0]["th"].assert_eq(());
    outcome[0]["fr"].assert_eq(());
    outcome[0]["sa"].assert_eq(1);

    // week 2
    outcome[1]["su"].assert_eq(2);
    outcome[1]["mo"].assert_eq(3);
    outcome[1]["tu"].assert_eq(4);
    outcome[1]["we"].assert_eq(5);
    outcome[1]["th"].assert_eq(6);
    outcome[1]["fr"].assert_eq(7);
    outcome[1]["sa"].assert_eq(8);

    // week 3
    outcome[2]["su"].assert_eq(9);
    outcome[2]["mo"].assert_eq(10);
    outcome[2]["tu"].assert_eq(11);
    outcome[2]["we"].assert_eq(12);
    outcome[2]["th"].assert_eq(13);
    outcome[2]["fr"].assert_eq(14);
    outcome[2]["sa"].assert_eq(15);

    // week 4
    outcome[3]["su"].assert_eq(16);
    outcome[3]["mo"].assert_eq(17);
    outcome[3]["tu"].assert_eq(18);
    outcome[3]["we"].assert_eq(19);
    outcome[3]["th"].assert_eq(20);
    outcome[3]["fr"].assert_eq(21);
    outcome[3]["sa"].assert_eq(22);

    // week 5
    outcome[4]["su"].assert_eq(23);
    outcome[4]["mo"].assert_eq(24);
    outcome[4]["tu"].assert_eq(25);
    outcome[4]["we"].assert_eq(26);
    outcome[4]["th"].assert_eq(27);
    outcome[4]["fr"].assert_eq(28);
    outcome[4]["sa"].assert_eq(29);

    // week 6
    outcome[5]["su"].assert_eq(30);
    outcome[5]["mo"].assert_eq(31);
    outcome[5]["tu"].assert_eq(());
    outcome[5]["we"].assert_eq(());
    outcome[5]["th"].assert_eq(());
    outcome[5]["fr"].assert_eq(());
    outcome[5]["sa"].assert_eq(());

    Ok(())
}

#[test]
fn cal_fr_the_thirteenths_in_2015() -> Result {
    test()
        .run("cal --as-table --full-year 2015 | default 0 fr | where fr == 13 | length")
        .expect_value_eq(3)
}

#[test]
fn cal_rows_in_2020() -> Result {
    test()
        .run("cal --as-table --full-year 2020 | length")
        .expect_value_eq(62)
}

#[test]
fn cal_week_day_start_mo() -> Result {
    let code = "
        cal --as-table --full-year 2020 -m --month-names --week-start mo
        | where month_name == january
    ";

    let outcome: Vec<HashMap<String, Value>> = test().run(code)?;
    assert_eq!(outcome.len(), 5);

    // week 1
    outcome[0]["month"].assert_eq(1);
    outcome[0]["month_name"].assert_eq("january");
    outcome[0]["mo"].assert_eq(());
    outcome[0]["tu"].assert_eq(());
    outcome[0]["we"].assert_eq(1);
    outcome[0]["th"].assert_eq(2);
    outcome[0]["fr"].assert_eq(3);
    outcome[0]["sa"].assert_eq(4);
    outcome[0]["su"].assert_eq(5);

    // week 2
    outcome[1]["mo"].assert_eq(6);
    outcome[1]["tu"].assert_eq(7);
    outcome[1]["we"].assert_eq(8);
    outcome[1]["th"].assert_eq(9);
    outcome[1]["fr"].assert_eq(10);
    outcome[1]["sa"].assert_eq(11);
    outcome[1]["su"].assert_eq(12);

    // week 3
    outcome[2]["mo"].assert_eq(13);
    outcome[2]["tu"].assert_eq(14);
    outcome[2]["we"].assert_eq(15);
    outcome[2]["th"].assert_eq(16);
    outcome[2]["fr"].assert_eq(17);
    outcome[2]["sa"].assert_eq(18);
    outcome[2]["su"].assert_eq(19);

    // week 4
    outcome[3]["mo"].assert_eq(20);
    outcome[3]["tu"].assert_eq(21);
    outcome[3]["we"].assert_eq(22);
    outcome[3]["th"].assert_eq(23);
    outcome[3]["fr"].assert_eq(24);
    outcome[3]["sa"].assert_eq(25);
    outcome[3]["su"].assert_eq(26);

    // week 5
    outcome[4]["mo"].assert_eq(27);
    outcome[4]["tu"].assert_eq(28);
    outcome[4]["we"].assert_eq(29);
    outcome[4]["th"].assert_eq(30);
    outcome[4]["fr"].assert_eq(31);
    outcome[4]["sa"].assert_eq(());
    outcome[4]["su"].assert_eq(());

    Ok(())
}

#[test]
fn cal_sees_pipeline_year() -> Result {
    test()
        .run("cal --as-table --full-year 1020 | get mo | first 4")
        .expect_value_eq(((), 3, 10, 17))
}

// Tests against default string output
#[test]
fn cal_is_string() -> Result {
    test()
        .run("cal | describe")
        .expect_value_eq("string (stream)")
}

#[test]
fn cal_year_num_lines() -> Result {
    test()
        .run("cal --full-year 2024 | lines | length")
        .expect_value_eq(68)
}

#[test]
fn cal_week_start_string() -> Result {
    test()
        .run("cal --week-start fr | lines | get 1 | split row '│' | get 2 | ansi strip | str trim")
        .expect_value_eq("sa")
}
