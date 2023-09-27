use std assert

# Parameter name:
# sig type   : nothing
# name       : filename
# type       : positional
# shape      : path
# description: the path of the file you want to create

# Parameter name:
# sig type   : nothing
# name       : reference
# type       : named
# shape      : string
# description: change the file or directory time to the time of the reference file/directory

# Parameter name:
# sig type   : nothing
# name       : modified
# type       : switch
# shape      : 
# description: change the modification time of the file or directory. If no timestamp, date or reference file/directory is given, the current time is used

# Parameter name:
# sig type   : nothing
# name       : access
# type       : switch
# shape      : 
# description: change the access time of the file or directory. If no timestamp, date or reference file/directory is given, the current time is used

# Parameter name:
# sig type   : nothing
# name       : no-create
# type       : switch
# shape      : 
# description: do not create the file if it does not exist


# This is the custom command 1 for touch:

#[test]
def touch_creates_fixturejson_1 [] {
  let result = (touch fixture.json)
  assert ($result == )
}

# This is the custom command 2 for touch:

#[test]
def touch_creates_files_a_b_and_c_2 [] {
  let result = (touch a b c)
  assert ($result == )
}

# This is the custom command 3 for touch:

#[test]
def touch_changes_the_last_modified_time_of_fixturejson_to_todays_date_3 [] {
  let result = (touch -m fixture.json)
  assert ($result == )
}

# This is the custom command 4 for touch:

#[test]
def touch_changes_the_last_modified_time_of_files_a_b_and_c_to_a_date_4 [] {
  let result = (touch -m -d "yesterday" a b c)
  assert ($result == )
}

# This is the custom command 5 for touch:

#[test]
def touch_changes_the_last_modified_time_of_file_d_and_e_to_fixturejsons_last_modified_time_5 [] {
  let result = (touch -m -r fixture.json d e)
  assert ($result == )
}

# This is the custom command 6 for touch:

#[test]
def touch_changes_the_last_accessed_time_of_fixturejson_to_a_date_6 [] {
  let result = (touch -a -d "August 24, 2019; 12:30:30" fixture.json)
  assert ($result == )
}


