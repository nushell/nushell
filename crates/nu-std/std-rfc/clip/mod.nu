# Commands for interacting with the system clipboard
#
# > These commands require your terminal to support OSC 52
# > Terminal multiplexers such as screen, tmux, zellij etc may interfere with this command

# Copy input to system clipboard
@example "Copy a string to the clipboard" {
  "Hello" | clip copy
}
export def copy [
  --ansi (-a)                 # Copy ansi formatting
]: any -> nothing {
  let input = $in | collect
  if not $ansi {
    $env.config.use_ansi_coloring = false
  }
  let text = match ($input | describe -d | get type) {
    $type if $type in [ table, record, list ] => {
      $input | table -e
    }
    _ => {$input}
  }

  print -n $'(ansi osc)52;c;($text | encode base64)(ansi st)'
}

# Paste contents of system clipboard
@example "Paste a string from the clipboard" {
  clip paste
} --result "Hello"
export def paste []: [nothing -> string] {
  try {
    term query $'(ansi osc)52;c;?(ansi st)' -p $'(ansi osc)52;c;' -t (ansi st)
  } catch {
    error make -u {
      msg: "Terminal did not responds to OSC 52 paste request."
      help: $"Check if your terminal supports OSC 52."
    }
  }
  | decode
  | decode base64
  | decode
}

# Add a prefix to each line of the content to be copied
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
  let input = $in | collect
  match ($input | describe -d | get type) {
    $type if $type in [ table, record, list ] => {
      $input | table -e
    }
    _ => {$input}
  }
  | str replace -r --all '(?m)(.*)' $'($prefix)$1'
}
