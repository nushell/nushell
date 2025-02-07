# Removes common indent from a multi-line string based on the number of spaces on the last line.
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
# # => Heading
# # =>   Indented Line
# # =>   Another Indented Line
# # => 
# # => Another Heading
export def dedent [
    --tabs (-t)
]: string -> string {
    let string = $in

    if ($string !~ '^\s*\n') {
        return (error make {
            msg: 'First line must be empty'
        })
    }

    if ($string !~ '\n[ \t]*$') {
        return (error make {
            msg: 'Last line must contain only whitespace indicating the dedent'
        })
     }

    # Get indent characters from the last line
    let indent_chars = $string
        | str replace -r "(?s).*\n([ \t]*)$" '$1'

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

    # Has to be done outside the replacement block or the error
    # is converted to text. This is probably a Nushell bug, and
    # this code can be recombined with the next iterator when
    # the Nushell behavior is fixed.
    for line in $lines {
        # Skip lines with whitespace-only
        if $line.text like '^\s*$' { continue }
        # Error if any line doesn't start with enough indentation
        if ($line.text | parse -r $"^\(($indent_chars)\)" | get capture0?.0?) != $indent_chars {
            error make {
                msg: $"Line ($line.lineNumber + 1) must have an indent of ($indent_chars | str length) or more."
            }
        }
    }

    $lines
    | each {|line|
        # Don't operate on lines containing only whitespace
        if ($line.text not-like '^\s*$') {
            $line.text | str replace $indent_chars ''
        } else {
            $line.text
        }
      }
    | to text
      # Remove the trailing newline which indicated
      # indent level
    | str replace -r '(?s)(.*)\n$' '$1'
}

# Remove common indent from a multi-line string based on the line with the smallest indent
# 
# Example - Two leading spaces are removed from all lines:
#
# > let s = "
#      Heading
#        Indented Line
#        Another Indented Line
#
#      Another Heading
#   "
# > $a | str dedent
#
# # => Heading
# # =>   Indented Line
# # =>   Another Indented Line
# # =>
# # => Another Heading
#
export def unindent [
    --tabs (-t)            # String uses tabs instead of spaces for indentation
]: string -> string {
    let indent_char = match $tabs {
        true => '\t'
        false => ' '
    }

    let text = (
        $in
        | # Remove the first line if it is only whitespace (tabs or spaces)
        | str replace -r $'^[ \t]*(char newline)' ''
        | # Remove the last line if it is only whitespace (tabs or spaces)
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