# Commands for interacting with the system clipboard
#
# > These commands require your terminal to support OSC 52
# > Terminal multiplexers such as screen, tmux, zellij etc may interfere with this command

# Copy input to system clipboard (stdlib command deprecated)
@example "Copy a string to the clipboard" {
  "Hello" | copy
}
@deprecated "Use `clip copy` without `use std/clip`, for OCS 52 copy request use `clip copy52`"
export def copy [
  --ansi (-a)                 # Copy ansi formatting
]: any -> nothing {
  $in | copy52 --ansi=$ansi
}

# Copy input to system clipboard using OSC 52 request
@example "Copy a string to the clipboard" {
  "Hello" | copy52
}
export def copy52 [
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

# Paste contents of system clipboard (stdlib command deprecated)
@example "Paste a string from the clipboard" {
  paste
} --result "Hello"
@deprecated "Use `clip paste` without `use std/clip`, for OCS 52 paste request use `clip paste52`"
export def paste []: [nothing -> string] {
  paste52
}

# Paste contents of system clipboard using OSC 52 request
@example "Paste a string from the clipboard" {
  paste52
} --result "Hello"
export def paste52 []: [nothing -> string] {
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

# After deprecated commands are removed, prefix will need to be changed to use clip copy or clip copy52.

# Add a prefix to each line of the content to be copied
@example "Format output for Nushell doc" {
  [1 2 3] | prefix '# => '
} --result "# => ╭───┬───╮
# => │ 0 │ 1 │
# => │ 1 │ 2 │
# => │ 2 │ 3 │
# => ╰───┴───╯
# => "
@example "Format output for Nushell doc and copy it" {
  ls | prefix '# => ' | copy
}
export def "prefix" [prefix: string]: any -> string {
  let input = $in | collect
  match ($input | describe -d | get type) {
    $type if $type in [ table, record, list ] => {
      $input | table -e
    }
    _ => {$input}
  }
  | str replace -r --all '(?m)(.*)' $'($prefix)$1'
}
