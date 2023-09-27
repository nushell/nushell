use std assert


# This is the custom command 1 for bytes_reverse:

#[test]
def bytes_reverse_reverse_bytes_0x1f_ff_aa_aa_1 [] {
  let result = (0x[1F FF AA AA] | bytes reverse)
  assert ($result == [170, 170, 255, 31])
}

# This is the custom command 2 for bytes_reverse:

#[test]
def bytes_reverse_reverse_bytes_0xff_aa_aa_2 [] {
  let result = (0x[FF AA AA] | bytes reverse)
  assert ($result == [170, 170, 255])
}


