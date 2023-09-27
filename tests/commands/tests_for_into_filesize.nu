use std assert


# This is the custom command 1 for into_filesize:

#[test]
def into_filesize_convert_string_to_filesize_in_table_1 [] {
  let result = ([[device size]; ["/dev/sda1" "200"] ["/dev/loop0" "50"]] | into filesize size)
  assert ($result == [{device: /dev/sda1, size: 200 B}, {device: /dev/loop0, size: 50 B}])
}

# This is the custom command 2 for into_filesize:

#[test]
def into_filesize_convert_string_to_filesize_2 [] {
  let result = ('2' | into filesize)
  assert ($result == 2 B)
}

# This is the custom command 3 for into_filesize:

#[test]
def into_filesize_convert_float_to_filesize_3 [] {
  let result = (8.3 | into filesize)
  assert ($result == 8 B)
}

# This is the custom command 4 for into_filesize:

#[test]
def into_filesize_convert_int_to_filesize_4 [] {
  let result = (5 | into filesize)
  assert ($result == 5 B)
}

# This is the custom command 5 for into_filesize:

#[test]
def into_filesize_convert_file_size_to_filesize_5 [] {
  let result = (4KB | into filesize)
  assert ($result == 3.9 KiB)
}


