# Nushell Config File
#
# version = "0.99.2"
#
# A `config.nu` file is used to override default Nushell settings,
# define # (or import) custom commands, or run any other startup tasks.
# See https://www.nushell.sh/book/configuration.html
#
# Nushell sets "sensible defaults" for most configuration settings, so
# the user's `config.nu` only needs to override these defaults if
# desired.
#
# This file serves as simple "in-shell" documentation for these
# settings, or you can view a more complete discussion online at:
# https://nushell.sh/book/configuration
#
# You can pretty-print and page this file using:
# config nu --sample | nu-highlight | less -R

# $env.config
# -----------
# The $env.config environment variable is a record containing most Nushell
# configuration settings. Keep in mind that, as a record, setting it to a
# new record will remove any keys which aren't in the new record. Nushell
# will then automatically merge in the internal defaults for missing keys.
#
# The same holds true for keys in the $env.config which are also records
# or lists.
#
# For this reason, settings are typically changed by updating the value of
# a particular key. Merging a new config record is also possible. See the
# Configuration chapter of the book for more information.

# ------------------------
# History-related settings
# ------------------------
# $env.config.history.*

# file_format (string):  Either "sqlite" or "plaintext". While text-backed history
# is currently the default for historical reasons, "sqlite" is stable and
# provides more advanced history features.
$env.config.history.file_format = "sqlite"

# max_size (int): The maximum number of entries allowed in the history.
# After # exceeding this value, the oldest history items will be removed
# as new commands are added.
$env.config.history.max_size = 5_000_000

# sync_on_enter (bool): Whether the plaintext history file is updated
# each time a command is entered. If set to `false`, the plaintext history
# is only updated/written when the shell exits. This setting has no effect
# for SQLite-backed history.
$env.config.history.sync_on_enter = true

# isolation (bool):
# `true`: New history from other currently-open Nushell sessions is not
# seen when scrolling through the history using PrevHistory (typically
# the Up key) or NextHistory (Down key)
# `false`: All commands entered in other Nushell sessions will be mixed with
# those from the current shell.
# Note: Older history items (from before the current shell was started) are
# always shown.
# This setting only applies to SQLite-backed history
$env.config.history.isolation = true

# ----------------------
# Miscellaneous Settings
# ----------------------

# show_banner (bool): Enable or disable the welcome banner at startup
$env.config.show_banner = true

# rm.always_trash (bool):
# true: rm behaves as if the --trash/-t option is specified
# false: rm behaves as if the --permanent/-p option is specified (default)
# Explicitly calling `rm` with `--trash` or `--permanent` always override this setting
# Note that this feature is dependent on the host OS trashcan support.
$env.config.rm.always_trash = false

# recursion_limit (int): how many times a command can call itself recursively
# before an error will be generated.
$env.config.recursion_limit = 50

# ---------------------------
# Commandline Editor Settings
# ---------------------------

# edit_mode (string) "vi" or "emacs" sets the editing behavior of Reedline
edit_mode: "emacs"

# Command that will be used to edit the current line buffer with Ctrl+O.
# If unset, uses $env.VISUAL and then $env.EDITOR
#
# Tip: Set to "editor" to use the default editor on Unix platforms using
#      the Alternatives system or equivalent
buffer_editor: "editor"

# cursor_shape_* (string)
# -----------------------
# The following variables accept a string from the following selections:
# "block", "underscore", "line", "blink_block", "blink_underscore", "blink_line", or "inherit"
# "inherit" skips setting cursor shape and uses the current terminal setting.
$env.config.cursor_shape.emacs = "inherit"         # Cursor shape in emacs mode
$env.config.cursor_shape.vi_insert = "block"       # Cursor shape in vi-insert mode
$env.config.cursor_shape.vi_normal = "underscore"  # Cursor shape in normal vi mode

# --------------------
# Completions Behavior
# --------------------
# $env.config.completions.*
# Apply to the Nushell completion system

# algorithm (string): Either "prefix" or "fuzzy"
$env.config.completions.algorithm = "prefix"

# sort (string): One of "smart" or "alphabetical"
# In "smart" mode sort order is based on the "algorithm" setting.
# When using the "prefix" algorithm, results are alphabetically sorted.
# When using the "fuzzy" algoritm, results are sorted based on their fuzzy score.
$env.config.completions.sort = "smart"

# case_sensitive (bool): true/false to enable/disable case-sensitive completions
$env.config.completions.case_sensitive = false 

# quick (bool):
# true: auto-select the completion when only one remains
# false: prevents auto-select of the final result
$env.config.completions.quick = true

# partial (bool):
# true: Partially complete up to the best possible match
# false: Do not partially complete
# Partial Example: If a directory contains only files named "forage", "food", and "forest",
#                  then typing "ls " and pressing <Tab> will partially complete "fo". If
#                  the directory also includes a file named "faster", then only "f" would
#                  be partially completed.
$env.config.completions.partial = true

# use_ls_colors (bool): When true, apply LS_COLORS to file/path/directory matches
$env.config.completions.use_ls_colors = true

# --------------------
# External Completions
# --------------------
# completions.external.*: Settings related to completing external commands
# and additional completers

# external.exnable (bool)
# true: search for external commands on the Path
# false: disabling might be desired for performance if your path includes
#        directories on a slower filesystem
$env.config.completions.external.enable = true

# max_results (int): Limit the number of external commands retrieved from
# path to this value. Has no effect if `...external.enable` (above) is set to `false`
$env.config.completions.external.max_results = 50

# completer (closure with a |spans| parameter): A command to call for *argument* completions
# to commands (internal or external). 
#
# The |spans| parameter is a list of strings representing the tokens (spans)
# on the current commandline. It is always a list of at least two strings - The 
# command being completed plus the first argument of that command ("" if no argument has
# been partially typed yet), and additional strings for additional arguments beyond
# the first.
#
# This setting is usually set to a closure which will call a third-party completion system, such
# as Carapace. 
#
# Note: The following is an over-simplified completer command that will call Carapace if it
# is installed. Please use the official Carapace completer, which can be generated automatically
# by Carapace itself. See the Carapace documentation for the proper syntax.
$env.config.completions.external.completer = {|spans|
  carapace $spans.0 nushell ...$spans | from json
}

# --------------------
# Terminal Integration
# --------------------
# Nushell can output a number of escape codes to enable advanced features in Terminal Emulators
# that support them. Settings in this section enable or disable these features in Nushell.
# Features aren't supported by your Terminal can be disabled. Features can also be disabled,
#  of course, if there is a conflict between the Nushell and Terminal's implementation.

# use_kitty_protocol (bool):
# A keyboard enhancement protocol supported by the Kitty Terminal. Additional keybindings are
# available when using this protocol in a supported terminal. For example, without this protocol,
# Ctrl+I is interpreted as the Tab Key. With this protocol, Ctrl+I and Tab can be mapped separately.
$env.config.use_kitty_protocol = false

# osc2 (bool):
# When true, the current directory and running command are shown in the terminal tab/window title.
# Also abbreviates the directory name by prepending ~ to the home directory and its subdirectories.
$env.config.shell_integration.osc2 = true

# osc7 (bool):
# Nushell will report the current directory to the terminal using OSC 7. This is useful when
# spawning new tabs in the same directory.
$env.config.shell_integration.osc7 = true

# osc9_9 (bool):
# Enables/Disables OSC 9;9 support, originally a ConEmu terminal feature. This is an
# alternative to OSC 7 which also communicates the current path to the terminal.
$env.config.shell_integration.osc9_9 = false

# osc8 (bool):
# When true, the `ls` command will generate clickable links that can be launched in another
# application by the terminal.
# Note: This setting replaces the now deprecated `ls.show_clickable_links`
$env.config.shell.integration.osc8: true

# Deprecated
# $env.config.ls.clickable_links = true

# osc133 (bool):
# true/false to enable/disable OSC 133 support, a set of several escape sequences which
# report the (1) starting location of the prompt, (2) ending location of the prompt,
# (3) starting location of the command output, and (4) the exit code of the command.

# originating with Final Term. These sequences report information regarding the prompt
# location as well as command status to the terminal. This enables advanced features in
# some terminals, including the ability to provide separate background colors for the
# command vs. the output, collapsible output, or keybindings to scroll between prompts.
$env.config.shell_integration.osc133 = true

# osc633 (bool):
# true/false to enable/disable OSC 633, an extension to OSC 133 for Visual Studio Code
$env.config.shell_integration.osc633 = true

# reset_application_mode (bool):
# true/false to enable/disable sending ESC[?1l to the terminal
# This sequence is commonly used to keep cursor key modes in sync between the local 
# terminal and a remove SSH host.
$env.config.shell_integration.reset_application_mode = true

# bracketed_paste (bool):
# true/false to enable/disable the bracketed-paste feature, which allows mutiple-lines
# to be pasted into Nushell at once without immediate execution. When disabled, 
# each pasted line is executed as it is received.
# Note that bracketed paste is not currently supported on the Windows version of
# Nushell.
$env.config.bracketed_paste = true

# use_ansi_coloring (bool):
# true/false to enable/disable the use of ANSI colors in Nushell internal commands.
# When disabled, output from Nushell built-in commands will display only in the default
# foreground color.
# Note: Does not apply to the `ansi` command.
$env.config.use_ansi_coloring = true

# ----------------------
# Error Display Settings
# ----------------------

# error_style (string): One of "fancy" or "plain"
# Plain: Display plain-text errors for screen-readers
# Fancy: Display errors using line-drawing characters to point to the span in which the
#        problem occurred.
$env.config.error_style = "fancy"

# display_errors.exit_code (bool):
# true: Display a Nushell error when an external command returns a non-zero exit code
# false: Display only the error information printed by the external command itself
# Note: Core dump errors are always printed; SIGPIPE never triggers an error
$env.config.display_errors.exit_code = false

# display_errors.termination_signal (bool):
# true/false to enable/disable displaying a Nushell error when a child process is
# terminated via any signal 
$env.config.display_errors.termination_signal = true

# -------------
# Table Display
# -------------
# footer_mode (string or int):
# Specifies when to display table footers with column names. Allowed values:
# "always"
# "never"
# "auto": When the length of the table would scroll the header past the first line of the terminal
# (int): When the number of table rows meets or exceeds this value
$env.config.footer_mode = 25

# table.*
# table_mode (string): 
# One of: "default", "basic", "compact", "compact_double", "heavy", "light", "none", "reinforced",
# "rounded", "thin", "with_love", "psql", "markdown", "dots", "restructured", "ascii_rounded",
# or "basic_compact"
$env.config.table.mode = "default"
$env.config.table.index_mode
$env.config.table.show_empty
$env.config.table.padding
$env.config.table.padding.left
$env.config.table.padding.right
$env.config.table.trim
$env.config.table.trim.methodology
$env.config.table.trim.wrapping_try_keep_words
$env.config.table.header_on_separator
$env.config.table.abbreviated_row_count
$env.config.table.footer_inheritance

table: {
    index_mode: always # "always" show indexes, "never" show indexes, "auto" = show indexes when a table has "index" column
    show_empty: true # show 'empty list' and 'empty record' placeholders for command output
    padding: { left: 1, right: 1 } # a left right padding of each column in a table
    trim: {
        methodology: wrapping # wrapping or truncating
        wrapping_try_keep_words: true # A strategy used by the 'wrapping' methodology
        truncating_suffix: "..." # A suffix used by the 'truncating' methodology
    }
    header_on_separator: false # show header text on separator/border line
    footer_inheritance: false # render footer in parent table if child is big enough (extended table option)
    # abbreviated_row_count: 10 # limit data rows from top and bottom after reaching a set point
}

$env.config.float_precision
    float_precision: 2 # the precision for displaying floats in tables

# ----------------
# Datetime Display
# ----------------
$env.config.datetime_format
$env.config.datetime_format.normal
$env.config.datetime_format.table
    # datetime_format determines what a datetime rendered in the shell would look like.
    # Behavior without this configuration point will be to "humanize" the datetime display,
    # showing something like "a day ago."
    datetime_format: {
        # normal: '%a, %d %b %Y %H:%M:%S %z'    # shows up in displays of variables or other datetime's outside of tables
        # table: '%m/%d/%y %I:%M:%S%p'          # generally shows up in tabular outputs such as ls. commenting this out will change it to the default human readable datetime format
    }

# ----------------
# Filesize Display
# ----------------
$env.config.filesize
$env.config.filesize.metric
$env.config.filesize.format
    filesize: {
        metric: false # true => KB, MB, GB (ISO standard), false => KiB, MiB, GiB (Windows standard)
        format: "auto" # b, kb, kib, mb, mib, gb, gib, tb, tib, pb, pib, eb, eib, auto
    }

# ---------------------
# Miscellaneous Display
# ---------------------

# render_right_prompt_on_last_line(bool):
# true: When using a multi-line left-prompt, the right-prompt will be displayed on the last line
# false: The right-prompt is displayed on the first line of the left-prompt
$env.config.render_right_prompt_on_last_line = false


$env.config.ls
$env.config.ls.use_ls_colors
    ls: {
        use_ls_colors: true # use the LS_COLORS environment variable to colorize output
    }

# $env.config.explore
# -------------------
$env.config.explore.status_bar_background
$env.config.explore.command_bar_text
$env.config.explore.highlight
$env.config.explore.selected_cell
$env.config.explore.status
$env.config.explore.status.error
$env.config.explore.status.warn
$env.config.explore.status.info
        status_bar_background: { fg: "#1D1F21", bg: "#C4C9C6" },
        command_bar_text: { fg: "#C4C9C6" },
        highlight: { fg: "black", bg: "yellow" },
        status: {
            error: { fg: "white", bg: "red" },
            warn: {}
            info: {}
        },
        selected_cell: { bg: light_blue },

# Per-plugin configuration. See https://www.nushell.sh/contributor-book/plugins.html#configuration.
plugins: {}
$env.config.plugins
$env.config.plugin_gc
$env.config.plugin_gc.default
$env.config.plugin_gc.default.enabled
$env.config.plugin_gc.default.stop_after
$env.config.plugin_gc.plugins

$env.config.keybindings
$env.config.menus

# Hooks
# -----
# $env.config.hooks is a record containing # the five different types of Nushell hooks.
# See the Hooks documentation at https://www.nushell.sh/book/hooks for details
#
# Most hooks can accept a string, a closure, or a list containing strings and/or closures.
# The display_output record can only accept a string or a closure, but never a list
#
# WARNING: A malformed display_output hook can suppress all Nushell output to the terminal.
#          It can be reset by assigning an empty string as below:

$env.config.hooks.pre_prompt = []          # Before each prompt is displayed
$env.config.hooks.pre_execution = []       # After <enter> is pressed; before the command is executed
$env.config.hooks.env_change = []          # When a specified environment variable changes
$env.config.hooks.display_output = ""      # Before Nushell output is displayed in the terminal
$env.config.hooks.command_not_found = []   # When a command is not found



$env.config.highlight_resolved_externals

$env.config.color_config
$env.config.color_config.shape_filepath
$env.config.color_config.shape_operator
$env.config.color_config.shape_literal
$env.config.color_config.shape_garbage
$env.config.color_config.shape_garbage.fg
$env.config.color_config.shape_garbage.bg
$env.config.color_config.shape_garbage.attr
$env.config.color_config.shape_datetime
$env.config.color_config.shape_datetime.fg
$env.config.color_config.shape_datetime.attr
$env.config.color_config.shape_or
$env.config.color_config.shape_or.fg
$env.config.color_config.shape_or.attr
$env.config.color_config.float
$env.config.color_config.shape_string_interpolation
$env.config.color_config.shape_string_interpolation.fg
$env.config.color_config.shape_string_interpolation.attr
$env.config.color_config.shape_and
$env.config.color_config.shape_and.fg
$env.config.color_config.shape_and.attr
$env.config.color_config.closure
$env.config.color_config.block
$env.config.color_config.shape_list
$env.config.color_config.shape_list.fg
$env.config.color_config.shape_list.attr
$env.config.color_config.hints
$env.config.color_config.shape_table
$env.config.color_config.shape_table.fg
$env.config.color_config.shape_table.attr
$env.config.color_config.shape_variable
$env.config.color_config.shape_nothing
$env.config.color_config.shape_glob_interpolation
$env.config.color_config.shape_glob_interpolation.fg
$env.config.color_config.shape_glob_interpolation.attr
$env.config.color_config.glob
$env.config.color_config.shape_raw_string
$env.config.color_config.shape_raw_string.fg
$env.config.color_config.shape_raw_string.attr
$env.config.color_config.shape_record
$env.config.color_config.shape_record.fg
$env.config.color_config.shape_record.attr
$env.config.color_config.shape_bool
$env.config.color_config.binary
$env.config.color_config.foreground
$env.config.color_config.shape_string
$env.config.color_config.shape_int
$env.config.color_config.shape_int.fg
$env.config.color_config.shape_int.attr
$env.config.color_config.custom
$env.config.color_config.leading_trailing_space_bg
$env.config.color_config.leading_trailing_space_bg.attr
$env.config.color_config.cursor
$env.config.color_config.nothing
$env.config.color_config.shape_match_pattern
$env.config.color_config.search_result
$env.config.color_config.search_result.fg
$env.config.color_config.search_result.bg
$env.config.color_config.shape_custom
$env.config.color_config.shape_externalarg
$env.config.color_config.shape_externalarg.fg
$env.config.color_config.shape_externalarg.attr
$env.config.color_config.int
$env.config.color_config.date
$env.config.color_config.shape_globpattern
$env.config.color_config.shape_globpattern.fg
$env.config.color_config.shape_globpattern.attr
$env.config.color_config.shape_pipe
$env.config.color_config.shape_pipe.fg
$env.config.color_config.shape_pipe.attr
$env.config.color_config.header
$env.config.color_config.shape_block
$env.config.color_config.shape_block.fg
$env.config.color_config.shape_block.attr
$env.config.color_config.shape_signature
$env.config.color_config.shape_signature.fg
$env.config.color_config.shape_signature.attr
$env.config.color_config.shape_keyword
$env.config.color_config.shape_keyword.fg
$env.config.color_config.shape_keyword.attr
$env.config.color_config.shape_directory
$env.config.color_config.shape_closure
$env.config.color_config.shape_closure.fg
$env.config.color_config.shape_closure.attr
$env.config.color_config.shape_internalcall
$env.config.color_config.shape_internalcall.fg
$env.config.color_config.shape_internalcall.attr
$env.config.color_config.shape_float
$env.config.color_config.shape_float.fg
$env.config.color_config.shape_float.attr
$env.config.color_config.filesize
$env.config.color_config.bool
$env.config.color_config.separator
$env.config.color_config.list
$env.config.color_config.range
$env.config.color_config.shape_external_resolved
$env.config.color_config.shape_range
$env.config.color_config.shape_range.fg
$env.config.color_config.shape_range.attr
$env.config.color_config.shape_vardecl
$env.config.color_config.shape_vardecl.fg
$env.config.color_config.shape_vardecl.attr
$env.config.color_config.duration
$env.config.color_config.background
$env.config.color_config.shape_redirection
$env.config.color_config.shape_redirection.fg
$env.config.color_config.shape_redirection.attr
$env.config.color_config.shape_matching_brackets
$env.config.color_config.shape_matching_brackets.attr
$env.config.color_config.row_index
$env.config.color_config.record
$env.config.color_config.cell-path
$env.config.color_config.shape_binary
$env.config.color_config.shape_binary.fg
$env.config.color_config.shape_binary.attr
$env.config.color_config.shape_external
$env.config.color_config.string
$env.config.color_config.shape_flag
$env.config.color_config.shape_flag.fg
$env.config.color_config.shape_flag.attr
$env.config.color_config.empty


# For more information on defining custom themes, see
# https://www.nushell.sh/book/coloring_and_theming.html
# And here is the theme collection
# https://github.com/nushell/nu_scripts/tree/main/themes
    leading_trailing_space_bg: { attr: n } # no fg, no bg, attr none effectively turns this off
    header: green_bold
    empty: blue
    # Closures can be used to choose colors for specific values.
    # The value (in this case, a bool) is piped into the closure.
    # eg) {|| if $in { 'light_cyan' } else { 'light_gray' } }
    bool: light_cyan
    int: white
    filesize: cyan
    duration: white
    date: purple
    range: white
    float: white
    string: white
    nothing: white
    binary: white
    cell-path: white
    row_index: green_bold
    record: white
    list: white
    block: white
    hints: dark_gray
    search_result: { bg: red fg: white }
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
    shape_raw_string: light_purple
}



# The default config record. This is where much of your global configuration is setup.
$env.config = {






    explore: {
        status_bar_background: { fg: "#1D1F21", bg: "#C4C9C6" },
        command_bar_text: { fg: "#C4C9C6" },
        highlight: { fg: "black", bg: "yellow" },
        status: {
            error: { fg: "white", bg: "red" },
            warn: {}
            info: {}
        },
        selected_cell: { bg: light_blue },
    }





    color_config: $dark_theme # if you want a more interesting theme, you can replace the empty record with `$dark_theme`, `$light_theme` or another custom record
    }
    highlight_resolved_externals: false # true enables highlighting of external commands in the repl resolved by which.


    plugin_gc: {
        # Configuration for plugin garbage collection
        default: {
            enabled: true # true to enable stopping of inactive plugins
            stop_after: 10sec # how long to wait after a plugin is inactive to stop it
        }
        plugins: {
            # alternate configuration for specific plugins, by name, for example:
            #
            # gstat: {
            #     enabled: false
            # }
        }
    }

    hooks: {
        pre_prompt: [{ null }] # run before the prompt is shown
        pre_execution: [{ null }] # run before the repl input is run
        env_change: {
            PWD: [{|before, after| null }] # run if the PWD environment is different since the last repl input
        }
        display_output: "if (term size).columns >= 100 { table -e } else { table }" # run to display the output of a pipeline
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
                col_width: 20     # Optional value. If missing all the screen width is used to calculate column width
                col_padding: 2
            }
            style: {
                text: green
                selected_text: { attr: r }
                description_text: yellow
                match_text: { attr: u }
                selected_match_text: { attr: ur }
            }
        }
        {
            name: ide_completion_menu
            only_buffer_difference: false
            marker: "| "
            type: {
                layout: ide
                min_completion_width: 0,
                max_completion_width: 50,
                max_completion_height: 10, # will be limited by the available lines in the terminal
                padding: 0,
                border: true,
                cursor_offset: 0,
                description_mode: "prefer_right"
                min_description_width: 0
                max_description_width: 50
                max_description_height: 10
                description_offset: 1
                # If true, the cursor pos will be corrected, so the suggestions match up with the typed text
                #
                # C:\> str
                #      str join
                #      str trim
                #      str split
                correct_cursor_pos: false
            }
            style: {
                text: green
                selected_text: { attr: r }
                description_text: yellow
                match_text: { attr: u }
                selected_match_text: { attr: ur }
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
                col_width: 20     # Optional value. If missing all the screen width is used to calculate column width
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
                    { edit: complete }
                ]
            }
        }
        {
            name: completion_previous_menu
            modifier: shift
            keycode: backtab
            mode: [emacs, vi_normal, vi_insert]
            event: { send: menuprevious }
        }
        {
            name: ide_completion_menu
            modifier: control
            keycode: space
            mode: [emacs vi_normal vi_insert]
            event: {
                until: [
                    { send: menu name: ide_completion_menu }
                    { send: menunext }
                    { edit: complete }
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
            modifier: none
            keycode: f1
            mode: [emacs, vi_insert, vi_normal]
            event: { send: menu name: help_menu }
        }
        {
            name: next_page_menu
            modifier: control
            keycode: char_x
            mode: emacs
            event: { send: menupagenext }
        }
        {
            name: undo_or_previous_page_menu
            modifier: control
            keycode: char_z
            mode: emacs
            event: {
                until: [
                    { send: menupageprevious }
                    { edit: undo }
                ]
            }
        }
        {
            name: escape
            modifier: none
            keycode: escape
            mode: [emacs, vi_normal, vi_insert]
            event: { send: esc }    # NOTE: does not appear to work
        }
        {
            name: cancel_command
            modifier: control
            keycode: char_c
            mode: [emacs, vi_normal, vi_insert]
            event: { send: ctrlc }
        }
        {
            name: quit_shell
            modifier: control
            keycode: char_d
            mode: [emacs, vi_normal, vi_insert]
            event: { send: ctrld }
        }
        {
            name: clear_screen
            modifier: control
            keycode: char_l
            mode: [emacs, vi_normal, vi_insert]
            event: { send: clearscreen }
        }
        {
            name: search_history
            modifier: control
            keycode: char_q
            mode: [emacs, vi_normal, vi_insert]
            event: { send: searchhistory }
        }
        {
            name: open_command_editor
            modifier: control
            keycode: char_o
            mode: [emacs, vi_normal, vi_insert]
            event: { send: openeditor }
        }
        {
            name: move_up
            modifier: none
            keycode: up
            mode: [emacs, vi_normal, vi_insert]
            event: {
                until: [
                    { send: menuup }
                    { send: up }
                ]
            }
        }
        {
            name: move_down
            modifier: none
            keycode: down
            mode: [emacs, vi_normal, vi_insert]
            event: {
                until: [
                    { send: menudown }
                    { send: down }
                ]
            }
        }
        {
            name: move_left
            modifier: none
            keycode: left
            mode: [emacs, vi_normal, vi_insert]
            event: {
                until: [
                    { send: menuleft }
                    { send: left }
                ]
            }
        }
        {
            name: move_right_or_take_history_hint
            modifier: none
            keycode: right
            mode: [emacs, vi_normal, vi_insert]
            event: {
                until: [
                    { send: historyhintcomplete }
                    { send: menuright }
                    { send: right }
                ]
            }
        }
        {
            name: move_one_word_left
            modifier: control
            keycode: left
            mode: [emacs, vi_normal, vi_insert]
            event: { edit: movewordleft }
        }
        {
            name: move_one_word_right_or_take_history_hint
            modifier: control
            keycode: right
            mode: [emacs, vi_normal, vi_insert]
            event: {
                until: [
                    { send: historyhintwordcomplete }
                    { edit: movewordright }
                ]
            }
        }
        {
            name: move_to_line_start
            modifier: none
            keycode: home
            mode: [emacs, vi_normal, vi_insert]
            event: { edit: movetolinestart }
        }
        {
            name: move_to_line_start
            modifier: control
            keycode: char_a
            mode: [emacs, vi_normal, vi_insert]
            event: { edit: movetolinestart }
        }
        {
            name: move_to_line_end_or_take_history_hint
            modifier: none
            keycode: end
            mode: [emacs, vi_normal, vi_insert]
            event: {
                until: [
                    { send: historyhintcomplete }
                    { edit: movetolineend }
                ]
            }
        }
        {
            name: move_to_line_end_or_take_history_hint
            modifier: control
            keycode: char_e
            mode: [emacs, vi_normal, vi_insert]
            event: {
                until: [
                    { send: historyhintcomplete }
                    { edit: movetolineend }
                ]
            }
        }
        {
            name: move_to_line_start
            modifier: control
            keycode: home
            mode: [emacs, vi_normal, vi_insert]
            event: { edit: movetolinestart }
        }
        {
            name: move_to_line_end
            modifier: control
            keycode: end
            mode: [emacs, vi_normal, vi_insert]
            event: { edit: movetolineend }
        }
        {
            name: move_down
            modifier: control
            keycode: char_n
            mode: [emacs, vi_normal, vi_insert]
            event: {
                until: [
                    { send: menudown }
                    { send: down }
                ]
            }
        }
        {
            name: move_up
            modifier: control
            keycode: char_p
            mode: [emacs, vi_normal, vi_insert]
            event: {
                until: [
                    { send: menuup }
                    { send: up }
                ]
            }
        }
        {
            name: delete_one_character_backward
            modifier: none
            keycode: backspace
            mode: [emacs, vi_insert]
            event: { edit: backspace }
        }
        {
            name: delete_one_word_backward
            modifier: control
            keycode: backspace
            mode: [emacs, vi_insert]
            event: { edit: backspaceword }
        }
        {
            name: delete_one_character_forward
            modifier: none
            keycode: delete
            mode: [emacs, vi_insert]
            event: { edit: delete }
        }
        {
            name: delete_one_character_forward
            modifier: control
            keycode: delete
            mode: [emacs, vi_insert]
            event: { edit: delete }
        }
        {
            name: delete_one_character_backward
            modifier: control
            keycode: char_h
            mode: [emacs, vi_insert]
            event: { edit: backspace }
        }
        {
            name: delete_one_word_backward
            modifier: control
            keycode: char_w
            mode: [emacs, vi_insert]
            event: { edit: backspaceword }
        }
        {
            name: move_left
            modifier: none
            keycode: backspace
            mode: vi_normal
            event: { edit: moveleft }
        }
        {
            name: newline_or_run_command
            modifier: none
            keycode: enter
            mode: emacs
            event: { send: enter }
        }
        {
            name: move_left
            modifier: control
            keycode: char_b
            mode: emacs
            event: {
                until: [
                    { send: menuleft }
                    { send: left }
                ]
            }
        }
        {
            name: move_right_or_take_history_hint
            modifier: control
            keycode: char_f
            mode: emacs
            event: {
                until: [
                    { send: historyhintcomplete }
                    { send: menuright }
                    { send: right }
                ]
            }
        }
        {
            name: redo_change
            modifier: control
            keycode: char_g
            mode: emacs
            event: { edit: redo }
        }
        {
            name: undo_change
            modifier: control
            keycode: char_z
            mode: emacs
            event: { edit: undo }
        }
        {
            name: paste_before
            modifier: control
            keycode: char_y
            mode: emacs
            event: { edit: pastecutbufferbefore }
        }
        {
            name: cut_word_left
            modifier: control
            keycode: char_w
            mode: emacs
            event: { edit: cutwordleft }
        }
        {
            name: cut_line_to_end
            modifier: control
            keycode: char_k
            mode: emacs
            event: { edit: cuttolineend }
        }
        {
            name: cut_line_from_start
            modifier: control
            keycode: char_u
            mode: emacs
            event: { edit: cutfromstart }
        }
        {
            name: swap_graphemes
            modifier: control
            keycode: char_t
            mode: emacs
            event: { edit: swapgraphemes }
        }
        {
            name: move_one_word_left
            modifier: alt
            keycode: left
            mode: emacs
            event: { edit: movewordleft }
        }
        {
            name: move_one_word_right_or_take_history_hint
            modifier: alt
            keycode: right
            mode: emacs
            event: {
                until: [
                    { send: historyhintwordcomplete }
                    { edit: movewordright }
                ]
            }
        }
        {
            name: move_one_word_left
            modifier: alt
            keycode: char_b
            mode: emacs
            event: { edit: movewordleft }
        }
        {
            name: move_one_word_right_or_take_history_hint
            modifier: alt
            keycode: char_f
            mode: emacs
            event: {
                until: [
                    { send: historyhintwordcomplete }
                    { edit: movewordright }
                ]
            }
        }
        {
            name: delete_one_word_forward
            modifier: alt
            keycode: delete
            mode: emacs
            event: { edit: deleteword }
        }
        {
            name: delete_one_word_backward
            modifier: alt
            keycode: backspace
            mode: emacs
            event: { edit: backspaceword }
        }
        {
            name: delete_one_word_backward
            modifier: alt
            keycode: char_m
            mode: emacs
            event: { edit: backspaceword }
        }
        {
            name: cut_word_to_right
            modifier: alt
            keycode: char_d
            mode: emacs
            event: { edit: cutwordright }
        }
        {
            name: upper_case_word
            modifier: alt
            keycode: char_u
            mode: emacs
            event: { edit: uppercaseword }
        }
        {
            name: lower_case_word
            modifier: alt
            keycode: char_l
            mode: emacs
            event: { edit: lowercaseword }
        }
        {
            name: capitalize_char
            modifier: alt
            keycode: char_c
            mode: emacs
            event: { edit: capitalizechar }
        }
        # The following bindings with `*system` events require that Nushell has
        # been compiled with the `system-clipboard` feature.
        # If you want to use the system clipboard for visual selection or to
        # paste directly, uncomment the respective lines and replace the version
        # using the internal clipboard.
        {
            name: copy_selection
            modifier: control_shift
            keycode: char_c
            mode: emacs
            event: { edit: copyselection }
            # event: { edit: copyselectionsystem }
        }
        {
            name: cut_selection
            modifier: control_shift
            keycode: char_x
            mode: emacs
            event: { edit: cutselection }
            # event: { edit: cutselectionsystem }
        }
        # {
        #     name: paste_system
        #     modifier: control_shift
        #     keycode: char_v
        #     mode: emacs
        #     event: { edit: pastesystem }
        # }
        {
            name: select_all
            modifier: control_shift
            keycode: char_a
            mode: emacs
            event: { edit: selectall }
        }
    ]
}
