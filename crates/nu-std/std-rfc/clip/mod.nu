# Commands for interacting with the system clipboard
#
# > These commands require your terminal to support OSC 52
# > Terminal multiplexers such as screen, tmux, zellij etc may interfere with this command

# Copy input to system clipboard
#
# # Example
# ```nushell
# >_ "Hello" | clip copy
# ```
export def copy [
  --ansi (-a)                 # Copy ansi formatting
]: any -> nothing {
  let input = $in | collect
  let text = match ($input | describe -d | get type) {
    $type if $type in [ table, record, list ] => {
      $input | table -e
    }
    _ => {$input}
  }

  let do_strip_ansi = match $ansi {
    true  => {{||}}
    false => {{|| ansi strip }}
  }

  let output = (
    $text
    | do $do_strip_ansi
    | encode base64
  )

	print -n $'(ansi osc)52;c;($output)(ansi st)'
}

# Paste contenst of system clipboard
#
# # Example
# ```nushell
# >_ clip paste
# "Hello"
# ```
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
#
# # Example: Format output for Nushell doc
# ls | clip prefix '# => ' | clip copy
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
