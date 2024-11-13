# Sample Nushell Environment Config File
#
# Environment variables are usually configured in `env.nu`. Nushell
# sets sensible defaults for many environment variables, so the user's
# `env.nu` only needs to override these defaults if desired.
#
# This file serves as simple "in-shell" documentation for these
# settings, or you can view a more complete discussion online at:
# https://nushell.sh/book/configuration

# PROMPT_*
# --------
# Prompt configuration
# PROMPT_ variables accept either a string or a closure that returns a string

# $env.PROMPT_COMMAND defines the primary prompt. 
# Simple example:
$env.PROMPT_COMMAND = "Nu"

# Closure example:
$env.PROMPT_COMMAND = {|| 
    let dir = match (do --ignore-shell-errors { $env.PWD | path relative-to $nu.home-path }) {
        null => $env.PWD
        '' => '~'
        $relative_pwd => ([~ $relative_pwd] | path join)
    }

    let path_color = (if (is-admin) { ansi red_bold } else { ansi green_bold })
    let separator_color = (if (is-admin) { ansi light_red_bold } else { ansi light_green_bold })
    let path_segment = $"($path_color)($dir)(ansi reset)"

    $path_segment | str replace --all (char path_sep) $"($separator_color)(char path_sep)($path_color)"
}

# PROMPT_INDICATOR*
# -----------------
# The prompt indicators are environmental variables that represent
# the state of the prompt. The specified character will appear
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
# Tip: Removing the multiline indicator and right-prompt can simplify copying from
# the terminal
$env.TRANSIENT_PROMPT_MULTILINE_INDICATOR = ""
$env.TRANSIENT_PROMPT_COMMAND_RIGHT = ""

# ENV_CONVERSIONS
# ---------------
# Specifies how environment variables are:
# - converted from a string to a value on Nushell startup (from_string)
# - converted from a value back to a string when running external commands (to_string)
#
# The Path variables are automatically converted before env.nu loads, so they can
# be treated as lists:
$env.ENV_CONVERSIONS = {
    "PATH": {
        from_string: { |s| $s | split row (char esep) | path expand --no-symlink }
        to_string: { |v| $v | path expand --no-symlink | str join (char esep) }
    }
    "Path": {
        from_string: { |s| $s | split row (char esep) | path expand --no-symlink }
        to_string: { |v| $v | path expand --no-symlink | str join (char esep) }
    }
}

# Other common directory-lists for conversion: XDG_DATA_DIRS and TERMINFO_DIRS.
# Note that other variable conversions take place after `config.nu` is loaded.

# NU_LIB_DIRS
# -----------
# Directories in this environment variable are searched by the
# `use` and `source` commands.
#
# By default, the `scripts` subdirectory of the default configuration
# directory is included:
$env.NU_LIB_DIRS = [
    ($nu.default-config-dir | path join 'scripts') # add <nushell-config-dir>/scripts
    ($nu.data-dir | path join 'completions') # default home for nushell completions
]

# NU_PLUGIN_DIRS
# --------------
# Directories to search for plugin binaries when calling register.

# By default, the `plugins` subdirectory of the default configuration
# directory is included:
$env.NU_PLUGIN_DIRS = [
    ($nu.default-config-dir | path join 'plugins') # add <nushell-config-dir>/plugins
]

# To add entries to PATH (on Windows you might use Path), you can use the following pattern:
# $env.PATH = ($env.PATH | split row (char esep) | prepend '/some/path')
# An alternate way to add entries to $env.PATH is to use the custom command `path add`
# which is built into the nushell stdlib:
# use std "path add"
# $env.PATH = ($env.PATH | split row (char esep))
# path add /some/path
# path add ($env.CARGO_HOME | path join "bin")
# path add ($env.HOME | path join ".local" "bin")
# $env.PATH = ($env.PATH | uniq)