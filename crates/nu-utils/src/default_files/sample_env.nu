# Sample Nushell Environment Config File
#
# Environment variables are usually configured in `env.nu`. Nushell
# sets sensible defaults for many environment variables, so the user's
# `env.nu` only needs to override these defaults if desired.
#
# This file serves as simple "in-shell" documentation for these
# settings, or you can view a more complete discussion online at:
# https://nushell.sh/book/configuration
#
# You can pretty-print and page this file using:
# config env --sample | nu-highlight | less -R

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
# Directories in this environment variable are searched by the
# `use` and `source` commands.
#
# By default, the `scripts` subdirectory of the default configuration
# directory is included:
$env.NU_LIB_DIRS = [
    ($nu.default-config-dir | path join 'scripts') # add <nushell-config-dir>/scripts
    ($nu.data-dir | path join 'completions') # default home for nushell completions
]
# You can replace (override) or append to this list:
$env.NU_LIB_DIRS ++= ($nu.default-config-dir | path join 'modules')

# NU_PLUGIN_DIRS
# --------------
# Directories to search for plugin binaries when calling register.

# By default, the `plugins` subdirectory of the default configuration
# directory is included:
$env.NU_PLUGIN_DIRS = [
    ($nu.default-config-dir | path join 'plugins') # add <nushell-config-dir>/plugins
]

# Appending to the OS path is a common configuration task.
# Because of the previous ENV_CONVERSIONS (performed internally
# before your env.nu loads), the path variable is a list that can 
# be appended to using, for example:
$env.path ++= "~/.local/bin"

# Or prepend using
$env.path = "~/.local/bin" ++ $env.path

# The `path add` function from the Standard Library also provides
# a convenience method for prepending to the path:
use std/util "path add"
path add "~/.local/bin"
path add ($env.CARGO_HOME | path join "bin")

# You can remove duplicate directories from the path using:
$env.PATH = ($env.PATH | uniq)
