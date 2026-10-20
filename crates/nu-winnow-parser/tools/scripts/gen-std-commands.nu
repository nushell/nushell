#!/usr/bin/env nu
# List the commands exported by Nushell's standard library, both bare and
# prefixed with their module name (`log info`, `assert equal`), so that a
# parser can resolve them as multi-word command names.
#
#   nu tools/scripts/gen-std-commands.nu | save -f tools/scripts/std_commands.txt

const PATTERN = '^\s*export\s+(?:def|extern)\s+(?:--\w+\s+)*(?:"(?<q>[^"]+)"|' + "'(?<s>[^']+)'" + '|(?<b>\S+))'

# Print one command name per line.
def main [
    std_dir?: path   # the `crates/nu-std` directory of a Nushell checkout (default: the enclosing workspace's)
]: nothing -> string {
    let std_dir = $std_dir | default ($env.FILE_PWD | path join ../../../nu-std) | path expand
    glob ($std_dir | path join '**/*.nu')
        | each {|f|
            let parts = $f | path relative-to $std_dir | path split
            let module = if ($f | path basename) == 'mod.nu' {
                $parts | drop 1 | last
            } else {
                $f | path parse | get stem
            }
            open --raw $f
                | lines
                | parse -r $PATTERN
                | each {|m|
                    let name = [$m.q $m.s $m.b] | where { is-not-empty } | first
                    if ($name | str starts-with "(") or ($name | str starts-with "$") {
                        return []
                    }
                    if $module in [std std-rfc tests] {
                        [$name]
                    } else {
                        [$name, $"($module) ($name)"]
                    }
                }
                | flatten
        }
        | flatten
        | uniq
        | sort
        | str join "\n"
}
