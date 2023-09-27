use std assert

# Parameter name:
# sig type   : binary
# name       : decimals
# type       : named
# shape      : int
# description: decimal digits to which to round

# Parameter name:
# sig type   : bool
# name       : decimals
# type       : named
# shape      : int
# description: decimal digits to which to round

# Parameter name:
# sig type   : datetime
# name       : decimals
# type       : named
# shape      : int
# description: decimal digits to which to round

# Parameter name:
# sig type   : duration
# name       : decimals
# type       : named
# shape      : int
# description: decimal digits to which to round

# Parameter name:
# sig type   : filesize
# name       : decimals
# type       : named
# shape      : int
# description: decimal digits to which to round

# Parameter name:
# sig type   : int
# name       : decimals
# type       : named
# shape      : int
# description: decimal digits to which to round

# Parameter name:
# sig type   : list<any>
# name       : decimals
# type       : named
# shape      : int
# description: decimal digits to which to round

# Parameter name:
# sig type   : number
# name       : decimals
# type       : named
# shape      : int
# description: decimal digits to which to round

# Parameter name:
# sig type   : record
# name       : decimals
# type       : named
# shape      : int
# description: decimal digits to which to round

# Parameter name:
# sig type   : string
# name       : decimals
# type       : named
# shape      : int
# description: decimal digits to which to round

# Parameter name:
# sig type   : table
# name       : decimals
# type       : named
# shape      : int
# description: decimal digits to which to round


# This is the custom command 1 for into_string:

#[test]
def into_string_convert_integer_to_string_and_append_three_decimal_places_1 [] {
  let result = (5 | into string -d 3)
  assert ($result == 5.000)
}

# This is the custom command 2 for into_string:

#[test]
def into_string_convert_float_to_string_and_round_to_nearest_integer_2 [] {
  let result = (1.7 | into string -d 0)
  assert ($result == 2)
}

# This is the custom command 3 for into_string:

#[test]
def into_string_convert_float_to_string_3 [] {
  let result = (1.7 | into string -d 1)
  assert ($result == 1.7)
}

# This is the custom command 4 for into_string:

#[test]
def into_string_convert_float_to_string_and_limit_to_2_decimals_4 [] {
  let result = (1.734 | into string -d 2)
  assert ($result == 1.73)
}

# This is the custom command 5 for into_string:

#[test]
def into_string_try_to_convert_float_to_string_and_provide_negative_decimal_points_5 [] {
  let result = (1.734 | into string -d -2)
  assert ($result == )
}

# This is the custom command 6 for into_string:

#[test]
def into_string_convert_float_to_string_6 [] {
  let result = (4.3 | into string)
  assert ($result == 4.3)
}

# This is the custom command 7 for into_string:

#[test]
def into_string_convert_string_to_string_7 [] {
  let result = ('1234' | into string)
  assert ($result == 1234)
}

# This is the custom command 8 for into_string:

#[test]
def into_string_convert_boolean_to_string_8 [] {
  let result = (true | into string)
  assert ($result == true)
}

# This is the custom command 9 for into_string:

#[test]
def into_string_convert_date_to_string_9 [] {
  let result = ('2020-10-10 10:00:00 +02:00' | into datetime | into string)
  assert ($result == Sat Oct 10 10:00:00 2020)
}

# This is the custom command 10 for into_string:

#[test]
def into_string_convert_filepath_to_string_10 [] {
  let result = (ls Cargo.toml | get name | into string)
  assert ($result == )
}

# This is the custom command 11 for into_string:

#[test]
def into_string_convert_filesize_to_string_11 [] {
  let result = (1KiB | into string)
  assert ($result == 1,024 B)
}

# This is the custom command 12 for into_string:

#[test]
def into_string_convert_duration_to_string_12 [] {
  let result = (9day | into string)
  assert ($result == 1wk 2day)
}


