# Nushell Config File Documentation
#
# Warning: This file is intended for documentation purposes only and
# is not intended to be used as an actual configuration file as-is.
#
# version = "0.111.0"
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
# History-related Settings
# ------------------------

# history.file_format (string): The format used for the command history file.
# "sqlite": Store history in an SQLite database with additional context (timestamps, etc.).
# "plaintext": Store one command per line without additional context.
# Default: "plaintext"
$env.config.history.file_format = "plaintext"

# history.max_size (int): Maximum number of entries allowed in the history.
# After exceeding this value, the oldest history items will be removed.
# Default: 100_000
$env.config.history.max_size = 100_000

# history.sync_on_enter (bool): Whether the history file is updated after each command.
# true: Write to history file after each command is entered.
# false: Only write history when the shell exits.
# Note: This setting only affects plaintext history; SQLite history always syncs.
# Default: true
$env.config.history.sync_on_enter = true

# history.isolation (bool): Controls history isolation between shell sessions.
# true: New history from other currently-open Nushell sessions is not seen
# when scrolling through history (Up/Down keys).
# false: All commands from other sessions are mixed with the current shell's history.
# Note: Only applies to SQLite-backed history. Older history items are always shown.
# Default: false
$env.config.history.isolation = false

# ----------------------
# Miscellaneous Settings
# ----------------------

# show_banner (bool|string): Control the welcome banner at startup.
# true | "full": Show the full banner including Ellie.
# "short": Show abbreviated banner with just startup time.
# false | "none": Don't show any banner.
# Default: true
$env.config.show_banner = true

# rm.always_trash (bool): Controls default behavior of the rm command.
# true: rm behaves as if --trash/-t is specified (move to system trash).
# false: rm behaves as if --permanent/-p is specified (permanent delete).
# Note: Explicit --trash or --permanent flags always override this setting.
# Note: Requires host OS trashcan support.
# Default: false
$env.config.rm.always_trash = false

# recursion_limit (int): Maximum times a command can call itself recursively.
# Prevents infinite recursion by generating an error when exceeded.
# Must be greater than 1.
# Default: 50
$env.config.recursion_limit = 50

# ------------------
# Clipboard Settings
# ------------------

# clip.resident_mode (bool): Use a background process for clipboard operations on Linux.
# true: Serves clipboard content in a background process for clipboard functionality (Linux-only).
# false: Just sets the clipboard value and forgets.
# Default: true (on Linux), false (otherwise)
$env.config.clip.resident_mode = ($nu.os-info.name == linux)

# clip.default_raw (bool): Controls whether `clip` copies raw content by default.
# true: `clip` will copy content as raw bytes by default.
# false: `clip` will attempt to convert content to a string before copying.
# This can be overridden with the `--raw` flag on the `clip` command.
# Default: false
$env.config.clip.default_raw = false

# ---------------------------
# Commandline Editor Settings
# ---------------------------

# edit_mode (string): Sets the editing behavior of Reedline.
# "emacs": Use Emacs-style keybindings (default).
# "vi": Use Vi-style keybindings with normal and insert modes.
# Default: "emacs"
$env.config.edit_mode = "emacs"

# buffer_editor (string|list|null): Command to edit the current line buffer with Ctrl+O.
# null: Uses $env.VISUAL, then $env.EDITOR, then falls back to default.
# string: Command name to invoke (e.g., "vim", "nano", "code --wait").
# list: Command with arguments as a list (e.g., ["vim", "-p"]).
# Default: null
$env.config.buffer_editor = null

# Example: Setting buffer_editor with arguments as a list:
# $env.config.buffer_editor = ["emacsclient", "-s", "light", "-t"]

# cursor_shape.emacs (string): Cursor shape when in emacs edit mode.
# One of: "block", "underscore", "line", "blink_block", "blink_underscore", "blink_line", or "inherit".
# "inherit" skips setting cursor shape and uses the current terminal setting.
# Default: "inherit"
$env.config.cursor_shape.emacs = "inherit"

# cursor_shape.vi_insert (string): Cursor shape when in vi insert mode.
# One of: "block", "underscore", "line", "blink_block", "blink_underscore", "blink_line", or "inherit".
# Default: "inherit"
$env.config.cursor_shape.vi_insert = "inherit"

# cursor_shape.vi_normal (string): Cursor shape when in vi normal mode.
# One of: "block", "underscore", "line", "blink_block", "blink_underscore", "blink_line", or "inherit".
# Default: "inherit"
$env.config.cursor_shape.vi_normal = "inherit"

# --------------------
# Completions Behavior
# --------------------

# show_hints (bool): Enable or disable inline hints for completions and history.
# true: Show hints as you type.
# false: Don't show hints.
# Default: true
$env.config.show_hints = true

# completions.algorithm (string): The algorithm used for matching completions.
# "prefix": Match from the beginning of the text.
# "substring": Match anywhere in the text.
# "fuzzy": Match using fuzzy matching algorithm.
# Default: "prefix"
$env.config.completions.algorithm = "prefix"

# completions.sort (string): How completion results are sorted.
# "smart": Sort order depends on the algorithm setting.
# "alphabetical": Always sort alphabetically.
# In "smart" mode: prefix/substring use alphabetical; fuzzy uses match score.
# Default: "smart"
$env.config.completions.sort = "smart"

# completions.case_sensitive (bool): Enable case-sensitive completions.
# true: Completions are case-sensitive.
# false: Completions are case-insensitive.
# Default: false
$env.config.completions.case_sensitive = false

# completions.quick (bool): Controls auto-selection of single completion results.
# true: Auto-select the completion when only one option remains.
# false: Always require explicit selection.
# Default: true
$env.config.completions.quick = true

# completions.partial (bool): Controls partial completion behavior.
# true: Partially complete up to the longest common prefix.
# false: Do not partially complete.
# Default: true
$env.config.completions.partial = true

# Example: If a directory contains only "forage", "food", and "forest",
# typing "ls " and pressing Tab will partially complete the first matching letters.
# If the directory also includes "faster", only "f" would be partially completed.

# completions.use_ls_colors (bool): Apply LS_COLORS to file/path completions.
# true: Use LS_COLORS for styling file completions.
# false: Don't use LS_COLORS.
# Default: true
$env.config.completions.use_ls_colors = true

# --------------------
# External Completions
# --------------------

# completions.external.enable (bool): Enable searching for external commands on PATH.
# true: Include external commands in completions.
# false: Disable for performance if PATH includes slow filesystems.
# Default: true
$env.config.completions.external.enable = true

# completions.external.max_results (int): Maximum external commands retrieved from PATH.
# Has no effect if external.enable is false.
# Default: 100
$env.config.completions.external.max_results = 100

# completions.external.completer (closure|null): Custom closure for argument completions.
# The closure receives a |spans| parameter - a list of strings representing
# tokens on the current commandline. Usually set to call a third-party
# completion system like Carapace.
# Default: null
$env.config.completions.external.completer = null

# Example: A simplified Carapace completer (use the official one from Carapace docs):
# $env.config.completions.external.completer = {|spans|
#   carapace $spans.0 nushell ...$spans | from json
# }

# --------------------
# Terminal Integration
# --------------------
# Nushell can output escape codes to enable advanced features in Terminal Emulators.
# Disable features not supported by your terminal or if there are conflicts.

# use_kitty_protocol (bool): Enable the Kitty keyboard enhancement protocol.
# Supported by Kitty, WezTerm, and other terminals. Enables additional keybindings
# (e.g., Ctrl+I and Tab can be mapped separately).
# Default: false
$env.config.use_kitty_protocol = false

# shell_integration.osc2 (bool): Set terminal window/tab title to current directory and command.
# Abbreviates home directory with ~.
# Default: true
$env.config.shell_integration.osc2 = true

# shell_integration.osc7 (bool): Report current directory to terminal using OSC 7.
# Enables opening new tabs/windows in the same directory.
# Default: true (false on Windows)
$env.config.shell_integration.osc7 = ($nu.os-info.name != windows)

# shell_integration.osc8 (bool): Generate clickable links in `ls` output.
# Terminal can launch files in associated applications.
# Default: true
$env.config.shell_integration.osc8 = true

# shell_integration.osc9_9 (bool): Enable OSC 9;9 support (ConEmu/Windows Terminal feature).
# Alternative to OSC 7 for communicating current path.
# Default: false (true on Windows)
$env.config.shell_integration.osc9_9 = ($nu.os-info.name == windows)

# shell_integration.osc133 (bool): Enable OSC 133 support for shell semantic zones.
# Reports prompt location and command exit status to terminal.
# Enables features like collapsible output, prompt-to-prompt scrolling, and click-to-cursor.
# Default: true
$env.config.shell_integration.osc133 = true

# shell_integration.osc633 (bool): Enable OSC 633 support (VS Code extension to OSC 133).
# Provides additional shell integration for VS Code's integrated terminal.
# Default: true
$env.config.shell_integration.osc633 = true

# shell_integration.reset_application_mode (bool): Send ESC[?1l to terminal.
# Keeps cursor key modes in sync between local terminal and remote SSH host.
# Default: true
$env.config.shell_integration.reset_application_mode = true

# bracketed_paste (bool): Enable bracketed-paste mode.
# Allows pasting multiple lines without immediate execution.
# When disabled, each pasted line executes as received.
# Note: Not currently supported on Windows.
# Default: true
$env.config.bracketed_paste = true

# use_ansi_coloring ("auto"|bool): Control ANSI coloring in Nushell output.
# "auto": Determine based on FORCE_COLOR, NO_COLOR, CLICOLOR, TERM="dumb" env vars, or if stdout is a terminal.
# true: Always enable ANSI coloring.
# false: Disable ANSI coloring (use default foreground only).
# Note: Does not affect the `ansi` command.
# Default: "auto"
$env.config.use_ansi_coloring = "auto"

# ----------------------
# Error Display Settings
# ----------------------

# error_style (string): One of "fancy", "plain", "short" or "nested"
# Plain: Display plain-text errors for screen-readers
# Fancy: Display errors using line-drawing characters to point to the span in which the
#        problem occurred.
# Short: Display errors as concise, single-line messages similar to classic shells.
# Nested: Same as Fancy but with nesting for related errors.
$env.config.error_style = "fancy"

# display_errors.exit_code (bool): Show Nushell error when external command returns non-zero.
# true: Display Nushell error message for non-zero exit codes.
# false: Only show the error output from the external command itself.
# Note: Core dump errors are always shown; SIGPIPE never triggers an error.
# Default: false
$env.config.display_errors.exit_code = false

# display_errors.termination_signal (bool): Show error when child process is terminated by signal.
# true: Display Nushell error on signal termination.
# false: Don't show error for signal termination.
# Default: true
$env.config.display_errors.termination_signal = true

# error_lines (int):
# Sets the number of context lines in the error output. Must be a positive integer.
$env.config.error_lines = 1

# -------------
# Table Display
# -------------

# footer_mode (string|int): When to display table footers with column names.
# "always": Always show footer.
# "never": Never show footer.
# "auto": Show when table would scroll the header off screen.
# (int): Show when row count meets or exceeds this value.
# Default: 25
$env.config.footer_mode = 25

# table.mode (string): Visual border style for tables.
# One of: "rounded", "basic", "compact", "compact_double", "light", "thin",
# "with_love", "reinforced", "heavy", "none", "psql", "markdown", "dots",
# "restructured", "ascii_rounded", "basic_compact", "single", "double".
# Can be overridden with `| table --theme/-t`.
# Default: "rounded"
$env.config.table.mode = "rounded"

# table.index_mode (string): When to show the index (#) column.
# "always": Always show index column.
# "never": Never show index column.
# "auto": Show only when an explicit "index" column exists.
# Can be overridden with `| table --index/-i`.
# Default: "always"
$env.config.table.index_mode = "always"

# table.show_empty (bool): Display placeholder for empty tables/lists.
# true: Show "empty list" or "empty record" message.
# false: Display nothing for empty values.
# Default: true
$env.config.table.show_empty = true

# table.padding.left (int): Spaces to pad on the left of cell values.
# Default: 1
$env.config.table.padding.left = 1

# table.padding.right (int): Spaces to pad on the right of cell values.
# Default: 1
$env.config.table.padding.right = 1

# table.trim (record): Rules for handling content when table exceeds terminal width.
# methodology (string): "wrapping" or "truncating".
# truncating_suffix (string): Suffix for truncated text (only for truncating).
# wrapping_try_keep_words (bool): Avoid breaking words when wrapping.
# Default: { methodology: "wrapping", wrapping_try_keep_words: true }
$env.config.table.trim = { methodology: "wrapping", wrapping_try_keep_words: true }

# Example: Using truncating mode instead:
# $env.config.table.trim = { methodology: "truncating", truncating_suffix: "..." }

# table.header_on_separator (bool): Display column headers on table border.
# true: Headers appear embedded in top/bottom border.
# false: Headers in their own row with separator below.
# Default: false
$env.config.table.header_on_separator = false

# table.abbreviated_row_count (int|null): Abbreviate large tables.
# (int): Show first N and last N rows with ellipsis.
# null: Show all rows.
# Can be overridden with `| table --abbreviated/-a`.
# Default: null
$env.config.table.abbreviated_row_count = null

# table.footer_inheritance (bool): Footer behavior in nested tables.
# true: If nested table shows footer, parent also shows footer.
# false: Apply footer_mode rules independently to parent.
# Default: false
$env.config.table.footer_inheritance = false

# table.missing_value_symbol (string): Symbol displayed for missing values.
# Default: "❎"
$env.config.table.missing_value_symbol = "❎"

# table.batch_duration (duration): Time to wait before showing streaming batch.
# Longer durations collect more data before display.
# Default: 1sec
$env.config.table.batch_duration = 1sec

# table.stream_page_size (int): Maximum items in a streaming batch.
# Use `collect` to gather entire stream into one table.
# Default: 1000
$env.config.table.stream_page_size = 1000

# ----------------
# Datetime Display
# ----------------

# datetime_format.table (string|null): Format for datetime in structured data.
# null: Humanize the value (e.g., "now", "a day ago").
# string: Format string (see `into datetime --list` for specifiers).
# Default: null
$env.config.datetime_format.table = null

# datetime_format.normal (string|null): Format for datetime as raw output.
# null: Humanize the value.
# string: Format string for display.
# Default: null
$env.config.datetime_format.normal = null

# ----------------
# Filesize Display
# ----------------

# filesize.unit (string): Unit for displaying file sizes.
# "metric": Auto-scale with metric prefixes (kB, MB, GB).
# "binary": Auto-scale with binary prefixes (KiB, MiB, GiB).
# Fixed: "B", "kB", "KB", "MB", "MiB", "GB", "GiB", "TB", "TiB", "PB", "PiB", "EB", "EiB".
# Default: "metric"
$env.config.filesize.unit = "metric"

# filesize.show_unit (bool): Whether to display the unit suffix.
# Useful to disable when using a fixed unit to avoid repetition.
# Default: true
$env.config.filesize.show_unit = true

# filesize.precision (int|null): Decimal places for file sizes.
# null: Show all significant digits.
# (int): Round to this many decimal places.
# Default: 1
$env.config.filesize.precision = 1

# ---------------------
# Miscellaneous Display
# ---------------------

# render_right_prompt_on_last_line (bool): Right prompt position with multi-line left prompt.
# true: Right prompt appears on the last line of the left prompt.
# false: Right prompt appears on the first line.
# Default: false
$env.config.render_right_prompt_on_last_line = false

# float_precision (int): Decimal places for float values in structured output.
# Default: 2
$env.config.float_precision = 2

# ls.use_ls_colors (bool): Apply LS_COLORS to filenames in `ls` output.
# true: Use LS_COLORS environment variable for styling.
# false: Use color_config string style.
# Default: true
$env.config.ls.use_ls_colors = true

# ls.clickable_links (bool): Generate clickable links in `ls` output.
# Note: This is now controlled by shell_integration.osc8.
# Default: true
$env.config.ls.clickable_links = true

# -----
# Hooks
# -----
# Hooks run code at specific shell events.
# Most accept a string (code), closure, or list of strings/closures.
# See https://www.nushell.sh/book/hooks for details.

# hooks.pre_prompt (list): Hook(s) to run before each prompt is displayed.
# Default: []
$env.config.hooks.pre_prompt = []

# hooks.pre_execution (list): Hook(s) to run after Enter, before command execution.
# Default: []
$env.config.hooks.pre_execution = []

# hooks.env_change (record): Hooks to run when environment variables change.
# Keys are environment variable names; values are lists of hooks.
# Default: {}
$env.config.hooks.env_change = {}

# Example: Run a hook when PWD changes:
# $env.config.hooks.env_change = {
#     PWD: [{|before, after| print $"Changed from ($before) to ($after)" }]
# }

# hooks.display_output (string|closure|null): Process output before display.
# WARNING: A malformed hook can suppress all Nushell output.
# Reset with empty string or null if needed.
# Default: "if (term size).columns >= 100 { table -e } else { table }"
$env.config.hooks.display_output = "if (term size).columns >= 100 { table -e } else { table }"

# hooks.command_not_found (closure|null): Hook when a command is not found.
# Can suggest packages or provide custom error handling.
# Default: null
$env.config.hooks.command_not_found = null

# -----------
# Keybindings
# -----------

# keybindings (list): User-defined keybindings for Reedline.
# Each keybinding is a record with: name, modifier, keycode, mode, and event.
# See https://www.nushell.sh/book/line_editor.html#keybindings for details.
# Default: []
$env.config.keybindings = []

# Example: Add Alt+. keybinding to insert the last token from previous command:
# $env.config.keybindings ++= [
#   {
#     name: insert_last_token
#     modifier: alt
#     keycode: char_.
#     mode: [emacs vi_normal vi_insert]
#     event: [
#       { edit: InsertString, value: "!$" }
#       { send: Enter }
#     ]
#   }
# ]

# -----
# Menus
# -----

# menus (list): Menu configurations for Reedline.
# Menus are typically activated via keybindings.
# See https://www.nushell.sh/book/line_editor.html#menus for details.
# Default: []
$env.config.menus = []

# Example: Custom completion menu configuration:
# $env.config.menus ++= [{
#     name: completion_menu
#     only_buffer_difference: false
#     marker: "| "
#     type: {
#         layout: columnar
#         columns: 4
#         col_width: 20
#         col_padding: 2
#     }
#     style: {
#         text: green
#         selected_text: green_reverse
#         description_text: yellow
#     }
# }]

# -------
# Plugins
# -------

# plugins (record): Per-plugin configuration.
# Keys must match registered plugin names.
# See https://www.nushell.sh/contributor-book/plugins.html#plugin-configuration
# Default: {}
$env.config.plugins = {}

# plugin_gc.default.enabled (bool): Enable plugin garbage collection.
# true: Stop inactive plugins automatically.
# false: Keep plugins running until shell exits.
# Default: true
$env.config.plugin_gc.default.enabled = true

# plugin_gc.default.stop_after (duration): Time to wait before stopping inactive plugins.
# Default: 10sec
$env.config.plugin_gc.default.stop_after = 10sec

# plugin_gc.plugins (record): Per-plugin garbage collection overrides.
# Keys are plugin names; values are records with enabled and/or stop_after.
# Default: {}
$env.config.plugin_gc.plugins = {}

# Example: Disable garbage collection for a specific plugin:
# $env.config.plugin_gc.plugins = {
#   gstat: {
#     enabled: false
#   }
# }

# -------------------------------------
# Themes/Colors and Syntax Highlighting
# -------------------------------------
# For more information on defining custom themes, see
# https://www.nushell.sh/book/coloring_and_theming.html
#
# Use and/or contribute to the theme collection at
# https://github.com/nushell/nu_scripts/tree/main/themes
#
# Note that this is usually set through a theme provided by a record in a custom command. For
# instance, the standard library contains two "starter" theme commands: "dark-theme" and
# "light-theme". For example:
# use std/config dark-theme
# $env.config.color_config = (dark-theme)
#
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
#
# foreground, background, and cursor colors are not handled by Nushell, but can be used by
# custom-commands such as `theme` from the nu_scripts repository. That `theme` command can be
# used to set the terminal foreground, background, and cursor colors.
# $env.config.color_config.foreground
# $env.config.color_config.background
# $env.config.color_config.cursor

# highlight_resolved_externals (bool): Style confirmed external commands differently.
# true: Apply shape_external_resolved color to commands found on PATH.
# false: Apply shape_external to all externals based on parsing position.
# Default: false
$env.config.highlight_resolved_externals = false

# color_config (record): Styling for shapes, types, and UI elements.
# Values can be: color names, RGB values (#RRGGBB), or records with fg, bg, attr keys.
# attr can include: 'n' (normal), 'b' (bold), 'u' (underline), 'r' (reverse), 'i' (italics), 'd' (dimmed).
# Default: (see default_config.nu for full default theme)
$env.config.color_config = {}

# Example: Using a theme from the standard library:
# use std/config dark-theme
# $env.config.color_config = (dark-theme)

# color_config.foreground: Terminal foreground color (for custom theme commands).
# Default: null
$env.config.color_config.foreground = null

# color_config.background: Terminal background color (for custom theme commands).
# Default: null
$env.config.color_config.background = null

# color_config.cursor: Terminal cursor color (for custom theme commands).
# Default: null
$env.config.color_config.cursor = null

# ---------------------------
# Syntax Highlighting (Shapes)
# ---------------------------
# shape_* settings style elements on the commandline based on their parsed "shape".
# Shapes are identified by Nushell's parser as you type.
# Default styles are defined in nu-color-config/src/shape_color.rs.

# color_config.shape_string: Style for string values.
# Applies to quoted strings, barewords, record keys, declared string arguments.
# Default: green
$env.config.color_config.shape_string = "green"

# color_config.shape_string_interpolation: Style for string interpolation ($"..." or $'...').
# Applies to the delimiters; inner expressions use their own shapes.
# Default: cyan_bold
$env.config.color_config.shape_string_interpolation = "cyan_bold"

# color_config.shape_raw_string: Style for raw string literals (r#'...'#).
# Default: light_purple
$env.config.color_config.shape_raw_string = "light_purple"

# color_config.shape_record: Style for record literal braces {}.
# Keys and values use their individual shapes.
# Default: cyan_bold
$env.config.color_config.shape_record = "cyan_bold"

# color_config.shape_list: Style for list literal brackets [].
# Items use their individual shapes.
# Default: cyan_bold
$env.config.color_config.shape_list = "cyan_bold"

# color_config.shape_table: Style for table literal brackets, semicolon, separators.
# Items use their individual shapes.
# Default: blue_bold
$env.config.color_config.shape_table = "blue_bold"

# color_config.shape_bool: Style for boolean literals (true/false).
# Default: light_cyan
$env.config.color_config.shape_bool = "light_cyan"

# color_config.shape_int: Style for integer literals.
# Default: purple_bold
$env.config.color_config.shape_int = "purple_bold"

# color_config.shape_float: Style for float literals and integers in float positions.
# Default: purple_bold
$env.config.color_config.shape_float = "purple_bold"

# color_config.shape_range: Style for range literals (e.g., 1..5).
# Default: yellow_bold
$env.config.color_config.shape_range = "yellow_bold"

# color_config.shape_binary: Style for binary literals (0x[...], 0b[...], 0o[...]).
# Default: purple_bold
$env.config.color_config.shape_binary = "purple_bold"

# color_config.shape_datetime: Style for datetime literals.
# Default: cyan_bold
$env.config.color_config.shape_datetime = "cyan_bold"

# color_config.shape_custom: Style for custom values (usually from plugins).
# Default: green
$env.config.color_config.shape_custom = "green"

# color_config.shape_nothing: Style for literal null values.
# Default: light_cyan
$env.config.color_config.shape_nothing = "light_cyan"

# color_config.shape_literal: Reserved for future use.
# Default: blue
$env.config.color_config.shape_literal = "blue"

# color_config.shape_operator: Style for operators (+, -, ++, in, not-in, etc.).
# Default: yellow
$env.config.color_config.shape_operator = "yellow"

# color_config.shape_filepath: Style for path arguments to commands.
# Default: cyan
$env.config.color_config.shape_filepath = "cyan"

# color_config.shape_directory: Style for directory-only path arguments.
# Default: cyan
$env.config.color_config.shape_directory = "cyan"

# color_config.shape_globpattern: Style for glob pattern arguments (e.g., * in `ls *`).
# Default: cyan_bold
$env.config.color_config.shape_globpattern = "cyan_bold"

# color_config.shape_glob_interpolation: Deprecated.
# Default: cyan_bold
$env.config.color_config.shape_glob_interpolation = "cyan_bold"

# color_config.shape_garbage: Style for invalid or unparsable arguments.
# Also shown for unclosed expressions while typing.
# Default: { fg: default, bg: red, attr: b }
$env.config.color_config.shape_garbage = { fg: "default", bg: "red", attr: "b" }

# color_config.shape_variable: Style for variable references ($env, $a).
# Default: purple
$env.config.color_config.shape_variable = "purple"

# color_config.shape_vardecl: Style for variable declarations (e.g., "a" in `let a = 5`).
# Default: purple
$env.config.color_config.shape_vardecl = "purple"

# color_config.shape_matching_brackets: Style for matching bracket pairs when cursor is on one.
# Default: { attr: u }
$env.config.color_config.shape_matching_brackets = { attr: "u" }

# color_config.shape_pipe: Style for the pipe symbol (|) in pipelines.
# Default: purple_bold
$env.config.color_config.shape_pipe = "purple_bold"

# color_config.shape_internalcall: Style for known Nushell commands in command position.
# Default: cyan_bold
$env.config.color_config.shape_internalcall = "cyan_bold"

# color_config.shape_external: Style for tokens parsed as external commands.
# Default: cyan
$env.config.color_config.shape_external = "cyan"

# color_config.shape_external_resolved: Style for confirmed external commands on PATH.
# Requires highlight_resolved_externals to be true.
# Default: light_yellow_bold
$env.config.color_config.shape_external_resolved = "light_yellow_bold"

# color_config.shape_externalarg: Style for arguments to external commands.
# Default: green_bold
$env.config.color_config.shape_externalarg = "green_bold"

# color_config.shape_match_pattern: Style for match expression patterns (not guards).
# Default: green
$env.config.color_config.shape_match_pattern = "green"

# color_config.shape_block: Style for block curly braces.
# Default: blue_bold
$env.config.color_config.shape_block = "blue_bold"

# color_config.shape_signature: Style for command signature definitions.
# Default: green_bold
$env.config.color_config.shape_signature = "green_bold"

# color_config.shape_keyword: Style for keywords (reserved for future use).
# Default: cyan_bold
$env.config.color_config.shape_keyword = "cyan_bold"

# color_config.shape_closure: Style for closure braces and parameters.
# Default: green_bold
$env.config.color_config.shape_closure = "green_bold"

# color_config.shape_redirection: Style for redirection symbols (o>, e>|, etc.).
# Default: purple_bold
$env.config.color_config.shape_redirection = "purple_bold"

# color_config.shape_flag: Style for flags and switches (--flag, -f).
# Default: blue_bold
$env.config.color_config.shape_flag = "blue_bold"

# --------------------------
# Type Colors (Output Values)
# --------------------------
# These style *values* of a particular *type* in structured data output
# (tables, records, lists). They can accept closures for dynamic styling.
# Default styles are defined in nu-color-config/src/style_computer.rs.

# color_config.bool: Style for boolean values in output.
# Default: light_cyan
$env.config.color_config.bool = "light_cyan"

# Example: Dynamic boolean styling:
# $env.config.color_config.bool = {||
#   if $in { { fg: 'green' attr: 'b' } } else { { fg: 'red' } }
# }

# color_config.int: Style for integer values in output.
# Default: default
$env.config.color_config.int = "default"

# color_config.string: Style for string values in output.
# Default: default
$env.config.color_config.string = "default"

# color_config.float: Style for float values in output.
# Default: default
$env.config.color_config.float = "default"

# color_config.glob: Style for glob values in output.
# Can be a color or closure for dynamic styling.
# Default: cyan_bold
$env.config.color_config.glob = "cyan_bold"

# color_config.closure: Style for closure values in output.
# Can be a color or closure for dynamic styling.
# Note: This is for output values, different from shape_closure which is for syntax.
# Default: green_bold
$env.config.color_config.closure = "green_bold"

# color_config.binary: Style for binary values in output.
# Default: default
$env.config.color_config.binary = "default"

# color_config.custom: Style for custom values in output.
# Default: default
$env.config.color_config.custom = "default"

# color_config.nothing: Style for null values (rarely displayed).
# Default: default
$env.config.color_config.nothing = "default"

# color_config.datetime: Style for datetime values in output.
# Default: purple
$env.config.color_config.datetime = "purple"

# color_config.filesize: Style for filesize values in output.
# Can use a closure for magnitude-based styling.
# Default: cyan
$env.config.color_config.filesize = "cyan"

# color_config.list: Style for list values (reserved, not currently used).
# Default: default
$env.config.color_config.list = "default"

# color_config.record: Style for record values (reserved, not currently used).
# Default: default
$env.config.color_config.record = "default"

# color_config.duration: Style for duration values in output.
# Default: default
$env.config.color_config.duration = "default"

# color_config.range: Style for range values in output.
# Default: default
$env.config.color_config.range = "default"

# color_config.cell-path: Style for cell-path values in output.
# Default: default
$env.config.color_config.cell-path = "default"

# color_config.block: Style for block values (reserved, not currently used).
# Default: default
$env.config.color_config.block = "default"

# -----------------
# UI Element Colors
# -----------------

# color_config.hints: Style for inline completion hints.
# Default: dark_gray
$env.config.color_config.hints = "dark_gray"

# color_config.search_result: Style for `find` command search result highlights.
# Default: { bg: red, fg: default }
$env.config.color_config.search_result = { bg: "red", fg: "default" }

# color_config.header: Style for table column headers.
# Default: green_bold
$env.config.color_config.header = "green_bold"

# color_config.separator: Style for table/list/record borders.
# Default: default
$env.config.color_config.separator = "default"

# color_config.row_index: Style for the index (#) column in tables and lists.
# Default: green_bold
$env.config.color_config.row_index = "green_bold"

# color_config.empty: Style for empty/missing values in tables.
# Limited styling due to emoji usage.
# Default: blue
$env.config.color_config.empty = "blue"

# color_config.leading_trailing_space_bg: Style for leading/trailing whitespace in strings.
# Use { attr: n } to disable highlighting.
# Default: { attr: n }
$env.config.color_config.leading_trailing_space_bg = { attr: "n" }

# -------------
# Banner Colors
# -------------

# color_config.banner_foreground: Default text style for the startup banner.
# Default: "attr_normal"
$env.config.color_config.banner_foreground = "attr_normal"

# color_config.banner_highlight1: First highlight color for the startup banner.
# Default: "green"
$env.config.color_config.banner_highlight1 = "green"

# color_config.banner_highlight2: Second highlight color for the startup banner.
# Default: "purple"
$env.config.color_config.banner_highlight2 = "purple"

# ------------------------
# Explore Command Settings
# ------------------------

# explore (record): UI configuration for the `explore` command.
# Configures colors and styles for the interactive data explorer.
# Default: {}
$env.config.explore = {}

# Example explore configuration:
# $env.config.explore = {
#     status_bar_background: { fg: "#1D1F21", bg: "#C4C9C6" },
#     command_bar_text: { fg: "#C4C9C6" },
#     highlight: { fg: "black", bg: "yellow" },
#     status: {
#         error: { fg: "white", bg: "red" },
#         warn: {}
#         info: {}
#     },
#     selected_cell: { bg: light_blue },
#     config: { cursor_color: 'red' },
#     table: {
#         selected_cell: { bg: 'blue' }
#         show_cursor: false
#     },
#     try: { reactive: true }
# }

# ---------------------------------------------------------------------------------------
# Environment Variables
# ---------------------------------------------------------------------------------------
# The following are environment variables (not $env.config settings) that affect Nushell.

# ------
# Prompt
# ------
# PROMPT_ variables accept either a string or a closure that returns a string.

# PROMPT_COMMAND: Defines the primary prompt.
# Note: PROMPT_INDICATOR is appended to this value.
# Default: A closure that displays the current directory with colors.
$env.PROMPT_COMMAND = {||
    let dir = match (do -i { $env.PWD | path relative-to $nu.home-dir }) {
        null => $env.PWD
        '' => '~'
        $relative_pwd => ([~ $relative_pwd] | path join)
    }

    let path_color = (if (is-admin) { ansi red_bold } else { ansi green_bold })
    let separator_color = (if (is-admin) { ansi light_red_bold } else { ansi light_green_bold })
    let path_segment = $"($path_color)($dir)(ansi reset)"

    $path_segment | str replace --all (char path_sep) $"($separator_color)(char path_sep)($path_color)"
}

# Example: Static string prompt:
# $env.PROMPT_COMMAND = "Nushell"

# Example: Simple prompt showing just the current directory:
# $env.PROMPT_COMMAND = {|| pwd }

# PROMPT_COMMAND_RIGHT: Defines a right-aligned prompt.
# Default: A closure that displays the date/time and last exit code.
$env.PROMPT_COMMAND_RIGHT = {||
    # create a right prompt in magenta with green separators and am/pm underlined
    let time_segment = ([
        (ansi reset)
        (ansi magenta)
        (date now | format date '%x %X') # try to respect user's locale
    ] | str join | str replace --regex --all "([/:])" $"(ansi green)${1}(ansi magenta)" |
        str replace --regex --all "([AP]M)" $"(ansi magenta_underline)${1}")

    let last_exit_code = if ($env.LAST_EXIT_CODE != 0) {([
        (ansi rb)
        ($env.LAST_EXIT_CODE)
    ] | str join)
    } else { "" }

    ([$last_exit_code, (char space), $time_segment] | str join)
}

# Example: Simple right prompt with just date/time:
# $env.PROMPT_COMMAND_RIGHT = {|| date now | format date "%d-%a %r" }

# PROMPT_INDICATOR: Characters shown after PROMPT_COMMAND in emacs mode.
$env.PROMPT_INDICATOR = "> "

# PROMPT_INDICATOR_VI_NORMAL: Prompt indicator in vi normal mode.
$env.PROMPT_INDICATOR_VI_NORMAL = "> "

# PROMPT_INDICATOR_VI_INSERT: Prompt indicator in vi insert mode.
$env.PROMPT_INDICATOR_VI_INSERT = ": "

# PROMPT_MULTILINE_INDICATOR: Prompt indicator for multi-line commands.
$env.PROMPT_MULTILINE_INDICATOR = "::: "

# ----------------
# Transient Prompt
# ----------------
# Transient prompts replace the regular prompt after a command is executed.
# Useful for condensing multi-line prompts in scrollback history.

# TRANSIENT_PROMPT_COMMAND: Alternative prompt shown after command execution.
$env.TRANSIENT_PROMPT_COMMAND = "🚀 "

# TRANSIENT_PROMPT_INDICATOR: Transient version of PROMPT_INDICATOR.
$env.TRANSIENT_PROMPT_INDICATOR = ""

# TRANSIENT_PROMPT_INDICATOR_VI_INSERT: Transient version of vi insert indicator.
$env.TRANSIENT_PROMPT_INDICATOR_VI_INSERT = ""

# TRANSIENT_PROMPT_INDICATOR_VI_NORMAL: Transient version of vi normal indicator.
$env.TRANSIENT_PROMPT_INDICATOR_VI_NORMAL = ""

# TRANSIENT_PROMPT_MULTILINE_INDICATOR: Transient version of multiline indicator.
$env.TRANSIENT_PROMPT_MULTILINE_INDICATOR = ""

# TRANSIENT_PROMPT_COMMAND_RIGHT: Transient version of right prompt.
$env.TRANSIENT_PROMPT_COMMAND_RIGHT = ""

# Tip: Removing transient multiline indicator and right-prompt can simplify copying from terminal.

# ---------------------
# Environment Settings
# ---------------------

# ENV_CONVERSIONS: Specifies how environment variables are converted.
# from_string: Convert from string to Nushell value on startup.
# to_string: Convert back to string when running external commands.
# Note: OS Path variable is automatically converted before env.nu loads.
$env.ENV_CONVERSIONS = {}

# Example: Convert XDG_DATA_DIRS to/from a list:
# $env.ENV_CONVERSIONS = $env.ENV_CONVERSIONS | merge {
#     "XDG_DATA_DIRS": {
#         from_string: { |s| $s | split row (char esep) | path expand --no-symlink }
#         to_string: { |v| $v | path expand --no-symlink | str join (char esep) }
#     }
# }

# NU_LIB_DIRS (const): Directories searched by `use` and `source` commands.
# Default includes <config-dir>/scripts and <data-dir>/completions.
const NU_LIB_DIRS = []

# Example: Add custom directories:
# const NU_LIB_DIRS = [
#     ($nu.default-config-dir | path join 'scripts')
#     ($nu.data-dir | path join 'completions')
# ]
# An environment variable version ($env.NU_LIB_DIRS) is searched after the constant.

# NU_PLUGIN_DIRS (const): Directories searched for plugin binaries.
# Default includes <config-dir>/plugins.
const NU_PLUGIN_DIRS = []

# Example: Add plugin directories:
# const NU_PLUGIN_DIRS = [
#     ($nu.default-config-dir | path join 'plugins')
# ]

# -----------------
# Path Manipulation
# -----------------
# The PATH/Path variable is a list that can be manipulated directly.
# It's automatically converted before config.nu loads.

# Example: Append to path:
# $env.PATH ++= [ "~/.local/bin" ]

# Example: Prepend to path:
# $env.PATH = [ "~/.local/bin" ] ++ $env.PATH

# Example: Using std library path add (prepends by default):
# use std/util "path add"
# path add "~/.local/bin"
# path add ($env.CARGO_HOME | path join "bin")

# Example: Remove duplicate directories:
# $env.PATH = ($env.PATH | uniq)
