mod helpers;

use helpers::Playground;

#[test]
fn cd_directory_not_found() {
    let actual = nu_error!(
    	cwd: "tests/fixtures",
    	"cd dir_that_does_not_exist"
    );

    assert!(actual.contains("dir_that_does_not_exist"));
    assert!(actual.contains("directory not found"));
}

#[test]
fn cd_back() {
    Playground::setup("cd_test_back", |dirs, sandbox| {
        sandbox
            .mkdir("andres")
            .mkdir("odin");

        let odin = dirs.test().join("odin");
        let andres = dirs.test().join("andres");

        nu!(
            cwd: dirs.test(),
            r#"
                cd odin
                mkdir a
                cd ../andres
                mkdir b
                cd -
                mkdir c
                mkdir -
                cd -
                mkdir d
            "#
        );

        assert!(odin.join("a").exists());
        assert!(andres.join("b").exists());
        assert!(odin.join("c").exists());
        assert!(odin.join("-").join("d").exists());
    })
}
