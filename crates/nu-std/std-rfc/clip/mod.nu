# The clip module has been added to std. This std-rfc module is deprecated and will be removed.
#
# Commands for interacting with the system clipboard
#
# > These commands require your terminal to support OSC 52
# > Terminal multiplexers such as screen, tmux, zellij etc may interfere with this command

module std-clip {
  export use std/clip *
}

use std-clip

# Copy input to system clipboard
@deprecated "The clip module has been moved to std. Please use the std version instead of the std-rfc version." --since 0.106.0 --remove 0.108.0
@example "Copy a string to the clipboard" {
  "Hello" | clip copy
}
export def copy [
  --ansi (-a)                 # Copy ansi formatting
]: any -> nothing {
  std-clip copy --ansi=$ansi
}

# Paste contents of system clipboard
@deprecated "The clip module has been moved to std. Please use the std version instead of the std-rfc version." --since 0.106.0 --remove 0.108.0
@example "Paste a string from the clipboard" {
  clip paste
} --result "Hello"
export def paste []: [nothing -> string] {
  std-clip paste
}

# Add a prefix to each line of the content to be copied
@deprecated "The clip module has been moved to std. Please use the std version instead of the std-rfc version." --since 0.106.0 --remove 0.108.0
@example "Format output for Nushell doc" {
  [1 2 3] | clip prefix '# => '
} --result "# => ╭───┬───╮
# => │ 0 │ 1 │
# => │ 1 │ 2 │
# => │ 2 │ 3 │
# => ╰───┴───╯
# => "
@example "Format output for Nushell doc and copy it" {
  ls | clip prefix '# => ' | clip copy
}
export def prefix [prefix: string]: any -> string {
  std-clip prefix $prefix
}
