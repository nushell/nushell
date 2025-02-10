# Convert a Nushell value to a list
# 
# Primary useful for range-to-list, but other types are accepted as well.
# 
# Example:
#
# 1..10 | into list
export def "into list" []: any -> list {
  let input = $in
  let type = ($input | describe --detailed | get type)
  match $type {
    range => {$input | each {||}}
    list => $input
    table => $input
    _ => [ $input ]
  }
}

# Convert a list of columns into a table
#
# Examples:
#
# [
#  ([ 1 2 3 ] | wrap a)
#  ([ 4 5 6 ] | wrap b)
#  ([ 7 8 9 ] | wrap c)
# ] | columns-into-table 
# => ╭───┬───┬───┬───╮
# => │ # │ a │ b │ c │
# => ├───┼───┼───┼───┤
# => │ 0 │ 1 │ 4 │ 7 │
# => │ 1 │ 2 │ 5 │ 8 │
# => │ 2 │ 3 │ 6 │ 9 │
# => ╰───┴───┴───┴───╯
#
# Can roundtrip with `table-into-columns`
# 
# ls | table-into-columns | columns-into-table
# => ╭───┬────────────────────────┬──────┬────────┬────────────────╮
# => │ # │          name          │ type │  size  │    modified    │
# => ├───┼────────────────────────┼──────┼────────┼────────────────┤
# => │ 0 │ into-list.nu           │ file │  378 B │ 40 minutes ago │
# => │ 1 │ mod.nu                 │ file │   28 B │ 41 minutes ago │
# => │ 2 │ name-values.nu         │ file │  394 B │ 34 minutes ago │
# => │ 3 │ record-into-columns.nu │ file │ 1.3 kB │ 27 minutes ago │
# => ╰───┴────────────────────────┴──────┴────────┴────────────────╯
export def columns-into-table []: [list<table> -> table] {
    reduce {|it| merge $it}
}

# Convert a record, where each value is a list, into a list of columns.
# { a: [ 1 2 3 ], b: [ 4 5 6 ] } | record-into-columns
# => ╭───┬───────────╮
# => │ 0 │ ╭───┬───╮ │
# => │   │ │ # │ a │ │
# => │   │ ├───┼───┤ │
# => │   │ │ 0 │ 1 │ │
# => │   │ │ 1 │ 2 │ │
# => │   │ │ 2 │ 3 │ │
# => │   │ ╰───┴───╯ │
# => │ 1 │ ╭───┬───╮ │
# => │   │ │ # │ b │ │
# => │   │ ├───┼───┤ │
# => │   │ │ 0 │ 4 │ │
# => │   │ │ 1 │ 5 │ │
# => │   │ │ 2 │ 6 │ │
# => │   │ ╰───┴───╯ │
# => ╰───┴───────────╯
# =>
# This can be especially useful when combined with `columns-into-table`, as in:
#
# { a: [ 1 2 3 ], b: [ 4 5 6 ] } | record-into-columns
# | columns-into-table
# => ╭───┬───┬───╮
# => │ # │ a │ b │
# => ├───┼───┼───┤
# => │ 0 │ 1 │ 4 │
# => │ 1 │ 2 │ 5 │
# => │ 2 │ 3 │ 6 │
# => ╰───┴───┴───╯
# =>
export def record-into-columns []: [record -> list] {
    items {|key, val| $val | wrap $key}
}

# Convert/split a table into a list of columns
#
# Examples:
# ls | table-into-columns 
# => Returns a list of 4 tables, one for each of the `ls` columns
#
# Can be roundtripped with `columns-into-table`
#
# ls | table-into-columns | columns-into-table
# => ╭───┬────────────────────────┬──────┬────────┬────────────────╮
# => │ # │          name          │ type │  size  │    modified    │
# => ├───┼────────────────────────┼──────┼────────┼────────────────┤
# => │ 0 │ into-list.nu           │ file │  378 B │ 40 minutes ago │
# => │ 1 │ mod.nu                 │ file │   28 B │ 41 minutes ago │
# => │ 2 │ name-values.nu         │ file │  394 B │ 34 minutes ago │
# => │ 3 │ record-into-columns.nu │ file │ 1.3 kB │ 27 minutes ago │
# => ╰───┴────────────────────────┴──────┴────────┴────────────────╯
export def table-into-columns []: [table -> list<table>] {
    let IN = $in
    $IN | columns | each {|col| $IN | select $col}
}

# Assign keynames to a list of values, effectively converting the list to a record.
#
# Example:
#
# [ 1 2 3 ] | name-values a b c
# => ╭───┬───╮
# => │ a │ 1 │
# => │ b │ 2 │
# => │ c │ 3 │
# => ╰───┴───╯
export def name-values [...names: string]: [list -> record] {
    let IN = $in
    0.. | zip $IN | into record | rename ...$names
}
