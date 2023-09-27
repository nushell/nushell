use std assert

# Parameter name:
# sig type   : binary
# name       : data
# type       : positional
# shape      : binary
# description: the binary to add

# Parameter name:
# sig type   : binary
# name       : index
# type       : named
# shape      : int
# description: index to insert binary data

# Parameter name:
# sig type   : binary
# name       : end
# type       : switch
# shape      : 
# description: add to the end of binary

# Parameter name:
# sig type   : list<binary>
# name       : data
# type       : positional
# shape      : binary
# description: the binary to add

# Parameter name:
# sig type   : list<binary>
# name       : index
# type       : named
# shape      : int
# description: index to insert binary data

# Parameter name:
# sig type   : list<binary>
# name       : end
# type       : switch
# shape      : 
# description: add to the end of binary

# Parameter name:
# sig type   : record
# name       : data
# type       : positional
# shape      : binary
# description: the binary to add

# Parameter name:
# sig type   : record
# name       : index
# type       : named
# shape      : int
# description: index to insert binary data

# Parameter name:
# sig type   : record
# name       : end
# type       : switch
# shape      : 
# description: add to the end of binary

# Parameter name:
# sig type   : table
# name       : data
# type       : positional
# shape      : binary
# description: the binary to add

# Parameter name:
# sig type   : table
# name       : index
# type       : named
# shape      : int
# description: index to insert binary data

# Parameter name:
# sig type   : table
# name       : end
# type       : switch
# shape      : 
# description: add to the end of binary


# This is the custom command 1 for bytes_add:

#[test]
def bytes_add_add_bytes_0xaa_to_0x1f_ff_aa_aa_1 [] {
  let result = (0x[1F FF AA AA] | bytes add 0x[AA])
  assert ($result == [170, 31, 255, 170, 170])
}

# This is the custom command 2 for bytes_add:

#[test]
def bytes_add_add_bytes_0xaa_bb_to_0x1f_ff_aa_aa_at_index_1_2 [] {
  let result = (0x[1F FF AA AA] | bytes add 0x[AA BB] -i 1)
  assert ($result == [31, 170, 187, 255, 170, 170])
}

# This is the custom command 3 for bytes_add:

#[test]
def bytes_add_add_bytes_0x11_to_0xff_aa_aa_at_the_end_3 [] {
  let result = (0x[FF AA AA] | bytes add 0x[11] -e)
  assert ($result == [255, 170, 170, 17])
}

# This is the custom command 4 for bytes_add:

#[test]
def bytes_add_add_bytes_0x11_22_33_to_0xff_aa_aa_at_the_end_at_index_1the_index_is_start_from_end_4 [] {
  let result = (0x[FF AA BB] | bytes add 0x[11 22 33] -e -i 1)
  assert ($result == [255, 170, 17, 34, 51, 187])
}


