use std assert

# Parameter name:
# sig type   : list<string>
# name       : character-set
# type       : named
# shape      : string
# description: specify the character rules for encoding the input. 	Valid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding','binhex', 'bcrypt', 'crypt', 'mutf7'

# Parameter name:
# sig type   : list<string>
# name       : binary
# type       : switch
# shape      : 
# description: Output a binary value instead of decoding payload as UTF-8

# Parameter name:
# sig type   : record
# name       : character-set
# type       : named
# shape      : string
# description: specify the character rules for encoding the input. 	Valid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding','binhex', 'bcrypt', 'crypt', 'mutf7'

# Parameter name:
# sig type   : record
# name       : binary
# type       : switch
# shape      : 
# description: Output a binary value instead of decoding payload as UTF-8

# Parameter name:
# sig type   : string
# name       : character-set
# type       : named
# shape      : string
# description: specify the character rules for encoding the input. 	Valid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding','binhex', 'bcrypt', 'crypt', 'mutf7'

# Parameter name:
# sig type   : string
# name       : binary
# type       : switch
# shape      : 
# description: Output a binary value instead of decoding payload as UTF-8

# Parameter name:
# sig type   : table
# name       : character-set
# type       : named
# shape      : string
# description: specify the character rules for encoding the input. 	Valid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding','binhex', 'bcrypt', 'crypt', 'mutf7'

# Parameter name:
# sig type   : table
# name       : binary
# type       : switch
# shape      : 
# description: Output a binary value instead of decoding payload as UTF-8


# This is the custom command 1 for decode_base64:

#[test]
def decode_base64_base64_decode_a_value_and_output_as_utf_8_string_1 [] {
  let result = ('U29tZSBEYXRh' | decode base64)
  assert ($result == Some Data)
}

# This is the custom command 2 for decode_base64:

#[test]
def decode_base64_base64_decode_a_value_and_output_as_binary_2 [] {
  let result = ('U29tZSBEYXRh' | decode base64 --binary)
  assert ($result == [83, 111, 109, 101, 32, 68, 97, 116, 97])
}


