mod helpers;

use helpers::in_directory as cwd;

use clipboard::{ClipboardProvider, ClipboardContext};

#[test]
fn clip() {

	let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();

    nu!(
        _output,
        cwd("tests/fixtures/formats"),
        "open caco3_plastics.csv --raw | lines | clip"
    );


    assert!(ctx.get_contents().is_ok());
}