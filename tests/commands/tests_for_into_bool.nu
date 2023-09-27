use std assert


# This is the custom command 1 for into_bool:

#[test]
def into_bool_convert_value_to_boolean_in_table_1 [] {
  let result = ([[value]; ['false'] ['1'] [0] [1.0] [true]] | into bool value)
  assert ($result == [{value: false}, {value: true}, {value: false}, {value: true}, {value: true}])
}

# This is the custom command 2 for into_bool:

#[test]
def into_bool_convert_bool_to_boolean_2 [] {
  let result = (true | into bool)
  assert ($result == true)
}

# This is the custom command 3 for into_bool:

#[test]
def into_bool_convert_integer_to_boolean_3 [] {
  let result = (1 | into bool)
  assert ($result == true)
}

# This is the custom command 4 for into_bool:

#[test]
def into_bool_convert_float_to_boolean_4 [] {
  let result = (0.3 | into bool)
  assert ($result == true)
}

# This is the custom command 5 for into_bool:

#[test]
def into_bool_convert_float_string_to_boolean_5 [] {
  let result = ('0.0' | into bool)
  assert ($result == false)
}

# This is the custom command 6 for into_bool:

#[test]
def into_bool_convert_string_to_boolean_6 [] {
  let result = ('true' | into bool)
  assert ($result == true)
}


