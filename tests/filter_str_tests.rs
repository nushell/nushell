mod helpers;

use helpers::in_directory as cwd;

#[test]
fn acts_without_passing_field() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open caco3_plastics.csv | first 1 | get origin | str --downcase | echo $it"
    );

    assert_eq!(output, "spain");
}

#[test]
fn str_can_only_apply_one() {
    nu_error!(
        output,
        cwd("tests/fixtures/formats"),
        "open caco3_plastics.csv | first 1 | str origin --downcase --upcase"
    );

    assert!(output.contains("Usage: str field [--downcase|--upcase|--to-int]"));
}

#[test]
fn downcases() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open caco3_plastics.csv | first 1 | str origin --downcase | get origin | echo $it"
    );

    assert_eq!(output, "spain");
}

#[test]
fn upcases() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open appveyor.yml | str environment.global.PROJECT_NAME --upcase | get environment.global.PROJECT_NAME | echo $it"
    );

    assert_eq!(output, "NUSHELL");
}

#[test]
fn converts_to_int() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open caco3_plastics.csv | get 0 | str tariff_item --to-int | where tariff_item == 2509000000 | get tariff_item | echo $it"
    );

    assert_eq!(output, "2509000000");
}