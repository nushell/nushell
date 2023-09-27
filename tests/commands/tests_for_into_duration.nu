use std assert

# Parameter name:
# sig type   : duration
# name       : unit
# type       : named
# shape      : string
# description: Unit to convert number into (will have an effect only with integer input)

# Parameter name:
# sig type   : int
# name       : unit
# type       : named
# shape      : string
# description: Unit to convert number into (will have an effect only with integer input)

# Parameter name:
# sig type   : string
# name       : unit
# type       : named
# shape      : string
# description: Unit to convert number into (will have an effect only with integer input)

# Parameter name:
# sig type   : table
# name       : unit
# type       : named
# shape      : string
# description: Unit to convert number into (will have an effect only with integer input)


# This is the custom command 1 for into_duration:

#[test]
def into_duration_convert_duration_string_to_duration_value_1 [] {
  let result = ('7min' | into duration)
  assert ($result == 7min)
}

# This is the custom command 2 for into_duration:

#[test]
def into_duration_convert_compound_duration_string_to_duration_value_2 [] {
  let result = ('1day 2hr 3min 4sec' | into duration)
  assert ($result == 1day 2hr 3min 4sec)
}

# This is the custom command 3 for into_duration:

#[test]
def into_duration_convert_table_of_duration_strings_to_table_of_duration_values_3 [] {
  let result = ([[value]; ['1sec'] ['2min'] ['3hr'] ['4day'] ['5wk']] | into duration value)
  assert ($result == [{value: 1sec}, {value: 2min}, {value: 3hr}, {value: 4day}, {value: 5wk}])
}

# This is the custom command 4 for into_duration:

#[test]
def into_duration_convert_duration_to_duration_4 [] {
  let result = (420sec | into duration)
  assert ($result == 7min)
}

# This is the custom command 5 for into_duration:

#[test]
def into_duration_convert_a_number_of_ns_to_duration_5 [] {
  let result = (1_234_567 | into duration)
  assert ($result == 1ms 234µs 567ns)
}

# This is the custom command 6 for into_duration:

#[test]
def into_duration_convert_a_number_of_an_arbitrary_unit_to_duration_6 [] {
  let result = (1_234 | into duration --unit ms)
  assert ($result == 1sec 234ms)
}


