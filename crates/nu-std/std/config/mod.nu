# Returns a dark-mode theme that can be assigned to $env.config.color_config
export def dark-theme [] {
    {
        # color for nushell primitives
        separator: default
        leading_trailing_space_bg: { attr: n } # no fg, no bg, attr none effectively turns this off
        header: green_bold
        empty: blue
        # Closures can be used to choose colors for specific values.
        # The value (in this case, a bool) is piped into the closure.
        # eg) {|| if $in { 'light_cyan' } else { 'light_gray' } }
        bool: light_cyan
        int: default
        filesize: cyan
        duration: default
        datetime: purple
        range: default
        float: default
        string: default
        nothing: default
        binary: default
        # binary hex viewer styles (table / explore)
        binary_null_char: grey42
        binary_printable: cyan_bold
        binary_whitespace: green_bold
        binary_ascii_other: purple_bold
        binary_non_ascii: yellow_bold
        cell-path: default
        row_index: green_bold
        record: default
        list: default
        block: default
        hints: dark_gray
        search_result: { bg: red fg: white }
        
        shape_binary: purple_bold
        shape_block: blue_bold
        shape_bool: light_cyan
        shape_closure: green_bold
        shape_custom: green
        shape_datetime: cyan_bold
        shape_directory: cyan
        shape_external: cyan
        shape_externalarg: green_bold
        shape_external_resolved: light_yellow_bold
        shape_filepath: cyan
        shape_flag: blue_bold
        shape_float: purple_bold
        # shapes are used to change the cli syntax highlighting
        shape_garbage: { fg: white bg: red attr: b }
        shape_glob_interpolation: cyan_bold
        shape_globpattern: cyan_bold
        shape_int: purple_bold
        shape_internalcall: cyan_bold
        shape_keyword: cyan_bold
        shape_list: cyan_bold
        shape_literal: blue
        shape_match_pattern: green
        shape_matching_brackets: { attr: u }
        shape_nothing: light_cyan
        shape_operator: yellow
        shape_pipe: purple_bold
        shape_range: yellow_bold
        shape_record: cyan_bold
        shape_redirection: purple_bold
        shape_signature: green_bold
        shape_string: green
        shape_string_interpolation: cyan_bold
        shape_table: blue_bold
        shape_variable: purple
        shape_vardecl: purple
        shape_raw_string: light_purple
    }
}

# Returns a light-mode theme that can be assigned to $env.config.color_config
export def light-theme [] {
    {
        # color for nushell primitives
        separator: dark_gray
        leading_trailing_space_bg: { attr: n } # no fg, no bg, attr none effectively turns this off
        header: green_bold
        empty: blue
        # Closures can be used to choose colors for specific values.
        # The value (in this case, a bool) is piped into the closure.
        # eg) {|| if $in { 'darkcyan' } else { 'dark_gray' } }
        bool: darkcyan
        int: dark_gray
        filesize: cyan_bold
        duration: dark_gray
        datetime: purple
        range: dark_gray
        float: dark_gray
        string: dark_gray
        nothing: dark_gray
        binary: dark_gray
        # binary hex viewer styles (table / explore)
        binary_null_char: grey42
        binary_printable: cyan_bold
        binary_whitespace: green_bold
        binary_ascii_other: purple_bold
        binary_non_ascii: yellow_bold
        cell-path: dark_gray
        row_index: green_bold
        record: dark_gray
        list: dark_gray
        block: dark_gray
        hints: dark_gray
        search_result: { fg: white bg: red }

        shape_binary: purple_bold
        shape_block: blue_bold
        shape_bool: light_cyan
        shape_closure: green_bold
        shape_custom: green
        shape_datetime: cyan_bold
        shape_directory: cyan
        shape_external: cyan
        shape_externalarg: green_bold
        shape_external_resolved: light_purple_bold
        shape_filepath: cyan
        shape_flag: blue_bold
        shape_float: purple_bold
        # shapes are used to change the cli syntax highlighting
        shape_garbage: { fg: white bg: red attr: b }
        shape_glob_interpolation: cyan_bold
        shape_globpattern: cyan_bold
        shape_int: purple_bold
        shape_internalcall: cyan_bold
        shape_keyword: cyan_bold
        shape_list: cyan_bold
        shape_literal: blue
        shape_match_pattern: green
        shape_matching_brackets: { attr: u }
        shape_nothing: light_cyan
        shape_operator: yellow
        shape_pipe: purple_bold
        shape_range: yellow_bold
        shape_record: cyan_bold
        shape_redirection: purple_bold
        shape_signature: green_bold
        shape_string: green
        shape_string_interpolation: cyan_bold
        shape_table: blue_bold
        shape_variable: purple
        shape_vardecl: purple
        shape_raw_string: light_purple
    }
}

# Returns helper closures that can be used for ENV_CONVERSIONS and other purposes
export def env-conversions [] {
    {
        "path": {
            from_string: {|s| $s | split row (char esep) | path expand --no-symlink }
            to_string: {|v| $v | path expand --no-symlink | str join (char esep) }
        }
    }
}

# Asks the terminal for one of its colors: `osc` is the number of the OSC query
# for it, 10 for the foreground and 11 for the background.
#
# Returns the color as "#rrggbb", or nothing when there is no terminal to ask or
# it does not report its colors.
def terminal-color [osc: int]: [nothing -> oneof<string, nothing>] {
    # The query is written to stdout and its answer read from stdin.
    if not ((is-terminal --stdin) and (is-terminal --stdout)) {
        return null
    }
    # The cursor position, which terminals do report, is asked for after the
    # color and read up to its ending "R": a terminal that does not report its
    # colors still answers, instead of keeping this waiting. `term query` has no
    # timeout, so one that answers neither keeps it waiting until Ctrl+C.
    let reply = try {
        term query $"(ansi osc)($osc);?(ansi st)(ansi cursor_position)" --terminator R | decode
    } catch {
        return null
    }
    let channels = $reply | parse --regex 'rgba?:(?<r>[[:xdigit:]]+)/(?<g>[[:xdigit:]]+)/(?<b>[[:xdigit:]]+)'
    if ($channels | is-empty) {
        return null
    }
    # A channel has one to four digits. Its first two are the "rr" of "#rrggbb",
    # and a single one is repeated, as "f" is "ff".
    $channels | first | values | each { $"($in)($in)" | str substring 0..1 } | str join | $"#($in)"
}

# Returns the background color of the terminal as "#rrggbb", or nothing when the
# terminal does not report it
#
# The terminal is asked, and its answer is read from the same input as what is
# typed: keys typed before the answer comes are lost. Ask once, when nushell
# starts.
#
# Nothing is also what comes back over ssh to Windows, where the answer of the
# terminal does not get through. Where nothing answers at all, as in a pty with
# no terminal behind it, this waits until Ctrl+C.
@example "Stripe tables with a color made from the background of the terminal, when it reports it" {
    let background = terminal-background
    if $background != null { $env.config.color_config.row_stripe_bg = $background }
}
export def terminal-background []: [nothing -> oneof<string, nothing>] {
    terminal-color 11
}

# Returns the foreground (text) color of the terminal as "#rrggbb", or nothing
# when the terminal does not report it
#
# The terminal is asked, and its answer is read from the same input as what is
# typed: keys typed before the answer comes are lost. Ask once, when nushell
# starts.
#
# Nothing is also what comes back over ssh to Windows, where the answer of the
# terminal does not get through. Where nothing answers at all, as in a pty with
# no terminal behind it, this waits until Ctrl+C.
@example "Get the color of the text of the terminal, or white when it does not report it" {
    terminal-foreground | default "#ffffff"
}
export def terminal-foreground []: [nothing -> oneof<string, nothing>] {
    terminal-color 10
}
