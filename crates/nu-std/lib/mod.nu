# std.nu, `used` to load all standard library components

export use dirs *
export-env {
    use dirs *
}
export use help *
export use iter *
export use log *
export use testing *
export use xml *

# Add the given paths to the PATH.
#
# # Example
# - adding some dummy paths to an empty PATH
# ```nushell
# >_ with-env [PATH []] {
#     std path add "foo"
#     std path add "bar" "baz"
#     std path add "fooo" --append
#
#     assert equal $env.PATH ["bar" "baz" "foo" "fooo"]
#
#     print (std path add "returned" --ret)
# }
# ╭───┬──────────╮
# │ 0 │ returned │
# │ 1 │ bar      │
# │ 2 │ baz      │
# │ 3 │ foo      │
# │ 4 │ fooo     │
# ╰───┴──────────╯
# ```
export def-env "path add" [
    --ret (-r)  # return $env.PATH, useful in pipelines to avoid scoping.
    --append (-a)  # append to $env.PATH instead of prepending to.
    ...paths  # the paths to add to $env.PATH.
] {
    let-env PATH = (
        $env.PATH
        | if $append { append $paths }
        else { prepend $paths }
    )

    if $ret {
        $env.PATH
    }
}

# print a command name as dimmed and italic
def pretty-command [] {
    let command = $in
    return $"(ansi default_dimmed)(ansi default_italic)($command)(ansi reset)"
}

# give a hint error when the clip command is not available on the system
def check-clipboard [
    clipboard: string  # the clipboard command name
    --system: string  # some information about the system running, for better error
] {
    if (which $clipboard | is-empty) {
        error make --unspanned {
            msg: $"(ansi red)clipboard_not_found(ansi reset):
    you are running ($system)
    but
    the ($clipboard | pretty-command) clipboard command was not found on your system."
        }
    }
}

# put the end of a pipe into the system clipboard.
#
# Dependencies:
#   - xclip on linux x11
#   - wl-copy on linux wayland
#   - clip.exe on windows
#
# Examples:
#     put a simple string to the clipboard, will be stripped to remove ANSI sequences
#     >_ "my wonderful string" | clip
#     my wonderful string
#     saved to clipboard (stripped)
#
#     put a whole table to the clipboard
#     >_ ls *.toml | clip
#     ╭───┬─────────────────────┬──────┬────────┬───────────────╮
#     │ # │        name         │ type │  size  │   modified    │
#     ├───┼─────────────────────┼──────┼────────┼───────────────┤
#     │ 0 │ Cargo.toml          │ file │ 5.0 KB │ 3 minutes ago │
#     │ 1 │ Cross.toml          │ file │  363 B │ 2 weeks ago   │
#     │ 2 │ rust-toolchain.toml │ file │ 1.1 KB │ 2 weeks ago   │
#     ╰───┴─────────────────────┴──────┴────────┴───────────────╯
#
#     saved to clipboard
#
#     put huge structured data in the clipboard, but silently
#     >_ open Cargo.toml --raw | from toml | clip --silent
#
#     when the clipboard system command is not installed
#     >_ "mm this is fishy..." | clip
#     Error:
#       × clipboard_not_found:
#       │     you are using xorg on linux
#       │     but
#       │     the xclip clipboard command was not found on your system.
export def clip [
    --silent: bool  # do not print the content of the clipboard to the standard output
    --no-notify: bool  # do not throw a notification (only on linux)
] {
    let input = ($in | table | into string | ansi strip)

    match $nu.os-info.name {
        "linux" => {
            if ($env.WAYLAND_DISPLAY? | is-empty) {
                check-clipboard xclip --system $"('xorg' | pretty-command) on linux"
                $input | xclip -sel clip
            } else {
                check-clipboard wl-copy --system $"('wayland' | pretty-command) on linux"
                $input | wl-copy
            }
        },
        "windows" => {
            chcp 65001  # see https://discord.com/channels/601130461678272522/601130461678272524/1085535756237426778
            check-clipboard clip.exe --system $"('xorg' | pretty-command) on linux"
            $input | clip.exe
        },
        "macos" => {
            check-clipboard pbcopy --system macOS
            $input | pbcopy
        },
        _ => {
            error make --unspanned {
                msg: $"(ansi red)unknown_operating_system(ansi reset):
    '($nu.os-info.name)' is not supported by the ('clip' | pretty-command) command.

    please open a feature request in the [issue tracker](char lparen)https://github.com/nushell/nushell/issues/new/choose(char rparen) to add your operating system to the standard library."
            }
        },
    }

    if not $silent {
        print $input
        print $"(ansi white_italic)(ansi white_dimmed)saved to clipboard(ansi reset)"
    }

    if (not $no_notify) and ($nu.os-info.name == linux) {
        notify-send "std clip" "saved to clipboard"
    }
}
