use std assert


# This is the custom command 1 for bytes_build:

#[test]
def bytes_build_builds_binary_data_from_0x01_02_0x03_0x04_1 [] {
  let result = (bytes build 0x[01 02] 0x[03] 0x[04])
  assert ($result == [1, 2, 3, 4])
}


