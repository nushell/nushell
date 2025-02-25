str substring 1..
| ansi strip

# User defined one
export def "foo bar" [] {
  # inside a block
  (
    # same line
    "🤔🤖🐘" | str substring 1.. | ansi strip
  )
}

foo bar

overlay use foo
