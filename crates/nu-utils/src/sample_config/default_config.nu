# Nushell Config File
#
# version = 0.82.1

# For more information on defining custom themes, see
# https://www.nushell.sh/book/coloring_and_theming.html
# And here is the theme collection
# https://github.com/nushell/nu_scripts/tree/main/themes
let dark_theme = {
    # color for nushell primitives
    separator: white
    leading_trailing_space_bg: { attr: n } # no fg, no bg, attr none effectively turns this off
    header: green_bold
    empty: blue
    # Closures can be used to choose colors for specific values.
    # The value (in this case, a bool) is piped into the closure.
    bool: {|| if $in { 'light_cyan' } else { 'light_gray' } }
    int: white
    filesize: {|e|
      if $e == 0b {
        'white'
      } else if $e < 1mb {
        'cyan'
      } else { 'blue' }
    }
    duration: white
    date: {|| (date now) - $in |
      if $in < 1hr {
        'purple'
      } else if $in < 6hr {
        'red'
      } else if $in < 1day {
        'yellow'
      } else if $in < 3day {
        'green'
      } else if $in < 1wk {
        'light_green'
      } else if $in < 6wk {
        'cyan'
      } else if $in < 52wk {
        'blue'
      } else { 'dark_gray' }
    }
    range: white
    float: white
    string: white
    nothing: white
    binary: white
    cellpath: white
    row_index: green_bold
    record: white
    list: white
    block: white
    hints: dark_gray
    search_result: {bg: red fg: white}

    shape_and: purple_bold
    shape_binary: purple_bold
    shape_block: blue_bold
    shape_bool: light_cyan
    shape_closure: green_bold
    shape_custom: green
    shape_datetime: cyan_bold
    shape_directory: cyan
    shape_external: cyan
    shape_externalarg: green_bold
    shape_filepath: cyan
    shape_flag: blue_bold
    shape_float: purple_bold
    # shapes are used to change the cli syntax highlighting
    shape_garbage: { fg: white bg: red attr: b}
    shape_globpattern: cyan_bold
    shape_int: purple_bold
    shape_internalcall: cyan_bold
    shape_list: cyan_bold
    shape_literal: blue
    shape_match_pattern: green
    shape_matching_brackets: { attr: u }
    shape_nothing: light_cyan
    shape_operator: yellow
    shape_or: purple_bold
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
}

let light_theme = {
    # color for nushell primitives
    separator: dark_gray
    leading_trailing_space_bg: { attr: n } # no fg, no bg, attr none effectively turns this off
    header: green_bold
    empty: blue
    # Closures can be used to choose colors for specific values.
    # The value (in this case, a bool) is piped into the closure.
    bool: {|| if $in { 'dark_cyan' } else { 'dark_gray' } }
    int: dark_gray
    filesize: {|e|
      if $e == 0b {
        'dark_gray'
      } else if $e < 1mb {
        'cyan_bold'
      } else { 'blue_bold' }
    }
    duration: dark_gray
  date: {|| (date now) - $in |
    if $in < 1hr {
      'purple'
    } else if $in < 6hr {
      'red'
    } else if $in < 1day {
      'yellow'
    } else if $in < 3day {
      'green'
    } else if $in < 1wk {
      'light_green'
    } else if $in < 6wk {
      'cyan'
    } else if $in < 52wk {
      'blue'
    } else { 'dark_gray' }
  }
    range: dark_gray
    float: dark_gray
    string: dark_gray
    nothing: dark_gray
    binary: dark_gray
    cellpath: dark_gray
    row_index: green_bold
    record: white
    list: white
    block: white
    hints: dark_gray
    search_result: {fg: white bg: red}

    shape_and: purple_bold
    shape_binary: purple_bold
    shape_block: blue_bold
    shape_bool: light_cyan
    shape_closure: green_bold
    shape_custom: green
    shape_datetime: cyan_bold
    shape_directory: cyan
    shape_external: cyan
    shape_externalarg: green_bold
    shape_filepath: cyan
    shape_flag: blue_bold
    shape_float: purple_bold
    # shapes are used to change the cli syntax highlighting
    shape_garbage: { fg: white bg: red attr: b}
    shape_globpattern: cyan_bold
    shape_int: purple_bold
    shape_internalcall: cyan_bold
    shape_list: cyan_bold
    shape_literal: blue
    shape_match_pattern: green
    shape_matching_brackets: { attr: u }
    shape_nothing: light_cyan
    shape_operator: yellow
    shape_or: purple_bold
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
}

# External completer example
# let carapace_completer = {|spans|
#     carapace $spans.0 nushell $spans | from json
# }


# The default config record. This is where much of your global configuration is setup.
$env.config = {
  show_banner: true # true or false to enable or disable the welcome banner at startup

  ls: {
    use_ls_colors: true # use the LS_COLORS environment variable to colorize output
    clickable_links: true # enable or disable clickable links. Your terminal has to support links.
  }

  rm: {
    always_trash: false # always act as if -t was given. Can be overridden with -p
  }

  cd: {
    abbreviations: false # allows `cd s/o/f` to expand to `cd some/other/folder`
  }

  table: {
    mode: rounded # basic, compact, compact_double, light, thin, with_love, rounded, reinforced, heavy, none, other
    index_mode: always # "always" show indexes, "never" show indexes, "auto" = show indexes when a table has "index" column
    show_empty: true # show 'empty list' and 'empty record' placeholders for command output
    trim: {
      methodology: wrapping # wrapping or truncating
      wrapping_try_keep_words: true # A strategy used by the 'wrapping' methodology
      truncating_suffix: "..." # A suffix used by the 'truncating' methodology
    }
  }

  # datetime_format determines what a datetime rendered in the shell would look like.
  # Behavior without this configuration point will be to "humanize" the datetime display,
  # showing something like "a day ago."
  datetime_format: {
    # normal: '%a, %d %b %Y %H:%M:%S %z'  # shows up in displays of variables or other datetime's outside of tables
    # table: '%m/%d/%y %I:%M:%S%p'        # generally shows up in tabular outputs such as ls. commenting this out will change it to the default human readable datetime format
  }

  explore: {
    try: {
        border_color: {fg: "white"}
    },
    status_bar_background: {fg: "#1D1F21", bg: "#C4C9C6"},
    command_bar_text: {fg: "#C4C9C6"},
    highlight: {fg: "black", bg: "yellow"},
    status: {
        error: {fg: "white", bg: "red"},
        warn: {}
        info: {}
    },
    table: {
        split_line: {fg: "#404040"},
        selected_cell: {},
        selected_row: {},
        selected_column: {},
        cursor: true,
        line_head_top: true,
        line_head_bottom: true,
        line_shift: true,
        line_index: true,
    },
    config: {
        border_color: {fg: "white"}
        cursor_color: {fg: "black", bg: "light_yellow"}
    },
  }

  history: {
    max_size: 100_000 # Session has to be reloaded for this to take effect
    sync_on_enter: true # Enable to share history between multiple sessions, else you have to close the session to write history to file
    file_format: "plaintext" # "sqlite" or "plaintext"
    isolation: true # true enables history isolation, false disables it. true will allow the history to be isolated to the current session. false will allow the history to be shared across all sessions.
  }

  completions: {
    case_sensitive: false # set to true to enable case-sensitive completions
    quick: true  # set this to false to prevent auto-selecting completions when only one remains
    partial: true  # set this to false to prevent partial filling of the prompt
    algorithm: "prefix"  # prefix or fuzzy
    external: {
      enable: true # set to false to prevent nushell looking into $env.PATH to find more suggestions, `false` recommended for WSL users as this look up may be very slow
      max_results: 100 # setting it lower can improve completion performance at the cost of omitting some options
      completer: null # check 'carapace_completer' above as an example
    }
  }

  filesize: {
    metric: true # true => KB, MB, GB (ISO standard), false => KiB, MiB, GiB (Windows standard)
    format: "auto" # b, kb, kib, mb, mib, gb, gib, tb, tib, pb, pib, eb, eib, auto
  }

  cursor_shape: {
    emacs: line # block, underscore, line, blink_block, blink_underscore, blink_line (line is the default)
    vi_insert: block # block, underscore, line , blink_block, blink_underscore, blink_line (block is the default)
    vi_normal: underscore # block, underscore, line, blink_block, blink_underscore, blink_line (underscore is the default)
  }

  color_config: {} # if you want a more interesting theme, you can replace the empty record with `$dark_theme`, `$light_theme` or another custom record
  use_grid_icons: true
  footer_mode: "25" # always, never, number_of_rows, auto
  float_precision: 2 # the precision for displaying floats in tables
  buffer_editor: "" # command that will be used to edit the current line buffer with ctrl+o, if unset fallback to $env.EDITOR and $env.VISUAL
  use_ansi_coloring: true
  bracketed_paste: true # enable bracketed paste, currently useless on windows
  edit_mode: emacs # emacs, vi
  shell_integration: false # enables terminal shell integration. Off by default, as some terminals have issues with this.
  render_right_prompt_on_last_line: false # true or false to enable or disable right prompt to be rendered on last line of the prompt.

  hooks: {
    pre_prompt: [{ null }] # run before the prompt is shown
    pre_execution: [{ null }] # run before the repl input is run
    env_change: {
      PWD: [{|before, after| null }] # run if the PWD environment is different since the last repl input
    }
    display_output: { table } # run before the output of a command is drawn
    command_not_found: { null } # return an error message when a command is not found
  }

  menus: [
      # Configuration for default nushell menus
      # Note the lack of source parameter
      {
        name: completion_menu
        only_buffer_difference: false
        marker: "| "
        type: {
            layout: columnar
            columns: 4
            col_width: 20   # Optional value. If missing all the screen width is used to calculate column width
            col_padding: 2
        }
        style: {
            text: green
            selected_text: green_reverse
            description_text: yellow
        }
      }
      {
        name: history_menu
        only_buffer_difference: true
        marker: "? "
        type: {
            layout: list
            page_size: 10
        }
        style: {
            text: green
            selected_text: green_reverse
            description_text: yellow
        }
      }
      {
        name: help_menu
        only_buffer_difference: true
        marker: "? "
        type: {
            layout: description
            columns: 4
            col_width: 20   # Optional value. If missing all the screen width is used to calculate column width
            col_padding: 2
            selection_rows: 4
            description_rows: 10
        }
        style: {
            text: green
            selected_text: green_reverse
            description_text: yellow
        }
      }
  ]

  keybindings: [
    {
      name: completion_menu
      modifier: none
      keycode: tab
      mode: [emacs vi_normal vi_insert]
      event: {
        until: [
          { send: menu name: completion_menu }
          { send: menunext }
        ]
      }
    }
    {
      name: history_menu
      modifier: control
      keycode: char_r
      mode: [emacs, vi_insert, vi_normal]
      event: { send: menu name: history_menu }
    }
    {
      name: help_menu
      modifier: control
      keycode: char_h
      mode: [emacs, vi_insert, vi_normal]
      event: { send: menu name: help_menu }
    }
  ]
}
