use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;

#[test]
fn semicolon_allows_lhs_to_complete(playground: Playground) -> Result {
    test()
        .cwd(playground.path())
        .run("touch i_will_be_created_semi.txt; 'done'")
        .expect_value_eq("done")?;

    let path = playground.path().join("i_will_be_created_semi.txt");
    assert!(path.exists());
    Ok(())
}

#[test]
fn semicolon_lhs_error_stops_processing() -> Result {
    test().run("where 1 1; 'done'").expect_parse_error()?;
    Ok(())
}
