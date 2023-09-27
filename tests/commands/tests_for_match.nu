use std assert

# Parameter name:
# sig type   : any
# name       : value
# type       : positional
# shape      : any
# description: value to check

# Parameter name:
# sig type   : any
# name       : match_block
# type       : positional
# shape      : match-block
# description: block to run if check succeeds


# This is the custom command 1 for match:

#[test]
def match_match_on_a_value_in_range_1 [] {
  let result = (match 3 { 1..10 => 'yes!' })
  assert ($result == yes!)
}

# This is the custom command 2 for match:

#[test]
def match_match_on_a_field_in_a_record_2 [] {
  let result = (match {a: 100} { {a: $my_value} => { $my_value } })
  assert ($result == 100)
}

# This is the custom command 3 for match:

#[test]
def match_match_with_a_catch_all_3 [] {
  let result = (match 3 { 1 => { 'yes!' }, _ => { 'no!' } })
  assert ($result == no!)
}

# This is the custom command 4 for match:

#[test]
def match_match_against_a_list_4 [] {
  let result = (match [1, 2, 3] { [$a, $b, $c] => { $a + $b + $c }, _ => 0 })
  assert ($result == 6)
}

# This is the custom command 5 for match:

#[test]
def match_match_against_pipeline_input_5 [] {
  let result = ({a: {b: 3}} | match $in {{a: { $b }} => ($b + 10) })
  assert ($result == 13)
}

# This is the custom command 6 for match:

#[test]
def match_match_with_a_guard_6 [] {
  let result = (match [1 2 3] {
        [$x, ..$y] if $x == 1 => { 'good list' },
        _ => { 'not a very good list' }
    }
    )
  assert ($result == good list)
}


