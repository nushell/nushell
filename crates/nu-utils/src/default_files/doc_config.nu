# Nushell Config File Documentation
#
# Warning: This file is intended for documentation purposes only and
# is not intended to be used as an actual configuration file as-is.
#
# version = "0.107.1"
#
# A `config.nu` file is used to override default Nushell settings,
# define (or import) custom commands, or run any other startup tasks.
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
# config nu --doc | nu-highlight | less -R

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
# After exceeding this value, the oldest history items will be removed
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

# show_banner (bool|string): Enable or disable the welcome banner at startup
# true | "full": show the full banner
# "short": just show the start-up time
# false | "none": don't show a banner
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
$env.config.edit_mode = "emacs"

# Command that will be used to edit the current line buffer with Ctrl+O.
# If unset, uses $env.VISUAL and then $env.EDITOR
#
# Tip: Set to "editor" to use the default editor on Unix platforms using
#      the Alternatives system or equivalent
$env.config.buffer_editor = "editor"
# To set arguments for the editor, a list can be used:
$env.config.buffer_editor = ["emacsclient", "-s", "light", "-t"]

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

# algorithm (string): "prefix", "substring" or "fuzzy"
$env.config.completions.algorithm = "prefix"

# sort (string): One of "smart" or "alphabetical"
# In "smart" mode sort order is based on the "algorithm" setting.
# When using the "prefix" algorithm, results are alphabetically sorted.
# When using the "substring" algorithm, results are alphabetically sorted.
# When using the "fuzzy" algorithm, results are sorted based on their fuzzy score.
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
#                  then typing "ls " and pressing <Tab> will partially complete the first two
#                  letters, "f" and "o". If the directory also includes a file named "faster",
#                  then only "f" would be partially completed.
$env.config.completions.partial = true

# use_ls_colors (bool): When true, apply LS_COLORS to file/path/directory matches
$env.config.completions.use_ls_colors = true

# --------------------
# External Completions
# --------------------
# completions.external.*: Settings related to completing external commands
# and additional completers

# external.enable (bool)
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
$env.config.shell_integration.osc7 = ($nu.os-info.name != windows)

# osc9_9 (bool):
# Enables/Disables OSC 9;9 support, originally a ConEmu terminal feature. This is an
# alternative to OSC 7 which also communicates the current path to the terminal.
$env.config.shell_integration.osc9_9 = ($nu.os-info.name == windows)

# osc8 (bool):
# When true, the `ls` command will generate clickable links that can be launched in another
# application by the terminal.
# Note: This setting replaces the now deprecated `ls.clickable_links`
$env.config.shell_integration.osc8 = true

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
# terminal and a remote SSH host.
$env.config.shell_integration.reset_application_mode = true

# bracketed_paste (bool):
# true/false to enable/disable the bracketed-paste feature, which allows multiple-lines
# to be pasted into Nushell at once without immediate execution. When disabled,
# each pasted line is executed as it is received.
# Note that bracketed paste is not currently supported on the Windows version of
# Nushell.
$env.config.bracketed_paste = true

# use_ansi_coloring ("auto" or bool):
# The default value `"auto"` dynamically determines if ANSI coloring is used.
# It evaluates the following environment variables in decreasingly priority:
# `FORCE_COLOR`, `NO_COLOR`, and `CLICOLOR`.
# - If `FORCE_COLOR` is set, coloring is always enabled.
# - If `NO_COLOR` is set, coloring is disabled.
# - If `CLICOLOR` is set, its value (0 or 1) decides whether coloring is used.
# If none of these are set, it checks whether the standard output is a terminal
# and enables coloring if it is.
# A value of `true` or `false` overrides this behavior, explicitly enabling or
# disabling ANSI coloring in Nushell's internal commands.
# When disabled, built-in commands will only use the default foreground color.
# Note: This setting does not affect the `ansi` command.
$env.config.use_ansi_coloring = "auto"

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
# Note: Does not take into account rows with multiple lines themselves
$env.config.footer_mode = 25

# table.*
# mode (string):
# Specifies the visual display style of a table
# One of: "default", "basic", "compact", "compact_double", "heavy", "light", "none", "reinforced",
# "rounded", "thin", "with_love", "psql", "markdown", "dots", "restructured", "ascii_rounded",
# "basic_compact", "single", or "double"
# Can be overridden by passing a table to `| table --theme/-t`
$env.config.table.mode = "default"

# index_mode (string) - One of:
# "never": never show the index column in a table or list
# "always": always show the index column in tables and lists
# "auto": show the column only when there is an explicit "index" column in the table
# Can be overridden by passing a table to `| table --index/-i`
$env.config.table.index_mode = "always"

# show_empty (bool):
# true: show "empty list" or "empty table" when no values exist
# false: display no output when no values exist
$env.config.table.show_empty = true

# padding.left/right (int): The number of spaces to pad around values in each column
$env.config.table.padding.left = 1
$env.config.table.padding.right = 1

# trim.*: The rules that will be used to display content in a table row when it would cause the
#         table to exceed the terminal width.
# methodology (string): One of "wrapping" or "truncating"
# truncating_suffix (string): The text to show at the end of the row to indicate that it has
#                             been truncated. Only valid when `methodology = "truncating"`.
# wrapping_try_keep_words (bool): true to keep words together based on whitespace
#                                 false to allow wrapping in the middle of a word.
#                                 Only valid when `methodology = wrapping`.
$env.config.table.trim = {
  methodology: "wrapping"
  wrapping_try_keep_words: true
}
# or
$env.config.table.trim = {
  methodology: "truncating"
  truncating_suffix: "..."
}

# header_on_separator (bool):
# true: Displays the column headers as part of the top (or bottom) border of the table
# false: Displays the column header in its own row with a separator below.
$env.config.table.header_on_separator = false

# abbreviated_row_count (int or nothing):
# If set to an int, all tables will be abbreviated to only show the first <n> and last <n> rows
# If set to `null`, all table rows will be displayed
# Can be overridden by passing a table to `| table --abbreviated/-a`
$env.config.table.abbreviated_row_count = null

# footer_inheritance (bool): Footer behavior in nested tables
# true: If a nested table is long enough on its own to display a footer (per `footer_mode` above),
#       then also display the footer for the parent table
# false: Always apply `footer_mode` rules to the parent table
$env.config.table.footer_inheritance = false

# missing_value_symbol (string): The symbol shown for missing values
$env.config.table.missing_value_symbol = "❎"

# ----------------
# Datetime Display
# ----------------
# datetime_format.* (string or nothing):
# Format strings that will be used for datetime values.
# When set to `null`, the default behavior is to "humanize" the value (e.g., "now" or "a day ago")

# datetime_format.table (string or nothing):
# The format string (or `null`) that will be used to display a datetime value when it appears in a
# structured value such as a table, list, or record.
# Execute `into datetime --list` to get a list of supported datetime specifiers.
$env.config.datetime_format.table = null

# datetime_format.normal (string or nothing):
# The format string (or `null`) that will be used to display a datetime value when it appears as
# a raw value.
$env.config.datetime_format.normal = "%m/%d/%y %I:%M:%S%p"

# ----------------
# Filesize Display
# ----------------
# filesize.unit (string): One of either:
# - A filesize unit: "B", "kB", "KiB", "MB", "MiB", "GB", "GiB", "TB", "TiB", "PB", "PiB", "EB", or "EiB".
# - An automatically scaled unit: "metric" or "binary".
# "metric" will use units with metric (SI) prefixes like kB, MB, or GB.
# "binary" will use units with binary prefixes like KiB, MiB, or GiB.
# Otherwise, setting this to one of the filesize units will use that particular unit when displaying all file sizes.
$env.config.filesize.unit = 'metric'

# filesize.show_unit (bool):
# Whether to show or hide the file size unit. Useful if `$env.config.filesize.unit` is set to a fixed unit,
# and you don't want that same unit repeated over and over again in which case you can set this to `false`.
$env.config.filesize.show_unit = true

# filesize.precision (int or nothing):
# The number of digits to display after the decimal point for file sizes.
# When set to `null`, all digits after the decimal point, if any, will be displayed.
$env.config.filesize.precision = 1

# ---------------------
# Miscellaneous Display
# ---------------------

# render_right_prompt_on_last_line(bool):
# true: When using a multi-line left-prompt, the right-prompt will be displayed on the last line
# false: The right-prompt is displayed on the first line of the left-prompt
$env.config.render_right_prompt_on_last_line = false

# float_precision (int):
# Float values will be rounded to this precision when displaying in structured values such as lists,
# tables, or records.
$env.config.float_precision = 2

# ls.use_ls_colors (bool):
# true: The `ls` command will apply the $env.LS_COLORS standard to filenames
# false: Filenames in the `ls` table will use the color_config for strings
$env.config.ls.use_ls_colors = true

# Hooks
# -----
# $env.config.hooks is a record containing the five different types of Nushell hooks.
# See the Hooks documentation at https://www.nushell.sh/book/hooks for details
#
# Most hooks can accept a string, a closure, or a list containing strings and/or closures.
# The display_output record can only accept a string or a closure, but never a list
#
# WARNING: A malformed display_output hook can suppress all Nushell output to the terminal.
#          It can be reset by assigning an empty string as below:

# Before each prompt is displayed
$env.config.hooks.pre_prompt = []
# After <enter> is pressed; before the commandline is executed
$env.config.hooks.pre_execution = []
# When a specified environment variable changes
$env.config.hooks.env_change = {
    # Example: Run if the PWD environment is different since the last REPL input
    PWD: [{|before, after| null }]
}
# Before Nushell output is displayed in the terminal
$env.config.hooks.display_output = "if (term size).columns >= 100 { table -e } else { table }"
# When a command is not found
$env.config.hooks.command_not_found = []

# The env_change hook accepts a record with environment variable names as keys, and a list
# of hooks to run when that variable changes
$env.config.hooks.env_change = {}

# -----------
# Keybindings
# -----------
# keybindings (list): A list of user-defined keybindings
# Nushell/Reedline keybindings can be added or overridden using this setting.
# See https://www.nushell.sh/book/line_editor.html#keybindings for details.
#
# Example - Add a new Alt+. keybinding to insert the last token used on the previous commandline
$env.config.keybindings ++= [
  {
    name: insert_last_token
    modifier: alt
    keycode: char_.
    mode: [emacs vi_normal vi_insert]
    event: [
      { edit: InsertString, value: "!$" }
      { send: Enter }
    ]
  }
]

# Example: Override the F1 keybinding with a user-defined help menu (see "Menus" below):
$env.config.keybindings ++= [
  {
    name: help_menu
    modifier: none
    keycode: f1
    mode: [emacs, vi_insert, vi_normal]
    event: { send: menu name: help_menu }
  }
]

# -----
# Menus
# -----
# menus (list):
#
# Nushell/Reedline menus can be created and modified using this setting.
# See https://www.nushell.sh/book/line_editor.html#menus for details.
#
# Note that menus are usually activated via keybindings, which are defined in
# $env.config.keybindings (above).
#
# Simple example - Add a new Help menu to the list (note that a similar menu is already
# defined internally):
$env.config.menus ++= [
    {
        name: help_menu
        only_buffer_difference: true
        marker: "? "
        type: {
            layout: description
            columns: 4
            # col_width is an optional value. If missing, the entire screen width is used to
            # calculate the column width
            col_width: 20
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

# ---------------
# Plugin behavior
# ---------------
# Per-plugin configuration. See https://www.nushell.sh/contributor-book/plugins.html#plugin-configuration
$env.config.plugins = {}

# Plugin garbage collection configuration
# $env.config.plugin_gc.*

# enabled (bool): true/false to enable/disable stopping inactive plugins
$env.config.plugin_gc.default.enabled = true
# stop_after (duration): How long to wait after a plugin is inactive before stopping it
$env.config.plugin_gc.default.stop_after = 10sec
# plugins (record): Alternate garbage collection configuration per-plugin.
$env.config.plugin_gc.plugins = {
  # gstat: {
  #   enabled: false
  # }
}

# -------------------------------------
# Themes/Colors and Syntax Highlighting
# -------------------------------------
# For more information on defining custom themes, see
# https://www.nushell.sh/book/coloring_and_theming.html

# Use and/or contribute to the theme collection at
# https://github.com/nushell/nu_scripts/tree/main/themes

# Values:

# highlight_resolved_externals (bool):
# true: Applies the `color_config.shape_external_resolved` color (below) to external commands
#       which are found (resolved) on the path
# false: Applies the `color_config.shape_external` color to *all* externals simply based on whether
#        or not they would be *parsed* as an external command based on their position.
# Defaults to false for systems with a slower search path
$env.config.highlight_resolved_externals = true

# color_config (record): A record of shapes, types, UI elements, etc. that can be styled (e.g.,
# colorized) in Nushell, either on the commandline itself (shapes) or in output.
#
# Note that this is usually set through a theme provided by a record in a custom command. For
# instance, the standard library contains two "starter" theme commands: "dark-theme" and
# "light-theme". For example:
use std/config dark-theme
$env.config.color_config = (dark-theme)

# Or, individual color settings can be configured or overridden.
#
# Values can be one of:
# - A color name such as "red" (see `ansi -l` for a list)
# - A color RGB value in the form of "#C4C9C6"
# - A record including:
#   * `fg` (color)
#   * `bg` (color)
#   * `attr`: a string with one or more of:
#     - 'n': normal
#     - 'b': bold
#     - 'u': underline
#     - 'r': reverse
#     - 'i': italics
#     - 'd': dimmed

# foreground, background, and cursor colors are not handled by Nushell, but can be used by
# custom-commands such as `theme` from the nu_scripts repository. That `theme` command can be
# used to set the terminal foreground, background, and cursor colors.
$env.config.color_config.foreground
$env.config.color_config.background
$env.config.color_config.cursor

# -------------------------------------------------------------------------------------------------
# shape_: Applies syntax highlighting based on the "shape" (inferred or declared type) of an
# element on the commandline. Nushell's parser can identify shapes based on many criteria, often
# as the commandline is being typed.

# shape_string: Can appear as a single-or-quoted value, a bareword string, the key of a record,
# an argument which has been declared as a string, and other parsed strings.
$env.config.color_config.shape_string

# shape_string_interpolation: A single-or-double-quoted string interpolation. This style
# applies to the dollar sign and quotes of the string. The elements inside the string are
# styled according to their own shape.
$env.config.color_config.shape_string_interpolation

# shape_raw_string: a raw string literal. E.g., r#'This is a raw string'#. This style applies
# to the entire raw string.
$env.config.color_config.shape_raw_string

# shape_record: A record-literal. This style applies to the brackets around the record. The keys
# and values will be styled according to their individual shapes.
$env.config.color_config.shape_record

# shape_list: A list-literal. This style applies to the brackets and list separator only. The
# items in a list are styled according to their individual shapes.
$env.config.color_config.shape_list

# shape_table: A table-literl. Color applies to the brackets, semicolon, and list separators. The
# items in the table are style according to their individual shapes.
$env.config.color_config.shape_table

# shape_bool: A boolean-literal `true` or `false` value
$env.config.color_config.shape_bool

# shape_int: Integer literals
$env.config.color_config.shape_int

# shape_float: Float literals. E.g., 5.4
# Also integer literals in a float-argument position
$env.config.color_config.shape_float

# shape_range: Range literals
$env.config.color_config.shape_range

# shape_binary: Binary literals
$env.config.color_config.shape_binary

# shape_datetime: Datetime literals
$env.config.color_config.shape_datetime

# shape_custom: A custom value, usually from a plugin
$env.config.color_config.shape_custom

# shape_nothing: A literal `null`
$env.config.color_config.shape_nothing

# shape_literal: Not currently used
$env.config.color_config.shape_literal

# shape_operator: An operator such as +, -, ++, in, not-in, etc.
$env.config.color_config.shape_operator

# shape_filepath: An argument that appears in the position of a `path` shape for a command
$env.config.color_config.shape_filepath

# shape_directory: A more specific 'path' shape that only accepts a directory.
$env.config.color_config.shape_directory

# shape_globpattern: An argument in the position of a glob parameter. E.g., the asterisk (or any other string) in `ls *`.
$env.config.color_config.shape_globpattern

# shape_glob_interpolation: Deprecated
$env.config.color_config.shape_glob_interpolation

# shape_garbage: When an argument is of the wrong type or cannot otherwise be parsed.
# E.g., `ls {a: 5}` - A record argument to `ls` is 'garbage'. Also applied in real-time when
# an expression is not (yet) properly closed.
$env.config.color_config.shape_garbage

# shape_variable: The *use* of a variable. E.g., `$env` or `$a`.
$env.config.color_config.shape_variable

# shape_vardecl: The *declaration* of a variable. E.g. the "a" in `let a = 5`.
$env.config.color_config.shape_vardecl

# shape_matching_brackets: When the cursor is positioned on an opening or closing bracket (e.g,
# braces, curly braces, or parenthesis), and there is a matching opening/closing bracket, both will
# temporarily have this style applied.
$env.config.color_config.shape_matching_brackets

# shape_pipe: The pipe `|` when used to separate expressions in a pipeline
$env.config.color_config.shape_pipe

# shape_internalcall: A known Nushell built-in or custom command in the "command position" (usually
# the first bare word of an expression).
$env.config.color_config.shape_internalcall

# shape_external: A token in the "command position" (see above) that is not a known Nushell
# built-in or custom command. This is assumed to be an external command.
$env.config.color_config.shape_external

# shape_external_resolved: Requires "highlight_resolved_externals" (above) to be enabled.
# When a token matches the "external" requirement (above) and is also a *confirmed* external
# command, this style will be applied.
$env.config.color_config.shape_external_resolved

# shape_externalarg: Arguments to an external command (whether resolved or not)
$env.config.color_config.shape_externalarg

# shape_match_pattern: The matching pattern for each arm in a match expression. Does not
# include the guard expression (if present).
$env.config.color_config.shape_match_pattern

# shape_block: The curly-braces around a block. Expressions within the block will have their
# their own shapes' styles applied.
$env.config.color_config.shape_block

# shape_signature: The parameter definitions and input/output types for a command signature.
$env.config.color_config.shape_signature

# shape_keyword: Not current used
$env.config.color_config.shape_keyword

# shape_closure: Styles the brackets and arguments of a closure.
$env.config.color_config.shape_closure

# shape_direction: The redirection symbols such as `o>`, `error>`, `e>|`, etc.
$env.config.color_config.shape_redirection

# shape_flag: Flags and switches to internal and custom-commands. Only the `--flag` (`-f`) portion
# is styled. The argument to a flag will be styled using its own shape.
$env.config.color_config.shape_flag

# -------------------------------------------------------------------------------------------------
# color.config.<type>
# *Values* of a particular *type* can be styled differently than the *shape*.
# Note that the style is applied only when this type is displayed in *structured* data (list,
# record, or table). It is not currently applied to basic raw values.
#
# Note that some types are rarely or never seen in a context in which styling would be applied.
# For example, a cell-path *value* is unlikely to (but can) appear in a list, record, or table.
#
# Tip: In addition to the styles above (fg, bg, attr), types typically accept a closure which can
# dynamically change the style based on the *value*. For instance, the themes in the nu_scripts
# repository will style filesizes difference in an `ls` (or other table) differently depending on
# their magnitude.

# Simple examples:

# bool: A boolean value
$env.config.color_config.bool = {||
  if $in {
    {
      bg: 'light_green'
      fg: 'white'
      attr: 'b'
    }
  } else {
    {
      bg: 'yellow'
      fg: 'black'
      attr: 'b'
    }
  }
}

# int: An integer value
$env.config.color_config.int = {||
  if $in == 42 { 'green' } else { 'red' }
}

# Additional type values (without examples):
$env.config.color_config.string      # String
$env.config.color_config.float       # Float value
$env.config.color_config.glob        # Glob value (must be declared)
$env.config.color_config.binary      # Binary value
$env.config.color_config.custom      # Custom value (often from a plugin)
$env.config.color_config.nothing     # Not used, since a null is not displayed
$env.config.color_config.date        # datetime value
$env.config.color_config.filesize    # filesize value
$env.config.color_config.list        # Not currently used. Lists are displayed using their
                                     # members' styles
$env.config.color_config.record      # Not currently used. Records are displayed using their
                                     # member's styles
$env.config.color_config.duration    # Duration type
$env.config.color_config.range       # Range value
$env.config.color_config.cell-path   # Cell-path value
$env.config.color_config.closure     # Not currently used
$env.config.color_config.block       # Not currently used

# Additional UI elements
# hints: The (usually dimmed) style in which completion hints are displayed
$env.config.color_config.hints

# search_result: The style applied to `find` search results
$env.config.color_config.search_result

# header: The column names in a table header
$env.config.color_config.header

# separator: Used for table/list/record borders
$env.config.color_config.separator

# row_index: The `#` or `index` column of a table or list
$env.config.color_config.row_index

# empty: This style is applied to empty/missing values in a table. However, since the ❎
# emoji is used for this purpose, there is limited styling that can be applied.
$env.config.color_config.empty

# leading_trailing_space_bg: When a string value inside structured data has leading or trailing
# whitespace, that whitespace will be displayed using this style.
# Use { attr: n } to disable.
$env.config.color_config.leading_trailing_space_bg = { bg: 'red' }

# banner_foreground: The default text style for the Welcome Banner displayed at startup
$env.config.color_config.banner_foreground = "attr_normal"

# banner_highlight1 and banner_highlight2: Colors for highlighted text in the Welcome Banner
$env.config.color_config.banner_highlight1 = "green"
$env.config.color_config.banner_highlight2 = "purple"

# ------------------------
# `explore` command colors
# ------------------------
# Configure the UI colors of the `explore` command
# Allowed values are the same as for the `color_config` options above.
# Example:
$env.config.explore = {
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

# ---------------------------------------------------------------------------------------
# Environment Variables
# ---------------------------------------------------------------------------------------

# In addition to the $env.config record, a number of other environment variables
# also affect Nushell's behavior:

# PROMPT_*
# --------
# Prompt configuration
# PROMPT_ variables accept either a string or a closure that returns a string

# PROMPT_COMMAND
# --------------
# Defines the primary prompt. Note that the PROMPT_INDICATOR (below) is appended to this value.
# Simple example - Static string:
$env.PROMPT_COMMAND = "Nushell"
# Simple example - Dynamic closure displaying the path:
$env.PROMPT_COMMAND = {|| pwd}

# PROMPT_COMMAND_RIGHT
# --------------------
# Defines a prompt which will appear right-aligned in the terminal
$env.PROMPT_COMMAND_RIGHT = {|| date now | format date "%d-%a %r" }

# PROMPT_INDICATOR*
# -----------------
# The prompt indicators are environmental variables that represent
# the state of the prompt. The specified character(s) will appear
# immediately following the PROMPT_COMMAND

# When in Emacs mode (default):
$env.PROMPT_INDICATOR = "> "

# When in normal vi mode:
$env.PROMPT_INDICATOR_VI_NORMAL = "> "
# When in vi insert-mode:
$env.PROMPT_INDICATOR_VI_INSERT = ": "

# When a commandline extends across multiple lines:
$env.PROMPT_MULTILINE_INDICATOR = "::: "

# TRANSIENT_PROMPT_*
# ------------------
# Allows a different prompt to be shown after a command has been executed.  This
# can be useful if you have a 2-line prompt. Instead of each previously-entered
# command taking up at least 2 lines, the transient prompt can condense it to a
# shorter version. The following example shows a rocket emoji before each
# previously-entered command:
$env.TRANSIENT_PROMPT_COMMAND = "🚀 "
$env.TRANSIENT_PROMPT_INDICATOR = ""
$env.TRANSIENT_PROMPT_INDICATOR_VI_INSERT = ""
$env.TRANSIENT_PROMPT_INDICATOR_VI_NORMAL = ""
# Tip: Removing the transient multiline indicator and right-prompt can simplify
#      copying from the terminal
$env.TRANSIENT_PROMPT_MULTILINE_INDICATOR = ""
$env.TRANSIENT_PROMPT_COMMAND_RIGHT = ""

# ENV_CONVERSIONS
# ---------------
# Certain variables, such as those containing multiple paths, are often stored as a
# colon-separated string in other shells. Nushell can convert these automatically to a
# more convenient Nushell list.  The ENV_CONVERSIONS variable specifies how environment
# variables are:
# - converted from a string to a value on Nushell startup (from_string)
# - converted from a value back to a string when running external commands (to_string)
#
# Note: The OS Path variable is automatically converted before env.nu loads, so it can
# be treated a list in this file.
#
# Note: Environment variables are not case-sensitive, so the following will work
# for both Windows and Unix-like platforms.
#
# By default, the internal conversion looks something like the following, so there
# is no need to add this in your actual env.nu:
$env.ENV_CONVERSIONS = {
    "Path": {
        from_string: { |s| $s | split row (char esep) | path expand --no-symlink }
        to_string: { |v| $v | path expand --no-symlink | str join (char esep) }
    }
}

# Here's an example converts the XDG_DATA_DIRS variable to and from a list:
$env.ENV_CONVERSIONS = $env.ENV_CONVERSIONS | merge {
    "XDG_DATA_DIRS": {
        from_string: { |s| $s | split row (char esep) | path expand --no-symlink }
        to_string: { |v| $v | path expand --no-symlink | str join (char esep) }
    }
}
#
# Other common directory-lists for conversion: TERMINFO_DIRS.
# Note that other variable conversions take place after `config.nu` is loaded.

# NU_LIB_DIRS
# -----------
# Directories in this constant are searched by the
# `use` and `source` commands.
#
# By default, the `scripts` subdirectory of the default configuration
# directory is included:
const NU_LIB_DIRS = [
    ($nu.default-config-dir | path join 'scripts') # add <nushell-config-dir>/scripts
    ($nu.data-dir | path join 'completions') # default home for nushell completions
]
# You can replace (override) or append to this list by shadowing the constant
const NU_LIB_DIRS = $NU_LIB_DIRS ++ [($nu.default-config-dir | path join 'modules')]

# An environment variable version of this also exists. It is searched after the constant.
$env.NU_LIB_DIRS ++= [ ($nu.data-dir | path join "nu_scripts") ]

# NU_PLUGIN_DIRS
# --------------
# Directories to search for plugin binaries when calling add.

# By default, the `plugins` subdirectory of the default configuration
# directory is included:
const NU_PLUGIN_DIRS = [
    ($nu.default-config-dir | path join 'plugins') # add <nushell-config-dir>/plugins
]
# You can replace (override) or append to this list by shadowing the constant
const NU_PLUGIN_DIRS = $NU_PLUGIN_DIRS ++ [($nu.default-config-dir | path join 'plugins')]

# As with NU_LIB_DIRS, an $env.NU_PLUGIN_DIRS is searched after the constant version

# Appending to the OS path is a common configuration task.
# Because of the previous ENV_CONVERSIONS (performed internally
# before your config.nu loads), the path variable is a list that can
# be appended to using, for example:
$env.PATH ++= [ "~/.local/bin" ]

# Or prepend using
$env.PATH = [ "~/.local/bin" ] ++ $env.PATH

# The `path add` function from the Standard Library also provides
# a convenience method for prepending to the path:
use std/util "path add"
path add "~/.local/bin"
path add ($env.CARGO_HOME | path join "bin")

# You can remove duplicate directories from the path using:
$env.PATH = ($env.PATH | uniq)
