alias "orig update" = update

# Update a column to have a new value if it exists.
#
# If the column exists with the value `null` it will be skipped.
export def "update" [
    field: cell-path # The name of the column to maybe update.
    value: any # The new value to give the cell(s), or a closure to create the value.
]: [record -> record, table -> table, list<any> -> list<any>] {
    let input = $in
    match ($input | describe | str replace --regex '<.*' '') {
        record => {
            if ($input | get -o $field) != null {
                $input | orig update $field $value
            } else { $input }
        }
        table|list => {
            $input | each {|| update $field $value }
        }
        _ => { $input | orig update $field $value }
    }
}
