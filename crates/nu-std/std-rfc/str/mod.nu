alias "str dedent" = dedent

# Removes common indent from a multi-line string based on the number of spaces on the last line.
@example "Two leading spaces are removed from all lines" {
  "
  Heading
    Indented Line
    Another Indented Line

  Another Heading
  " | str dedent
} --result "Heading
  Indented Line
  Another Indented Line

Another Heading"
export def dedent [
    --tabs (-t)
]: string -> string {
    let string = $in

    if ($string !~ $'^\s*(char lsep)') {
        return (error make {
            msg: 'First line must be empty'
        })
    }

    if ($string !~ $'(char lsep)[ \t]*$') {
        return (error make {
            msg: 'Last line must contain only whitespace indicating the dedent'
        })
     }

    # Get indent characters from the last line
    let indent_chars = $string
        | str replace -r $"\(?s\).*(char lsep)\([ \t]*\)$" '$1'

    # Skip the first and last lines
    let lines = (
        $string
        | lines
        | skip
        | # Only drop if there is whitespace. Otherwise, `lines`
        | # drops a 0-length line anyway
        | if ($indent_chars | str length) > 0 { drop } else {}
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
    | str join (char line_sep)
}

alias "str unindent" = unindent

# Remove common indent from a multi-line string based on the line with the smallest indent
@example "Two leading spaces are removed from all lines" {
"
  Heading
    Indented Line
    Another Indented Line

  Another Heading
" | str unindent
} --result "Heading
  Indented Line
  Another Indented Line

Another Heading"
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
        | str replace -r $'^[ \t]*(char lsep)' ''
        | # Remove the last line if it is only whitespace (tabs or spaces)
        | str replace -r $'(char lsep)[ \t]*$' ''
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

alias "str align" = align

# Aligns each line in the input string to have the target in the same column through padding
@example "Align variable assignments" { [ "one = 1", "two = 2", "three = 3", "four = 4", "five = 5" ] | str align '=' } --result r#'one   = 1
two   = 2
three = 3
four  = 4
five  = 5'#
@example "Align variable assignments to the center" { [ "one = 1", "two = 2", "three = 3", "four = 4", "five = 5" ] | str align '=' --center } --result r#'  one = 1
  two = 2
three = 3
 four = 4
 five = 5'#
export def align [
    target:string       # Substring to align
    --char (-c) = " "   # Character to use for padding
    --center (-C)       # Add padding at the beginning of the line instead of before the target
    --range (-r): range # The range of lines to align
]: [string -> string, list<string> -> string] {
    # noop on empty string
    if ($in | is-empty) { return "" }
    let $input = $in | to text | lines

    let $indexes = (
        $input
        | enumerate
        | each {|x|
            if $x.index in ($range | default 0..) {
                $x.item | str index-of $target
            } else {
                -1
            }
        }
    )
    let $max = $indexes | math max

    $input
    | zip $indexes
    | each {|x|
        # Fold adding a `$char` at the index until they are in the same column
        # If the substring is not in the line, the index is -1 and it is left as it is
        seq 1 (if $x.1 == -1 { 0 } else { $max - $x.1 })
        | reduce -f ($x.0 | split chars) {|_, acc|
            let $idx = if $center { 0 } else { $x.1 }
            $acc | insert $idx $char
        }
        | str join
    }
    | str join (char nl)
}

alias "str lcp" = lcp

# Given list of strings, find the longest common prefix and return the remaining parts of input.
@example "No matching prefix" { [] | str lcp  } --result {prefix: "", rest: [] success: false}
@example "List of 1" { [abc] | str lcp } --result {prefix: abc, rest: [""] success: true}
@example "Matching prefix" { [abc abd] | str lcp } --result {prefix: ab, rest: [c d], success: true}
@example "Non-matching prefix" { [qwe asd zxc] | str lcp } --result {prefix: "", rest: [qwe asd zxc], success: false}
@example "Format package version differences" {
  let pkg_current = "acme-3.4.5"
  let pkg_updated = "acme-3.6.0"

  let diff = [$pkg_current $pkg_updated] | str lcp

  $"($diff.prefix)>>>($diff.rest.0) => ($diff.rest.1)<<<"
} --result "acme-3.>>>4.5 => 6.0<<<"
export def lcp []: list<string> -> record<prefix: string, rest: list<string>, success: bool> {
    match ($in | length) {
      0 => {prefix: "", rest: [], success: false}
      1 => {prefix: $in.0?, rest: [""], success: true}
      _ => {
        let chars: list<list<string>> = $in | each {|value| $value | split chars --grapheme-clusters }
        let shortest = $in
            | enumerate
            | sort-by { $in.item | str length }
            | get --optional 0

        let shortest_chars = $chars | get --optional $shortest.index
        mut prefix_len = 0

        for i in 0..<($shortest.item | str length) {
          if ($chars | all {|row|
              ($row | get --optional $i) == ($shortest_chars | get --optional $i)
            }) {
            $prefix_len += 1
          } else {
            break
          }
        }

        let split_at = $prefix_len
        let prefix = $shortest_chars | first $split_at | str join
        let rest = $chars | each {|value| $value | slice $split_at.. | str join }

        {prefix: $prefix, rest: $rest, success: ($split_at > 0)}
      }
    }
}
