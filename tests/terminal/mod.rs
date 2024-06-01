use alacritty_terminal::tty::EventedReadWrite;
use nu_test_support::terminal::{
    default_terminal, extract_cursor, extract_text, pty_with_nushell, pty_write_handler,
    read_to_end,
};
use std::{io::Write, time::Duration};

#[test]
fn auto_cd_works() {
    // Setup a directory with a sub-directory in it.
    let cwd = tempfile::tempdir().unwrap();
    std::fs::create_dir(cwd.path().join("foo")).unwrap();

    // Create the PTY and the terminal.
    let mut pty = pty_with_nushell(
        vec!["--no-config-file".to_string()],
        Some(cwd.path().to_path_buf()),
    );
    let (mut term, mut events) = default_terminal();

    // Wait for Nushell to initalize.
    std::thread::sleep(Duration::from_millis(500));

    #[cfg(windows)]
    pty.writer().write_all(b".\\foo\r").unwrap();
    #[cfg(unix)]
    pty.writer().write_all(b"./foo\r").unwrap();

    pty.writer().write_all(b"pwd\r").unwrap();

    // Read the response from Nushell.
    read_to_end(&mut term, &mut pty, &mut events, pty_write_handler);

    // Examine the terminal state.
    let (row, _col) = extract_cursor(&term);
    let text = extract_text(&term);
    assert!(text[row - 1].contains("foo"));
}

#[test]
fn command_hints_are_pwd_aware() {
    // PWD-aware command hints require setting history file format to "sqlite".
    let nu_config = tempfile::NamedTempFile::new().unwrap();
    let nu_config_string = nu_config.path().to_string_lossy().to_string();
    std::fs::write(
        &nu_config,
        "$env.config = { history: { file_format: 'sqlite' } }",
    )
    .unwrap();

    // Setup a directory with two sub-directories in it.
    let cwd = tempfile::tempdir().unwrap();
    std::fs::create_dir(cwd.path().join("foo")).unwrap();
    std::fs::create_dir(cwd.path().join("bar")).unwrap();

    // Create the PTY and the terminal.
    let mut pty = pty_with_nushell(
        vec!["--config".to_string(), nu_config_string],
        Some(cwd.path().to_path_buf()),
    );
    let (mut term, mut events) = default_terminal();

    // Wait for Nushell to initalize.
    std::thread::sleep(Duration::from_millis(500));

    pty.writer().write_all(b"cd foo\r").unwrap();
    pty.writer().write_all(b"print 'FOO'\r").unwrap();
    pty.writer().write_all(b"cd ../bar\r").unwrap();
    pty.writer().write_all(b"print 'BAR'\r").unwrap();
    pty.writer().write_all(b"cd ../foo\r").unwrap();
    // Type "print", then press the right arrow, then press Enter.
    pty.writer().write_all(b"print\x1b[C\r").unwrap();

    // Read the response from Nushell.
    read_to_end(&mut term, &mut pty, &mut events, pty_write_handler);

    // Examine the terminal state.
    let (row, _col) = extract_cursor(&term);
    let text = extract_text(&term);
    assert!(text[row - 2].contains("print 'FOO'"));
}
