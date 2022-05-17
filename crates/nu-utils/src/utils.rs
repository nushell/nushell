use std::io::{Result, Write};

pub fn enable_vt_processing() -> Result<()> {
    #[cfg(windows)]
    {
        use crossterm_winapi::{ConsoleMode, Handle};

        pub const ENABLE_PROCESSED_OUTPUT: u32 = 0x0001;
        pub const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
        // let mask = ENABLE_VIRTUAL_TERMINAL_PROCESSING;

        let console_mode = ConsoleMode::from(Handle::current_out_handle()?);
        let old_mode = console_mode.mode()?;

        // researching odd ansi behavior in windows terminal repo revealed that
        // enable_processed_output and enable_virtual_terminal_processing should be used
        // also, instead of checking old_mode & mask, just set the mode already

        // if old_mode & mask == 0 {
        console_mode
            .set_mode(old_mode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING)?;
        // }
    }
    Ok(())
}

pub fn stdout_write_all_and_flush(output: String) -> Result<()> {
    let stdout = std::io::stdout();
    let ret = match stdout.lock().write_all(output.as_bytes()) {
        Ok(_) => Ok(stdout.lock().flush()?),
        Err(err) => Err(err),
    };

    ret
}

pub fn stdout_write_all_as_binary_and_flush(output: &[u8]) -> Result<()> {
    let stdout = std::io::stdout();
    let ret = match stdout.lock().write_all(output) {
        Ok(_) => Ok(stdout.lock().flush()?),
        Err(err) => Err(err),
    };

    ret
}
