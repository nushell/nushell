use std assert

# Parameter name:
# sig type   : any
# name       : filename
# type       : positional
# shape      : path
# description: the filename to use

# Parameter name:
# sig type   : any
# name       : stderr
# type       : named
# shape      : path
# description: the filename used to save stderr, only works with `-r` flag

# Parameter name:
# sig type   : any
# name       : raw
# type       : switch
# shape      : 
# description: save file as raw binary

# Parameter name:
# sig type   : any
# name       : append
# type       : switch
# shape      : 
# description: append input to the end of the file

# Parameter name:
# sig type   : any
# name       : force
# type       : switch
# shape      : 
# description: overwrite the destination

# Parameter name:
# sig type   : any
# name       : progress
# type       : switch
# shape      : 
# description: enable progress bar


# This is the custom command 1 for save:

#[test]
def save_save_a_string_to_footxt_in_the_current_directory_1 [] {
  let result = ('save me' | save foo.txt)
  assert ($result == )
}

# This is the custom command 2 for save:

#[test]
def save_append_a_string_to_the_end_of_footxt_2 [] {
  let result = ('append me' | save --append foo.txt)
  assert ($result == )
}

# This is the custom command 3 for save:

#[test]
def save_save_a_record_to_foojson_in_the_current_directory_3 [] {
  let result = ({ a: 1, b: 2 } | save foo.json)
  assert ($result == )
}

# This is the custom command 4 for save:

#[test]
def save_save_a_running_programs_stderr_to_footxt_4 [] {
  let result = (do -i {} | save foo.txt --stderr foo.txt)
  assert ($result == )
}

# This is the custom command 5 for save:

#[test]
def save_save_a_running_programs_stderr_to_separate_file_5 [] {
  let result = (do -i {} | save foo.txt --stderr bar.txt)
  assert ($result == )
}


