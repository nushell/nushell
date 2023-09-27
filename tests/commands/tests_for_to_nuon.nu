use std assert

# Parameter name:
# sig type   : any
# name       : raw
# type       : switch
# shape      : 
# description: remove all of the whitespace (default behaviour and overwrites -i and -t)

# Parameter name:
# sig type   : any
# name       : indent
# type       : named
# shape      : number
# description: specify indentation width

# Parameter name:
# sig type   : any
# name       : tabs
# type       : named
# shape      : number
# description: specify indentation tab quantity


# This is the custom command 1 for to_nuon:

#[test]
def to_nuon_outputs_a_nuon_string_representing_the_contents_of_this_list_compact_by_default_1 [] {
  let result = ([1 2 3] | to nuon)
  assert ($result == [1, 2, 3])
}

# This is the custom command 2 for to_nuon:

#[test]
def to_nuon_outputs_a_nuon_array_of_integers_with_pretty_indentation_2 [] {
  let result = ([1 2 3] | to nuon --indent 2)
  assert ($result == [
  1,
  2,
  3
])
}

# This is the custom command 3 for to_nuon:

#[test]
def to_nuon_overwrite_any_set_option_with___raw_3 [] {
  let result = ([1 2 3] | to nuon --indent 2 --raw)
  assert ($result == [1, 2, 3])
}

# This is the custom command 4 for to_nuon:

#[test]
def to_nuon_a_more_complex_record_with_multiple_data_types_4 [] {
  let result = ({date: 2000-01-01, data: [1 [2 3] 4.56]} | to nuon --indent 2)
  assert ($result == {
  date: 2000-01-01T00:00:00+00:00,
  data: [
    1,
    [
      2,
      3
    ],
    4.56
  ]
})
}


