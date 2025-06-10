# Convert a Nushell value to a list
# 
# Primary useful for range-to-list, but other types are accepted as well.
@example "Convert a range to a list" {
    1..10 | into list
} --result [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
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
@example "Construct a columns" {
    [
        ([1, 2, 3] | wrap a)
        ([4, 5, 6] | wrap b)
        ([7, 8, 9] | wrap c)
    ] | columns-into-table 
} --result [[a b c]; [1 4 7] [2 5 8] [3 6 9]]
@example "Can roundtrip with `table-into-columns`" {
  ls | table-into-columns | columns-into-table
}
export def columns-into-table []: [list<table> -> table] {
    reduce {|it| merge $it}
}

# Convert a record, where each value is a list, into a list of columns.
@example "Convert record of lists into list of columns" {
    { a: [1, 2, 3], b: [4, 5, 6] } | record-into-columns
} --result [[[a]; [1], [2], [3]], [[b]; [4], [5], [6]]]
@example "This can be especially useful when combined with `columns-into-table`" {
    { a: [1, 2, 3], b: [4, 5, 6] } | record-into-columns | columns-into-table
} --result [[a, b]; [1, 4], [2, 5], [3, 6]]
export def record-into-columns []: [record -> list] {
    items {|key, val| $val | wrap $key}
}

# Convert/split a table into a list of columns
@example "Return a list of 4 tables, one for each of the `ls` columns" {
    ls | table-into-columns 
}
@example "Can be roundtripped with `columns-into-table`" {
    ls | table-into-columns | columns-into-table
} --result [
    [name, type, size, modified];
    ["into-list.nu", file, "378 B", 2025-02-09T20:52:38+03:00],
    ["mod.nu", file, "28 B", 2025-02-09T20:51:38+03:00],
    ["name-values.nu", file, "394 B", 2025-02-09T20:58:38+03:00],
    ["record-into-columns.nu", file, "1.3 kB", 2025-02-09T21:05:38+03:00]
]
export def table-into-columns []: [table -> list<table>] {
    let IN = $in
    $IN | columns | each {|col| $IN | select $col}
}

# Assign keynames to a list of values, effectively converting the list to a record.
@example "Name the items in a list" {
    [1, 2, 3] | name-values a b c
} --result {a: 1, b: 2, c: 3}
export def name-values [...names: string]: [list -> record] {
    let IN = $in
    0.. | zip $IN | into record | rename ...$names
}
