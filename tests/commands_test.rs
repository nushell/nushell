mod helpers;

use h::in_directory as cwd;
use helpers as h;

use nu::AbsolutePath;

#[test]
fn lines() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml --raw | lines | skip-while $it != \"[dependencies]\" | skip 1 | first 1 | split-column \"=\" | get Column1 | trim | echo $it");

    assert_eq!(output, "rustyline");
}

#[test]
fn open_can_parse_csv() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open caco3_plastics.csv | first 1 | get origin | echo $it"
    );

    assert_eq!(output, "SPAIN");
}

#[test]
fn open_can_parse_toml() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | get package.edition | echo $it"
    );

    assert_eq!(output, "2018");
}

#[test]
fn open_can_parse_json() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open sgml_description.json | get glossary.GlossDiv.GlossList.GlossEntry.GlossSee | echo $it");

    assert_eq!(output, "markup")
}

#[test]
fn open_can_parse_xml() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open jonathan.xml | get rss.channel.item.link | echo $it"
    );

    assert_eq!(
        output,
        "http://www.jonathanturner.org/2015/10/off-to-new-adventures.html"
    )
}

#[test]
fn open_can_parse_ini() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open sample.ini | get SectionOne.integer | echo $it"
    );

    assert_eq!(output, "1234")
}

#[test]
fn open_error_if_file_not_found() {
    nu_error!(
        output,
        cwd("tests/fixtures/formats"),
        "open i_dont_exist.txt | echo $it"
    );

    assert!(output.contains("File could not be opened"));
}

#[test]
fn save_can_write_out_csv() -> Result<(), std::io::Error> {
    let (playground, tmp, _dir) = h::setup_playground_for("save_test")?;

    let expected_file = tmp.as_ref().join("cargo_sample.csv");

    let root = &AbsolutePath::new(std::env::current_dir()?);

    let path = root / "tests/fixtures/formats/cargo_sample.toml";

    let command = format!(
        "open {} | inc package.version --minor | get package | save {}",
        path.as_ref().display(),
        "cargo_sample.csv"
    );

    for item in std::fs::read_dir(tmp.as_ref()) {
        println!("item :: {:?}", item);
    }

    nu!(_output, tmp.as_ref().display(), command);

    let actual = h::file_contents(&expected_file);
    assert!(actual.contains("[list list],A shell for the GitHub era,2018,ISC,nu,0.2.0"));

    drop(playground);
    drop(tmp);

    Ok(())
}

#[test]
fn rm_can_remove_a_file() -> Result<(), std::io::Error> {
    let _ = pretty_env_logger::try_init();

    let (_playground, tmp, _) = h::setup_playground_for("remove_file")?;

    let file = &AbsolutePath::new(&tmp) / "rm_test.txt";

    h::create_file_at(&file)?;

    nu!(_output, tmp.path().display(), "rm rm_test.txt");

    assert!(!file.as_ref().exists());

    Ok(())
}

#[test]
fn rm_can_remove_directory_contents_with_recursive_flag() -> Result<(), std::io::Error> {
    let _ = pretty_env_logger::try_init();

    let (playground, tmp, dir) = h::setup_playground_for("rm_test")?;

    for f in ["yehuda.txt", "jonathan.txt", "andres.txt"].iter() {
        h::create_file_at(&tmp.path().join(f))?;
    }

    nu!(
        _output,
        playground.path().display(),
        format!("rm {} --recursive", dir)
    );

    assert!(!tmp.path().exists());

    Ok(())
}

#[test]
fn rm_error_if_attempting_to_delete_a_directory_without_recursive_flag(
) -> Result<(), std::io::Error> {
    let (playground, tmp, dir) = h::setup_playground_for("rm_test_2")?;

    nu_error!(output, playground.path().display(), format!("rm {}", dir));

    assert!(tmp.path().exists());
    assert!(output.contains("is a directory"));
    h::delete_directory_at(tmp.path());

    Ok(())
}

#[test]
fn rm_error_if_attempting_to_delete_single_dot_as_argument() -> Result<(), std::io::Error> {
    let (_playground, tmp, _) = h::setup_playground_for("rm_test_2")?;

    nu_error!(output, tmp.path().display(), "rm .");

    assert!(output.contains("may not be removed"));

    Ok(())
}

#[test]
fn rm_error_if_attempting_to_delete_two_dot_as_argument() -> Result<(), std::io::Error> {
    let (_playground, tmp, _) = h::setup_playground_for("rm_test_2")?;

    nu_error!(output, tmp.path().display(), "rm ..");

    assert!(output.contains("may not be removed"));

    Ok(())
}
