# Removes common indent from a multi-line string based on the number of spaces on the last line.
# 
# A.k.a. Unindent
#
# Example - Two leading spaces are removed from all lines:
#
# > let s = "
#      Heading
#        Indented Line
#        Another Indented Line
#
#      Another Heading
#      "
# > $a | str dedent
#
# Heading
#   Indented Line
#   Another Indented Line
#
# Another Heading
export def dedent []: string -> string {
    let string = $in

    if ($string | describe) != "string" {
        let span = (view files | last)
        error make {
            msg: 'Requires multi-line string as pipeline input'
            label: {
                text: "err::pipeline_input"
                span: {
                    start: $span.start
                    end: $span.end
                }
            }
        }
    }

    if ($string !~ '^\s*\n') {
        return (error make {
            msg: 'First line must be empty'
        })
    }

    if ($string !~ '\n\s*$') {
        return (error make {
            msg: 'Last line must contain only whitespace indicating the dedent'
        })
     }

    # Get number of spaces on the last line
    let indent = $string
        | str replace -r '(?s).*\n( *)$' '$1'
        | str length

    # Skip the first and last lines
    let lines = (
        $string
        | str replace -r '(?s)^[^\n]*\n(.*)\n[^\n]*$' '$1'
          # Use `split` instead of `lines`, since `lines` will
          # drop legitimate trailing empty lines
        | split row "\n"
        | enumerate
        | rename lineNumber text
    )

    let spaces = ('' | fill -c ' ' -w $indent)

    # Has to be done outside the replacement block or the error
    # is converted to text. This is probably a Nushell bug, and
    # this code can be recombined with the next iterator when
    # the Nushell behavior is fixed.
    for line in $lines {
        if ($line.text !~ '^\s*$') and ($line.text | str index-of --range 0..($indent) $spaces) == -1 {
            error make {
                msg: $"Line ($line.lineNumber + 1) must be indented by ($indent) or more spaces."
            }
        }
    }

    $lines
    | each {|line|
        # Don't operate on lines containing only whitespace
        if ($line.text !~ '^\s*$') {
            $line.text | str replace $spaces ''
        } else {
            $line.text
        }
      }
    | to text
      # Remove the trailing newline which indicated
      # indent level
    | str replace -r '(?s)(.*)\n$' '$1'
}

# Remove common indent from a multi-line string
export def unindent [
    --tab (-t)            # String uses tabs instead of spaces for indentation
]: string -> string {
    let indent_char = match $tab {
        true => '\t'
        false => ' '
    }

    let text = (
        $in
        | # Remove the first line if it is only whitespace (tabs or spaces)
        | str replace -r $'^[ \t]*(char newline)' ''
        | str replace -r $'(char newline)[ \t]*$' ''
    )

    # Early return if there is only a single, empty (other than whitespace) line
    if ($text like '^[ \t]*$') {
        return $text
    }

    let minimumIndent = (
        $text
        | lines
        | # Ignore indentation in any line that is only whitespace
        | where $it not-like '^[ \t]*$'
        | # Replaces the text with its indentation
        | each {
            str replace -r $"^\(($indent_char)*\).*" '$1'
            | str length
        }
        | math min
    )

    let indent_chars = ('' | fill -c $indent_char -w $minimumIndent)

    $text
    | str replace -r --all $"\(?m\)^($indent_chars)" ''
}