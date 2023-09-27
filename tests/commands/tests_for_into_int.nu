use std assert

# Parameter name:
# sig type   : binary
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : binary
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big

# Parameter name:
# sig type   : bool
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : bool
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big

# Parameter name:
# sig type   : datetime
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : datetime
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big

# Parameter name:
# sig type   : duration
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : duration
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big

# Parameter name:
# sig type   : filesize
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : filesize
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big

# Parameter name:
# sig type   : list<any>
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : list<any>
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big

# Parameter name:
# sig type   : list<bool>
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : list<bool>
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big

# Parameter name:
# sig type   : list<datetime>
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : list<datetime>
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big

# Parameter name:
# sig type   : list<duration>
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : list<duration>
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big

# Parameter name:
# sig type   : list<filesize>
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : list<filesize>
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big

# Parameter name:
# sig type   : list<number>
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : list<number>
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big

# Parameter name:
# sig type   : list<string>
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : list<string>
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big

# Parameter name:
# sig type   : number
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : number
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big

# Parameter name:
# sig type   : record
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : record
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big

# Parameter name:
# sig type   : string
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : string
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big

# Parameter name:
# sig type   : table
# name       : radix
# type       : named
# shape      : number
# description: radix of integer

# Parameter name:
# sig type   : table
# name       : endian
# type       : named
# shape      : string
# description: byte encode endian, available options: native(default), little, big


# This is the custom command 1 for into_int:

#[test]
def into_int_convert_string_to_integer_in_table_1 [] {
  let result = ([[num]; ['-5'] [4] [1.5]] | into int num)
  assert ($result == )
}

# This is the custom command 2 for into_int:

#[test]
def into_int_convert_string_to_integer_2 [] {
  let result = ('2' | into int)
  assert ($result == 2)
}

# This is the custom command 3 for into_int:

#[test]
def into_int_convert_float_to_integer_3 [] {
  let result = (5.9 | into int)
  assert ($result == 5)
}

# This is the custom command 4 for into_int:

#[test]
def into_int_convert_decimal_string_to_integer_4 [] {
  let result = ('5.9' | into int)
  assert ($result == 5)
}

# This is the custom command 5 for into_int:

#[test]
def into_int_convert_file_size_to_integer_5 [] {
  let result = (4KB | into int)
  assert ($result == 4000)
}

# This is the custom command 6 for into_int:

#[test]
def into_int_convert_bool_to_integer_6 [] {
  let result = ([false, true] | into int)
  assert ($result == [0, 1])
}

# This is the custom command 7 for into_int:

#[test]
def into_int_convert_date_to_integer_unix_nanosecond_timestamp_7 [] {
  let result = (1983-04-13T12:09:14.123456789-05:00 | into int)
  assert ($result == 419101754123456789)
}

# This is the custom command 8 for into_int:

#[test]
def into_int_convert_to_integer_from_binary_8 [] {
  let result = ('1101' | into int -r 2)
  assert ($result == 13)
}

# This is the custom command 9 for into_int:

#[test]
def into_int_convert_to_integer_from_hex_9 [] {
  let result = ('FF' |  into int -r 16)
  assert ($result == 255)
}

# This is the custom command 10 for into_int:

#[test]
def into_int_convert_octal_string_to_integer_10 [] {
  let result = ('0o10132' | into int)
  assert ($result == 4186)
}

# This is the custom command 11 for into_int:

#[test]
def into_int_convert_0_padded_string_to_integer_11 [] {
  let result = ('0010132' | into int)
  assert ($result == 10132)
}

# This is the custom command 12 for into_int:

#[test]
def into_int_convert_0_padded_string_to_integer_with_radix_12 [] {
  let result = ('0010132' | into int -r 8)
  assert ($result == 4186)
}


