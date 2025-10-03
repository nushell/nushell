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
