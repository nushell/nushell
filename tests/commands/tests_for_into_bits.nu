use std assert


# This is the custom command 1 for into_bits:

#[test]
def into_bits_convert_a_binary_value_into_a_string_padded_to_8_places_with_0s_1 [] {
  let result = (01b | into bits)
  assert ($result == 00000001)
}

# This is the custom command 2 for into_bits:

#[test]
def into_bits_convert_an_int_into_a_string_padded_to_8_places_with_0s_2 [] {
  let result = (1 | into bits)
  assert ($result == 00000001)
}

# This is the custom command 3 for into_bits:

#[test]
def into_bits_convert_a_filesize_value_into_a_string_padded_to_8_places_with_0s_3 [] {
  let result = (1b | into bits)
  assert ($result == 00000001)
}

# This is the custom command 4 for into_bits:

#[test]
def into_bits_convert_a_duration_value_into_a_string_padded_to_8_places_with_0s_4 [] {
  let result = (1ns | into bits)
  assert ($result == 00000001)
}

# This is the custom command 5 for into_bits:

#[test]
def into_bits_convert_a_boolean_value_into_a_string_padded_to_8_places_with_0s_5 [] {
  let result = (true | into bits)
  assert ($result == 00000001)
}

# This is the custom command 6 for into_bits:

#[test]
def into_bits_convert_a_datetime_value_into_a_string_padded_to_8_places_with_0s_6 [] {
  let result = (2023-04-17T01:02:03 | into bits)
  assert ($result == 01001101 01101111 01101110 00100000 01000001 01110000 01110010 00100000 00110001 00110111 00100000 00110000 00110001 00111010 00110000 00110010 00111010 00110000 00110011 00100000 00110010 00110000 00110010 00110011)
}

# This is the custom command 7 for into_bits:

#[test]
def into_bits_convert_a_string_into_a_raw_binary_string_padded_with_0s_to_8_places_7 [] {
  let result = ('nushell.sh' | into bits)
  assert ($result == 01101110 01110101 01110011 01101000 01100101 01101100 01101100 00101110 01110011 01101000)
}


