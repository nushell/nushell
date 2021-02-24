use crate::prelude::*;
use nu_ansi_term::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Command;

#[derive(Deserialize)]
struct AnsiArgs {
    code: Value,
    escape: Option<Tagged<String>>,
    osc: Option<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "ansi"
    }

    fn signature(&self) -> Signature {
        Signature::build("ansi")
            .optional(
                "code",
                SyntaxShape::Any,
                "the name of the code to use like 'green' or 'reset' to reset the color",
            )
            .named(
                "escape", // \x1b[
                SyntaxShape::Any,
                "escape sequence without the escape character(s)",
                Some('e'),
            )
            .named(
                "osc", // \x1b]
                SyntaxShape::Any,
                "operating system command (ocs) escape sequence without the escape character(s)",
                Some('o'),
            )
    }

    fn usage(&self) -> &str {
        r#"Output ANSI codes to change color

For escape sequences:
Escape: '\x1b[' is not required for --escape parameter
Format: #(;#)m
Example: 1;31m for bold red or 2;37;41m for dimmed white fg with red bg
There can be multiple text formatting sequence numbers
separated by a ; and ending with an m where the # is of the
following values:
    attributes
    0    reset / normal display
    1    bold or increased intensity
    2    faint or decreased intensity
    3    italic on (non-mono font)
    4    underline on
    5    slow blink on
    6    fast blink on
    7    reverse video on
    8    nondisplayed (invisible) on
    9    strike-through on

    foreground/bright colors    background/bright colors
    30/90    black              40/100    black
    31/91    red                41/101    red
    32/92    green              42/102    green
    33/93    yellow             43/103    yellow
    34/94    blue               44/104    blue
    35/95    magenta            45/105    magenta
    36/96    cyan               46/106    cyan
    37/97    white              47/107    white
    https://en.wikipedia.org/wiki/ANSI_escape_code

OSC: '\x1b]' is not required for --osc parameter
Example: echo [$(ansi -o '0') 'some title' $(char bel)] | str collect
Format: #
    0 Set window title and icon name
    1 Set icon name
    2 Set window title
    4 Set/read color palette
    9 iTerm2 Grown notifications
    10 Set foreground color (x11 color spec)
    11 Set background color (x11 color spec)
    ... others"#
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Change color to green",
                example: r#"ansi green"#,
                result: Some(vec![Value::from("\u{1b}[32m")]),
            },
            Example {
                description: "Reset the color",
                example: r#"ansi reset"#,
                result: Some(vec![Value::from("\u{1b}[0m")]),
            },
            Example {
                description:
                    "Use ansi to color text (rb = red bold, gb = green bold, pb = purple bold)",
                example: r#"echo [$(ansi rb) Hello " " $(ansi gb) Nu " " $(ansi pb) World] | str collect"#,
                result: Some(vec![Value::from(
                    "\u{1b}[1;31mHello \u{1b}[1;32mNu \u{1b}[1;35mWorld",
                )]),
            },
            Example {
                description:
                    "Use ansi to color text (rb = red bold, gb = green bold, pb = purple bold)",
                example: r#"echo [$(ansi -e '3;93;41m') Hello $(ansi reset) " " $(ansi gb) Nu " " $(ansi pb) World] | str collect"#,
                result: Some(vec![Value::from(
                    "\u{1b}[3;93;41mHello\u{1b}[0m \u{1b}[1;32mNu \u{1b}[1;35mWorld",
                )]),
            },
        ]
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let (AnsiArgs { code, escape, osc }, _) = args.process().await?;

        if let Some(e) = escape {
            let esc_vec: Vec<char> = e.item.chars().collect();
            if esc_vec[0] == '\\' {
                return Err(ShellError::labeled_error(
                    "no need for escape characters",
                    "no need for escape characters",
                    e.tag(),
                ));
            }
            let output = format!("\x1b[{}", e.item);
            return Ok(OutputStream::one(ReturnSuccess::value(
                UntaggedValue::string(output).into_value(e.tag()),
            )));
        }

        if let Some(o) = osc {
            let osc_vec: Vec<char> = o.item.chars().collect();
            if osc_vec[0] == '\\' {
                return Err(ShellError::labeled_error(
                    "no need for escape characters",
                    "no need for escape characters",
                    o.tag(),
                ));
            }

            //Operating system command aka osc  ESC ] <- note the right brace, not left brace for osc
            // OCS's need to end with a bell '\x07' char
            let output = format!("\x1b]{};", o.item);
            return Ok(OutputStream::one(ReturnSuccess::value(
                UntaggedValue::string(output).into_value(o.tag()),
            )));
        }

        let code_string = code.as_string()?;
        let ansi_code = str_to_ansi(code_string);

        if let Some(output) = ansi_code {
            Ok(OutputStream::one(ReturnSuccess::value(
                UntaggedValue::string(output).into_value(code.tag()),
            )))
        } else {
            Err(ShellError::labeled_error(
                "Unknown ansi code",
                "unknown ansi code",
                code.tag(),
            ))
        }
    }
}

pub fn str_to_ansi(s: String) -> Option<String> {
    match s.as_str() {
        "g" | "green" => Some(Color::Green.prefix().to_string()),
        "gb" | "green_bold" => Some(Color::Green.bold().prefix().to_string()),
        "gu" | "green_underline" => Some(Color::Green.underline().prefix().to_string()),
        "gi" | "green_italic" => Some(Color::Green.italic().prefix().to_string()),
        "gd" | "green_dimmed" => Some(Color::Green.dimmed().prefix().to_string()),
        "gr" | "green_reverse" => Some(Color::Green.reverse().prefix().to_string()),

        "lg" | "light_green" => Some(Color::LightGreen.prefix().to_string()),
        "lgb" | "light_green_bold" => Some(Color::LightGreen.bold().prefix().to_string()),
        "lgu" | "light_green_underline" => Some(Color::LightGreen.underline().prefix().to_string()),
        "lgi" | "light_green_italic" => Some(Color::LightGreen.italic().prefix().to_string()),
        "lgd" | "light_green_dimmed" => Some(Color::LightGreen.dimmed().prefix().to_string()),
        "lgr" | "light_green_reverse" => Some(Color::LightGreen.reverse().prefix().to_string()),

        "r" | "red" => Some(Color::Red.prefix().to_string()),
        "rb" | "red_bold" => Some(Color::Red.bold().prefix().to_string()),
        "ru" | "red_underline" => Some(Color::Red.underline().prefix().to_string()),
        "ri" | "red_italic" => Some(Color::Red.italic().prefix().to_string()),
        "rd" | "red_dimmed" => Some(Color::Red.dimmed().prefix().to_string()),
        "rr" | "red_reverse" => Some(Color::Red.reverse().prefix().to_string()),

        "lr" | "light_red" => Some(Color::LightRed.prefix().to_string()),
        "lrb" | "light_red_bold" => Some(Color::LightRed.bold().prefix().to_string()),
        "lru" | "light_red_underline" => Some(Color::LightRed.underline().prefix().to_string()),
        "lri" | "light_red_italic" => Some(Color::LightRed.italic().prefix().to_string()),
        "lrd" | "light_red_dimmed" => Some(Color::LightRed.dimmed().prefix().to_string()),
        "lrr" | "light_red_reverse" => Some(Color::LightRed.reverse().prefix().to_string()),

        "u" | "blue" => Some(Color::Blue.prefix().to_string()),
        "ub" | "blue_bold" => Some(Color::Blue.bold().prefix().to_string()),
        "uu" | "blue_underline" => Some(Color::Blue.underline().prefix().to_string()),
        "ui" | "blue_italic" => Some(Color::Blue.italic().prefix().to_string()),
        "ud" | "blue_dimmed" => Some(Color::Blue.dimmed().prefix().to_string()),
        "ur" | "blue_reverse" => Some(Color::Blue.reverse().prefix().to_string()),

        "lu" | "light_blue" => Some(Color::LightBlue.prefix().to_string()),
        "lub" | "light_blue_bold" => Some(Color::LightBlue.bold().prefix().to_string()),
        "luu" | "light_blue_underline" => Some(Color::LightBlue.underline().prefix().to_string()),
        "lui" | "light_blue_italic" => Some(Color::LightBlue.italic().prefix().to_string()),
        "lud" | "light_blue_dimmed" => Some(Color::LightBlue.dimmed().prefix().to_string()),
        "lur" | "light_blue_reverse" => Some(Color::LightBlue.reverse().prefix().to_string()),

        "b" | "black" => Some(Color::Black.prefix().to_string()),
        "bb" | "black_bold" => Some(Color::Black.bold().prefix().to_string()),
        "bu" | "black_underline" => Some(Color::Black.underline().prefix().to_string()),
        "bi" | "black_italic" => Some(Color::Black.italic().prefix().to_string()),
        "bd" | "black_dimmed" => Some(Color::Black.dimmed().prefix().to_string()),
        "br" | "black_reverse" => Some(Color::Black.reverse().prefix().to_string()),

        "ligr" | "light_gray" => Some(Color::LightGray.prefix().to_string()),
        "ligrb" | "light_gray_bold" => Some(Color::LightGray.bold().prefix().to_string()),
        "ligru" | "light_gray_underline" => Some(Color::LightGray.underline().prefix().to_string()),
        "ligri" | "light_gray_italic" => Some(Color::LightGray.italic().prefix().to_string()),
        "ligrd" | "light_gray_dimmed" => Some(Color::LightGray.dimmed().prefix().to_string()),
        "ligrr" | "light_gray_reverse" => Some(Color::LightGray.reverse().prefix().to_string()),

        "y" | "yellow" => Some(Color::Yellow.prefix().to_string()),
        "yb" | "yellow_bold" => Some(Color::Yellow.bold().prefix().to_string()),
        "yu" | "yellow_underline" => Some(Color::Yellow.underline().prefix().to_string()),
        "yi" | "yellow_italic" => Some(Color::Yellow.italic().prefix().to_string()),
        "yd" | "yellow_dimmed" => Some(Color::Yellow.dimmed().prefix().to_string()),
        "yr" | "yellow_reverse" => Some(Color::Yellow.reverse().prefix().to_string()),

        "ly" | "light_yellow" => Some(Color::LightYellow.prefix().to_string()),
        "lyb" | "light_yellow_bold" => Some(Color::LightYellow.bold().prefix().to_string()),
        "lyu" | "light_yellow_underline" => {
            Some(Color::LightYellow.underline().prefix().to_string())
        }
        "lyi" | "light_yellow_italic" => Some(Color::LightYellow.italic().prefix().to_string()),
        "lyd" | "light_yellow_dimmed" => Some(Color::LightYellow.dimmed().prefix().to_string()),
        "lyr" | "light_yellow_reverse" => Some(Color::LightYellow.reverse().prefix().to_string()),

        "p" | "purple" => Some(Color::Purple.prefix().to_string()),
        "pb" | "purple_bold" => Some(Color::Purple.bold().prefix().to_string()),
        "pu" | "purple_underline" => Some(Color::Purple.underline().prefix().to_string()),
        "pi" | "purple_italic" => Some(Color::Purple.italic().prefix().to_string()),
        "pd" | "purple_dimmed" => Some(Color::Purple.dimmed().prefix().to_string()),
        "pr" | "purple_reverse" => Some(Color::Purple.reverse().prefix().to_string()),

        "lp" | "light_purple" => Some(Color::LightPurple.prefix().to_string()),
        "lpb" | "light_purple_bold" => Some(Color::LightPurple.bold().prefix().to_string()),
        "lpu" | "light_purple_underline" => {
            Some(Color::LightPurple.underline().prefix().to_string())
        }
        "lpi" | "light_purple_italic" => Some(Color::LightPurple.italic().prefix().to_string()),
        "lpd" | "light_purple_dimmed" => Some(Color::LightPurple.dimmed().prefix().to_string()),
        "lpr" | "light_purple_reverse" => Some(Color::LightPurple.reverse().prefix().to_string()),

        "c" | "cyan" => Some(Color::Cyan.prefix().to_string()),
        "cb" | "cyan_bold" => Some(Color::Cyan.bold().prefix().to_string()),
        "cu" | "cyan_underline" => Some(Color::Cyan.underline().prefix().to_string()),
        "ci" | "cyan_italic" => Some(Color::Cyan.italic().prefix().to_string()),
        "cd" | "cyan_dimmed" => Some(Color::Cyan.dimmed().prefix().to_string()),
        "cr" | "cyan_reverse" => Some(Color::Cyan.reverse().prefix().to_string()),

        "lc" | "light_cyan" => Some(Color::LightCyan.prefix().to_string()),
        "lcb" | "light_cyan_bold" => Some(Color::LightCyan.bold().prefix().to_string()),
        "lcu" | "light_cyan_underline" => Some(Color::LightCyan.underline().prefix().to_string()),
        "lci" | "light_cyan_italic" => Some(Color::LightCyan.italic().prefix().to_string()),
        "lcd" | "light_cyan_dimmed" => Some(Color::LightCyan.dimmed().prefix().to_string()),
        "lcr" | "light_cyan_reverse" => Some(Color::LightCyan.reverse().prefix().to_string()),

        "w" | "white" => Some(Color::White.prefix().to_string()),
        "wb" | "white_bold" => Some(Color::White.bold().prefix().to_string()),
        "wu" | "white_underline" => Some(Color::White.underline().prefix().to_string()),
        "wi" | "white_italic" => Some(Color::White.italic().prefix().to_string()),
        "wd" | "white_dimmed" => Some(Color::White.dimmed().prefix().to_string()),
        "wr" | "white_reverse" => Some(Color::White.reverse().prefix().to_string()),

        "dgr" | "dark_gray" => Some(Color::DarkGray.prefix().to_string()),
        "dgrb" | "dark_gray_bold" => Some(Color::DarkGray.bold().prefix().to_string()),
        "dgru" | "dark_gray_underline" => Some(Color::DarkGray.underline().prefix().to_string()),
        "dgri" | "dark_gray_italic" => Some(Color::DarkGray.italic().prefix().to_string()),
        "dgrd" | "dark_gray_dimmed" => Some(Color::DarkGray.dimmed().prefix().to_string()),
        "dgrr" | "dark_gray_reverse" => Some(Color::DarkGray.reverse().prefix().to_string()),

        "reset" => Some("\x1b[0m".to_owned()),

        // Reference for ansi codes https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797
        // Another good reference http://ascii-table.com/ansi-escape-sequences.php

        // For setting title like `echo [$(char title) $(pwd) $(char bel)] | str collect`
        "title" => Some("\x1b]2;".to_string()), // ESC]2; xterm sets window title using OSC syntax escapes
        "bel" => Some('\x07'.to_string()),      // Terminal Bell
        "backspace" => Some('\x08'.to_string()), // Backspace

        // Ansi Erase Sequences
        "clear_screen" => Some("\x1b[J".to_string()), // clears the screen
        "clear_screen_from_cursor_to_end" => Some("\x1b[0J".to_string()), // clears from cursor until end of screen
        "clear_screen_from_cursor_to_beginning" => Some("\x1b[1J".to_string()), // clears from cursor to beginning of screen
        "cls" | "clear_entire_screen" => Some("\x1b[2J".to_string()), // clears the entire screen
        "erase_line" => Some("\x1b[K".to_string()),                   // clears the current line
        "erase_line_from_cursor_to_end" => Some("\x1b[0K".to_string()), // clears from cursor to end of line
        "erase_line_from_cursor_to_beginning" => Some("\x1b[1K".to_string()), // clears from cursor to start of line
        "erase_entire_line" => Some("\x1b[2K".to_string()),                   // clears entire line

        // Turn on/off cursor
        "cursor_off" => Some("\x1b[?25l".to_string()),
        "cursor_on" => Some("\x1b[?25h".to_string()),

        // Turn on/off blinking
        "cursor_blink_off" => Some("\x1b[?12l".to_string()),
        "cursor_blink_on" => Some("\x1b[?12h".to_string()),

        // Cursor position in ESC [ <r>;<c>R where r = row and c = column
        "cursor_position" => Some("\x1b[6n".to_string()),

        // Report Terminal Identity
        "identity" => Some("\x1b[0c".to_string()),

        // Ansi escape only - CSI command
        "csi" | "escape" | "escape_left" => Some("\x1b[".to_string()),
        // OSC escape (Operating system command)
        "osc" | "escape_right" => Some("\x1b]".to_string()),

        // Ansi RGB - Needs to be 32;2;r;g;b or 48;2;r;g;b
        // assuming the rgb will be passed via command and no here
        "rgb_fg" => Some("\x1b[32;2;".to_string()),
        "rgb_bg" => Some("\x1b[48;2;".to_string()),

        // Ansi color index - Needs 38;5;idx or 48;5;idx where idx = 0 to 255
        "idx_fg" | "color_idx_fg" => Some("\x1b[38;5;".to_string()),
        "idx_bg" | "color_idx_bg" => Some("\x1b[48;5;".to_string()),

        // Returns terminal size like "[<r>;<c>R" where r is rows and c is columns
        // This should work assuming your terminal is not greater than 999x999
        "size" => Some("\x1b[s\x1b[999;999H\x1b[6n\x1b[u".to_string()),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::Command;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Command {})
    }
}
