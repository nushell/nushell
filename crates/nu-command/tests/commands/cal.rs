use nu_test_support::{nu, pipeline};

#[test]
fn cal_full_year() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        cal -y --full-year 2010 | first | to json -r
        "#
    ));

    let first_week_2010_json = r#"{"year": 2010,"sunday": null,"monday": null,"tuesday": null,"wednesday": null,"thursday": null,"friday": 1,"saturday": 2}"#;

    assert_eq!(actual.out, first_week_2010_json);
}

#[test]
fn cal_february_2020_leap_year() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        cal -ym --full-year 2020 --month-names | where month == "february" | to json -r
        "#
    ));

    let cal_february_json = r#"[{"year": 2020,"month": "february","sunday": null,"monday": null,"tuesday": null,"wednesday": null,"thursday": null,"friday": null,"saturday": 1},{"year": 2020,"month": "february","sunday": 2,"monday": 3,"tuesday": 4,"wednesday": 5,"thursday": 6,"friday": 7,"saturday": 8},{"year": 2020,"month": "february","sunday": 9,"monday": 10,"tuesday": 11,"wednesday": 12,"thursday": 13,"friday": 14,"saturday": 15},{"year": 2020,"month": "february","sunday": 16,"monday": 17,"tuesday": 18,"wednesday": 19,"thursday": 20,"friday": 21,"saturday": 22},{"year": 2020,"month": "february","sunday": 23,"monday": 24,"tuesday": 25,"wednesday": 26,"thursday": 27,"friday": 28,"saturday": 29}]"#;

    assert_eq!(actual.out, cal_february_json);
}

#[test]
fn cal_friday_the_thirteenths_in_2015() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        cal --full-year 2015 | default friday 0 | where friday == 13 | length
        "#
    ));

    assert!(actual.out.contains('3'));
}

#[test]
fn cal_rows_in_2020() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        cal --full-year 2020 | length
        "#
    ));

    assert!(actual.out.contains("62"));
}

#[test]
fn cal_week_day_start_monday() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        cal --full-year 2020 -m --month-names --week-start monday | where month == january | to json -r
        "#
    ));

    let cal_january_json = r#"[{"month": "january","monday": null,"tuesday": null,"wednesday": 1,"thursday": 2,"friday": 3,"saturday": 4,"sunday": 5},{"month": "january","monday": 6,"tuesday": 7,"wednesday": 8,"thursday": 9,"friday": 10,"saturday": 11,"sunday": 12},{"month": "january","monday": 13,"tuesday": 14,"wednesday": 15,"thursday": 16,"friday": 17,"saturday": 18,"sunday": 19},{"month": "january","monday": 20,"tuesday": 21,"wednesday": 22,"thursday": 23,"friday": 24,"saturday": 25,"sunday": 26},{"month": "january","monday": 27,"tuesday": 28,"wednesday": 29,"thursday": 30,"friday": 31,"saturday": null,"sunday": null}]"#;

    assert_eq!(actual.out, cal_january_json);
}

#[test]
fn cal_sees_pipeline_year() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        cal --full-year 1020 | get monday | first 4 | to json -r
        "#
    ));

    assert_eq!(actual.out, "[null,3,10,17]");
}
