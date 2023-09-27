use std assert


# This is the custom command 1 for into_decimal:

#[test]
def into_decimal_convert_string_to_float_in_table_1 [] {
  let result = ([[num]; ['5.01']] | into decimal num)
  assert ($result == [{num: 5.01}])
}

# This is the custom command 2 for into_decimal:

#[test]
def into_decimal_convert_string_to_float_2 [] {
  let result = ('1.345' | into decimal)
  assert ($result == 1.345)
}

# This is the custom command 3 for into_decimal:

#[test]
def into_decimal_coerce_list_of_ints_and_floats_to_float_3 [] {
  let result = ([4 -5.9] | into decimal)
  assert ($result == [4, -5.9])
}

# This is the custom command 4 for into_decimal:

#[test]
def into_decimal_convert_boolean_to_float_4 [] {
  let result = (true | into decimal)
  assert ($result == 1)
}


