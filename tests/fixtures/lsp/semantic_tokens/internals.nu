str substring 1..
| ansi strip

# User defined one
export def "foo bar" [] {
  # inside a block
  (
    echo "🤔🤖🐘"
    | str substring 1..
  )
}

foo bar
