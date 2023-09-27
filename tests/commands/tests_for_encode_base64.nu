use std assert

# Parameter name:
# sig type   : binary
# name       : character-set
# type       : named
# shape      : string
# description: specify the character rules for encoding the input. 	Valid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding','binhex', 'bcrypt', 'crypt', 'mutf7'

# Parameter name:
# sig type   : list<any>
# name       : character-set
# type       : named
# shape      : string
# description: specify the character rules for encoding the input. 	Valid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding','binhex', 'bcrypt', 'crypt', 'mutf7'

# Parameter name:
# sig type   : list<binary>
# name       : character-set
# type       : named
# shape      : string
# description: specify the character rules for encoding the input. 	Valid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding','binhex', 'bcrypt', 'crypt', 'mutf7'

# Parameter name:
# sig type   : list<string>
# name       : character-set
# type       : named
# shape      : string
# description: specify the character rules for encoding the input. 	Valid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding','binhex', 'bcrypt', 'crypt', 'mutf7'

# Parameter name:
# sig type   : record
# name       : character-set
# type       : named
# shape      : string
# description: specify the character rules for encoding the input. 	Valid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding','binhex', 'bcrypt', 'crypt', 'mutf7'

# Parameter name:
# sig type   : string
# name       : character-set
# type       : named
# shape      : string
# description: specify the character rules for encoding the input. 	Valid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding','binhex', 'bcrypt', 'crypt', 'mutf7'

# Parameter name:
# sig type   : table
# name       : character-set
# type       : named
# shape      : string
# description: specify the character rules for encoding the input. 	Valid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding','binhex', 'bcrypt', 'crypt', 'mutf7'


# This is the custom command 1 for encode_base64:

#[test]
def encode_base64_encode_binary_data_1 [] {
  let result = (0x[09 F9 11 02 9D 74 E3 5B D8 41 56 C5 63 56 88 C0] | encode base64)
  assert ($result == CfkRAp1041vYQVbFY1aIwA==)
}

# This is the custom command 2 for encode_base64:

#[test]
def encode_base64_encode_a_string_with_default_settings_2 [] {
  let result = ('Some Data' | encode base64)
  assert ($result == U29tZSBEYXRh)
}

# This is the custom command 3 for encode_base64:

#[test]
def encode_base64_encode_a_string_with_the_binhex_character_set_3 [] {
  let result = ('Some Data' | encode base64 --character-set binhex)
  assert ($result == 7epXB5"%A@4J)
}


