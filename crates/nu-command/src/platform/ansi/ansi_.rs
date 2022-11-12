use lazy_static::lazy_static;
use nu_ansi_term::*;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call, engine::Command, Category, Example, IntoInterruptiblePipelineData, IntoPipelineData,
    PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct AnsiCommand;

struct AnsiCode {
    short_name: Option<&'static str>,
    long_name: &'static str,
    code: String,
}

lazy_static! {
    static ref CODE_LIST: Vec<AnsiCode> = vec!{
    AnsiCode{ short_name: Some("g"), long_name: "green", code: Color::Green.prefix().to_string()},
    AnsiCode{ short_name: Some("gb"), long_name: "green_bold", code: Color::Green.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("gu"), long_name: "green_underline", code: Color::Green.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("gi"), long_name: "green_italic", code: Color::Green.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("gd"), long_name: "green_dimmed", code: Color::Green.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("gr"), long_name: "green_reverse", code: Color::Green.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("lg"), long_name: "light_green", code: Color::LightGreen.prefix().to_string()},
    AnsiCode{ short_name: Some("lgb"), long_name: "light_green_bold", code: Color::LightGreen.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("lgu"), long_name: "light_green_underline", code: Color::LightGreen.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("lgi"), long_name: "light_green_italic", code: Color::LightGreen.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("lgd"), long_name: "light_green_dimmed", code: Color::LightGreen.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("lgr"), long_name: "light_green_reverse", code: Color::LightGreen.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("r"), long_name: "red", code: Color::Red.prefix().to_string()},
    AnsiCode{ short_name: Some("rb"), long_name: "red_bold", code: Color::Red.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("ru"), long_name: "red_underline", code: Color::Red.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("ri"), long_name: "red_italic", code: Color::Red.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("rd"), long_name: "red_dimmed", code: Color::Red.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("rr"), long_name: "red_reverse", code: Color::Red.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("lr"), long_name: "light_red", code: Color::LightRed.prefix().to_string()},
    AnsiCode{ short_name: Some("lrb"), long_name: "light_red_bold", code: Color::LightRed.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("lru"), long_name: "light_red_underline", code: Color::LightRed.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("lri"), long_name: "light_red_italic", code: Color::LightRed.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("lrd"), long_name: "light_red_dimmed", code: Color::LightRed.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("lrr"), long_name: "light_red_reverse", code: Color::LightRed.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("u"), long_name: "blue", code: Color::Blue.prefix().to_string()},
    AnsiCode{ short_name: Some("ub"), long_name: "blue_bold", code: Color::Blue.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("uu"), long_name: "blue_underline", code: Color::Blue.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("ui"), long_name: "blue_italic", code: Color::Blue.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("ud"), long_name: "blue_dimmed", code: Color::Blue.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("ur"), long_name: "blue_reverse", code: Color::Blue.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("lu"), long_name: "light_blue", code: Color::LightBlue.prefix().to_string()},
    AnsiCode{ short_name: Some("lub"), long_name: "light_blue_bold", code: Color::LightBlue.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("luu"), long_name: "light_blue_underline", code: Color::LightBlue.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("lui"), long_name: "light_blue_italic", code: Color::LightBlue.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("lud"), long_name: "light_blue_dimmed", code: Color::LightBlue.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("lur"), long_name: "light_blue_reverse", code: Color::LightBlue.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("b"), long_name: "black", code: Color::Black.prefix().to_string()},
    AnsiCode{ short_name: Some("bb"), long_name: "black_bold", code: Color::Black.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("bu"), long_name: "black_underline", code: Color::Black.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("bi"), long_name: "black_italic", code: Color::Black.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("bd"), long_name: "black_dimmed", code: Color::Black.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("br"), long_name: "black_reverse", code: Color::Black.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("ligr"), long_name: "light_gray", code: Color::LightGray.prefix().to_string()},
    AnsiCode{ short_name: Some("ligrb"), long_name: "light_gray_bold", code: Color::LightGray.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("ligru"), long_name: "light_gray_underline", code: Color::LightGray.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("ligri"), long_name: "light_gray_italic", code: Color::LightGray.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("ligrd"), long_name: "light_gray_dimmed", code: Color::LightGray.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("ligrr"), long_name: "light_gray_reverse", code: Color::LightGray.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("y"), long_name: "yellow", code: Color::Yellow.prefix().to_string()},
    AnsiCode{ short_name: Some("yb"), long_name: "yellow_bold", code: Color::Yellow.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("yu"), long_name: "yellow_underline", code: Color::Yellow.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("yi"), long_name: "yellow_italic", code: Color::Yellow.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("yd"), long_name: "yellow_dimmed", code: Color::Yellow.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("yr"), long_name: "yellow_reverse", code: Color::Yellow.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("ly"), long_name: "light_yellow", code: Color::LightYellow.prefix().to_string()},
    AnsiCode{ short_name: Some("lyb"), long_name: "light_yellow_bold", code: Color::LightYellow.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("lyu"), long_name: "light_yellow_underline", code: Color::LightYellow.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("lyi"), long_name: "light_yellow_italic", code: Color::LightYellow.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("lyd"), long_name: "light_yellow_dimmed", code: Color::LightYellow.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("lyr"), long_name: "light_yellow_reverse", code: Color::LightYellow.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("p"), long_name: "purple", code: Color::Purple.prefix().to_string()},
    AnsiCode{ short_name: Some("pb"), long_name: "purple_bold", code: Color::Purple.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("pu"), long_name: "purple_underline", code: Color::Purple.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("pi"), long_name: "purple_italic", code: Color::Purple.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("pd"), long_name: "purple_dimmed", code: Color::Purple.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("pr"), long_name: "purple_reverse", code: Color::Purple.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("lp"), long_name: "light_purple", code: Color::LightPurple.prefix().to_string()},
    AnsiCode{ short_name: Some("lpb"), long_name: "light_purple_bold", code: Color::LightPurple.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("lpu"), long_name: "light_purple_underline", code: Color::LightPurple.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("lpi"), long_name: "light_purple_italic", code: Color::LightPurple.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("lpd"), long_name: "light_purple_dimmed", code: Color::LightPurple.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("lpr"), long_name: "light_purple_reverse", code: Color::LightPurple.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("c"), long_name: "cyan", code: Color::Cyan.prefix().to_string()},
    AnsiCode{ short_name: Some("cb"), long_name: "cyan_bold", code: Color::Cyan.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("cu"), long_name: "cyan_underline", code: Color::Cyan.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("ci"), long_name: "cyan_italic", code: Color::Cyan.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("cd"), long_name: "cyan_dimmed", code: Color::Cyan.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("cr"), long_name: "cyan_reverse", code: Color::Cyan.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("lc"), long_name: "light_cyan", code: Color::LightCyan.prefix().to_string()},
    AnsiCode{ short_name: Some("lcb"), long_name: "light_cyan_bold", code: Color::LightCyan.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("lcu"), long_name: "light_cyan_underline", code: Color::LightCyan.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("lci"), long_name: "light_cyan_italic", code: Color::LightCyan.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("lcd"), long_name: "light_cyan_dimmed", code: Color::LightCyan.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("lcr"), long_name: "light_cyan_reverse", code: Color::LightCyan.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("w"), long_name: "white", code: Color::White.prefix().to_string()},
    AnsiCode{ short_name: Some("wb"), long_name: "white_bold", code: Color::White.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("wu"), long_name: "white_underline", code: Color::White.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("wi"), long_name: "white_italic", code: Color::White.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("wd"), long_name: "white_dimmed", code: Color::White.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("wr"), long_name: "white_reverse", code: Color::White.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("dgr"), long_name: "dark_gray", code: Color::DarkGray.prefix().to_string()},
    AnsiCode{ short_name: Some("dgrb"), long_name: "dark_gray_bold", code: Color::DarkGray.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("dgru"), long_name: "dark_gray_underline", code: Color::DarkGray.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("dgri"), long_name: "dark_gray_italic", code: Color::DarkGray.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("dgrd"), long_name: "dark_gray_dimmed", code: Color::DarkGray.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("dgrr"), long_name: "dark_gray_reverse", code: Color::DarkGray.reverse().prefix().to_string()},

    AnsiCode{ short_name: Some("def"), long_name: "default", code: Color::Default.prefix().to_string()},
    AnsiCode{ short_name: Some("defb"), long_name: "default_bold", code: Color::Default.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("defu"), long_name: "default_underline", code: Color::Default.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("defi"), long_name: "default_italic", code: Color::Default.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("defd"), long_name: "default_dimmed", code: Color::Default.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("defr"), long_name: "default_reverse", code: Color::Default.reverse().prefix().to_string()},

    AnsiCode{ short_name: None, long_name: "reset", code: "\x1b[0m".to_owned()},
    // Reference for ansi codes https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797
    // Another good reference http://ascii-table.com/ansi-escape-sequences.php

    // For setting title like `echo [(char title) (pwd) (char bel)] | str join`
    AnsiCode{short_name: None, long_name:"title", code: "\x1b]2;".to_string()}, // ESC]2; xterm sets window title using OSC syntax escapes

    // Ansi Erase Sequences
    AnsiCode{ short_name: None, long_name:"clear_screen", code: "\x1b[J".to_string()}, // clears the screen
    AnsiCode{ short_name: None, long_name:"clear_screen_from_cursor_to_end", code: "\x1b[0J".to_string()}, // clears from cursor until end of screen
    AnsiCode{ short_name: None, long_name:"clear_screen_from_cursor_to_beginning", code: "\x1b[1J".to_string()}, // clears from cursor to beginning of screen
    AnsiCode{ short_name: Some("cls"), long_name:"clear_entire_screen", code: "\x1b[2J".to_string()}, // clears the entire screen
    AnsiCode{ short_name: Some("clsb"), long_name:"clear_entire_screen_plus_buffer", code: "\x1b[3J".to_string()}, // clear entire screen and delete all lines saved in the scrollback buffer
    AnsiCode{ short_name: None, long_name:"erase_line", code: "\x1b[K".to_string()},                   // clears the current line
    AnsiCode{ short_name: None, long_name:"erase_line_from_cursor_to_end", code: "\x1b[0K".to_string()}, // clears from cursor to end of line
    AnsiCode{ short_name: None, long_name:"erase_line_from_cursor_to_beginning", code: "\x1b[1K".to_string()}, // clears from cursor to start of line
    AnsiCode{ short_name: None, long_name:"erase_entire_line", code: "\x1b[2K".to_string()},                   // clears entire line

    // Turn on/off cursor
    AnsiCode{ short_name: None, long_name:"cursor_off", code: "\x1b[?25l".to_string()},
    AnsiCode{ short_name: None, long_name:"cursor_on", code: "\x1b[?25h".to_string()},
    AnsiCode{ short_name: Some("home"), long_name:"cursor_home", code: "\x1b[H".to_string()},

    // Turn on/off blinking
    AnsiCode{ short_name: None, long_name:"cursor_blink_off", code: "\x1b[?12l".to_string()},
    AnsiCode{ short_name: None, long_name:"cursor_blink_on", code: "\x1b[?12h".to_string()},

    // Cursor position in ESC [ <r>;<c>R where r = row and c = column
    AnsiCode{ short_name: None, long_name:"cursor_position", code: "\x1b[6n".to_string()},

    // Report Terminal Identity
    AnsiCode{ short_name: None, long_name:"identity", code: "\x1b[0c".to_string()},

    // Ansi escape only - CSI command
    AnsiCode{ short_name: Some("esc"), long_name: "escape", code: "\x1b".to_string()},
    // Ansi escape only - CSI command
    AnsiCode{ short_name: Some("csi"), long_name: "escape_left", code: "\x1b[".to_string()},
    // OSC escape (Operating system command)
    AnsiCode{ short_name: Some("osc"), long_name:"escape_right", code: "\x1b]".to_string()},
    // OSC string terminator
    AnsiCode{ short_name: Some("st"), long_name:"string_terminator", code: "\x1b\\".to_string()},

    // Ansi Rgb - Needs to be 32;2;r;g;b or 48;2;r;g;b
    // assuming the rgb will be passed via command and no here
    AnsiCode{ short_name: None, long_name:"rgb_fg", code: "\x1b[38;2;".to_string()},
    AnsiCode{ short_name: None, long_name:"rgb_bg", code: "\x1b[48;2;".to_string()},

    // Ansi color index - Needs 38;5;idx or 48;5;idx where idx = 0 to 255
    AnsiCode{ short_name: Some("idx_fg"), long_name: "color_idx_fg", code: "\x1b[38;5;".to_string()},
    AnsiCode{ short_name: Some("idx_bg"), long_name:"color_idx_bg", code: "\x1b[48;5;".to_string()},

    // Returns terminal size like "[<r>;<c>R" where r is rows and c is columns
    // This should work assuming your terminal is not greater than 999x999
    AnsiCode{ short_name: None, long_name:"size", code: "\x1b[s\x1b[999;999H\x1b[6n\x1b[u".to_string()},};

    static ref CODE_MAP: HashMap<&'static str, &'static str > = build_ansi_hashmap(&CODE_LIST);
}

impl Command for AnsiCommand {
    fn name(&self) -> &str {
        "ansi"
    }

    fn signature(&self) -> Signature {
        Signature::build("ansi")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .optional(
                "code",
                SyntaxShape::Any,
                "the name of the code to use like 'green' or 'reset' to reset the color",
            )
            .switch(
                "escape", // \x1b[
                "escape sequence without the escape character(s)",
                Some('e'),
            )
            .switch(
                "osc", // \x1b]
                "operating system command (ocs) escape sequence without the escape character(s)",
                Some('o'),
            )
            .switch("list", "list available ansi code names", Some('l'))
            .category(Category::Platform)
    }

    fn usage(&self) -> &str {
        "Output ANSI codes to change color."
    }

    fn extra_usage(&self) -> &str {
        r#"For escape sequences:
Escape: '\x1b[' is not required for --escape parameter
Format: #(;#)m
Example: 1;31m for bold red or 2;37;41m for dimmed white fg with red bg
There can be multiple text formatting sequence numbers
separated by a ; and ending with an m where the # is of the
following values:
    attribute_number, abbreviation, description
    0     reset / normal display
    1  b  bold or increased intensity
    2  d  faint or decreased intensity
    3  i  italic on (non-mono font)
    4  u  underline on
    5  l  slow blink on
    6     fast blink on
    7  r  reverse video on
    8  h  nondisplayed (invisible) on
    9  s  strike-through on

    foreground/bright colors    background/bright colors
    30/90    black              40/100    black
    31/91    red                41/101    red
    32/92    green              42/102    green
    33/93    yellow             43/103    yellow
    34/94    blue               44/104    blue
    35/95    magenta            45/105    magenta
    36/96    cyan               46/106    cyan
    37/97    white              47/107    white
    39       default            49        default
    https://en.wikipedia.org/wiki/ANSI_escape_code

OSC: '\x1b]' is not required for --osc parameter
Example: echo [(ansi -o '0') 'some title' (char bel)] | str join
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
                result: Some(Value::test_string("\u{1b}[32m")),
            },
            Example {
                description: "Reset the color",
                example: r#"ansi reset"#,
                result: Some(Value::test_string("\u{1b}[0m")),
            },
            Example {
                description:
                    "Use ansi to color text (rb = red bold, gb = green bold, pb = purple bold)",
                example: r#"$'(ansi rb)Hello (ansi gb)Nu (ansi pb)World(ansi reset)'"#,
                result: Some(Value::test_string(
                    "\u{1b}[1;31mHello \u{1b}[1;32mNu \u{1b}[1;35mWorld\u{1b}[0m",
                )),
            },
            Example {
                description: "Use ansi to color text (italic bright yellow on red 'Hello' with green bold 'Nu' and purple bold 'World')",
                example: r#"[(ansi -e '3;93;41m') Hello (ansi reset) " " (ansi gb) Nu " " (ansi pb) World (ansi reset)] | str join"#,
                result: None,
                // Test disabled because the final expression in the pipeline is
                // not the command being tested, and this violated assumptions
                // made by the run-time input/output type-checking tests.
                // result: Some(Value::test_string(
                //     "\u{1b}[3;93;41mHello\u{1b}[0m \u{1b}[1;32mNu \u{1b}[1;35mWorld\u{1b}[0m",
                // )),
            },
            Example {
                description: "Use ansi to color text with a style (blue on red in bold)",
                example: r#"$"(ansi -e { fg: '#0000ff' bg: '#ff0000' attr: b })Hello Nu World(ansi reset)""#,
                result: Some(Value::test_string(
                    "\u{1b}[1;48;2;255;0;0;38;2;0;0;255mHello Nu World\u{1b}[0m",
                )),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["text-color", "text-style", "colors"]
    }

    fn run(
        &self,
        engine_state: &nu_protocol::engine::EngineState,
        stack: &mut nu_protocol::engine::Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let list: bool = call.has_flag("list");
        let escape: bool = call.has_flag("escape");
        let osc: bool = call.has_flag("osc");
        let use_ansi_coloring = engine_state.get_config().use_ansi_coloring;

        if list {
            return generate_ansi_code_list(engine_state, call.head, use_ansi_coloring);
        }

        // The code can now be one of the ansi abbreviations like green_bold
        // or it can be a record like this: { fg: "#ff0000" bg: "#00ff00" attr: bli }
        // this record is defined in nu-color-config crate
        let code: Value = match call.opt(engine_state, stack, 0)? {
            Some(c) => c,
            None => return Err(ShellError::MissingParameter("code".into(), call.head)),
        };

        let param_is_string = matches!(code, Value::String { val: _, span: _ });

        if escape && osc {
            return Err(ShellError::IncompatibleParameters {
                left_message: "escape".into(),
                left_span: call
                    .get_named_arg("escape")
                    .expect("Unexpected missing argument")
                    .span,
                right_message: "osc".into(),
                right_span: call
                    .get_named_arg("osc")
                    .expect("Unexpected missing argument")
                    .span,
            });
        }

        let code_string = if param_is_string {
            code.as_string().expect("error getting code as string")
        } else {
            "".to_string()
        };

        let param_is_valid_string = param_is_string && !code_string.is_empty();

        if (escape || osc) && (param_is_valid_string) {
            let code_vec: Vec<char> = code_string.chars().collect();
            if code_vec[0] == '\\' {
                return Err(ShellError::UnsupportedInput(
                    String::from("no need for escape characters"),
                    call.get_flag_expr("escape")
                        .expect("Unexpected missing argument")
                        .span,
                ));
            }
        }

        let output = if escape && param_is_valid_string {
            format!("\x1b[{}", code_string)
        } else if osc && param_is_valid_string {
            // Operating system command aka osc  ESC ] <- note the right brace, not left brace for osc
            // OCS's need to end with a bell '\x07' char
            format!("\x1b]{};", code_string)
        } else if param_is_valid_string {
            // parse hex colors like #00FF00
            if code_string.starts_with('#') {
                match nu_color_config::color_from_hex(&code_string) {
                    Ok(color) => match color {
                        Some(c) => c.prefix().to_string(),
                        None => Color::White.prefix().to_string(),
                    },
                    Err(err) => {
                        return Err(ShellError::GenericError(
                            "error parsing hex color".to_string(),
                            format!("{}", err),
                            Some(code.span()?),
                            None,
                            Vec::new(),
                        ));
                    }
                }
            } else {
                match str_to_ansi(&code_string) {
                    Some(c) => c,
                    None => {
                        return Err(ShellError::UnsupportedInput(
                            String::from("Unknown ansi code"),
                            call.positional_nth(0)
                                .expect("Unexpected missing argument")
                                .span,
                        ))
                    }
                }
            }
        } else {
            // This is a record that should look like
            // { fg: "#ff0000" bg: "#00ff00" attr: bli }
            let record = code.as_record()?;
            // create a NuStyle to parse the information into
            let mut nu_style = nu_color_config::NuStyle {
                fg: None,
                bg: None,
                attr: None,
            };
            // Iterate and populate NuStyle with real values
            for (k, v) in record.0.iter().zip(record.1) {
                match k.as_str() {
                    "fg" => nu_style.fg = Some(v.as_string()?),
                    "bg" => nu_style.bg = Some(v.as_string()?),
                    "attr" => nu_style.attr = Some(v.as_string()?),
                    _ => {
                        return Err(ShellError::IncompatibleParametersSingle(
                            format!("problem with key: {}", k),
                            code.span().expect("error with span"),
                        ))
                    }
                }
            }
            // Now create a nu_ansi_term::Style from the NuStyle
            let style = nu_color_config::parse_nustyle(nu_style);
            // Return the prefix string. The prefix is the Ansi String. The suffix would be 0m, reset/stop coloring.
            style.prefix().to_string()
        };

        Ok(Value::string(output, call.head).into_pipeline_data())
    }
}

pub fn str_to_ansi(s: &str) -> Option<String> {
    CODE_MAP.get(s).map(|x| String::from(*x))
}

fn generate_ansi_code_list(
    engine_state: &nu_protocol::engine::EngineState,
    call_span: Span,
    use_ansi_coloring: bool,
) -> Result<nu_protocol::PipelineData, ShellError> {
    return Ok(CODE_LIST
        .iter()
        .enumerate()
        .map(move |(i, ansi_code)| {
            let cols = if use_ansi_coloring {
                vec![
                    "name".into(),
                    "preview".into(),
                    "short name".into(),
                    "code".into(),
                ]
            } else {
                vec!["name".into(), "short name".into(), "code".into()]
            };
            let name: Value = Value::string(String::from(ansi_code.long_name), call_span);
            let short_name = Value::string(ansi_code.short_name.unwrap_or(""), call_span);
            // The first 102 items in the ansi array are colors
            let preview = if i < 102 {
                Value::string(format!("{}NUSHELL\u{1b}[0m", &ansi_code.code), call_span)
            } else {
                Value::string("\u{1b}[0m", call_span)
            };
            let code_string = String::from(&ansi_code.code.replace('\u{1b}', ""));
            let code = Value::string(code_string, call_span);
            let vals = if use_ansi_coloring {
                vec![name, preview, short_name, code]
            } else {
                vec![name, short_name, code]
            };
            Value::Record {
                cols,
                vals,
                span: call_span,
            }
        })
        .into_pipeline_data(engine_state.ctrlc.clone()));
}

fn build_ansi_hashmap(v: &'static [AnsiCode]) -> HashMap<&'static str, &'static str> {
    let mut result = HashMap::new();
    for code in v.iter() {
        let value: &'static str = &code.code;
        if let Some(sn) = code.short_name {
            result.insert(sn, value);
        }
        result.insert(code.long_name, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::platform::ansi::ansi_::AnsiCommand;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;

        test_examples(AnsiCommand {})
    }
}
