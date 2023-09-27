use std assert

# Parameter name:
# sig type   : list<any>
# name       : other
# type       : positional
# shape      : any
# description: the other input

# Parameter name:
# sig type   : range
# name       : other
# type       : positional
# shape      : any
# description: the other input


# This is the custom command 1 for zip:

#[test]
def zip_zip_two_lists_1 [] {
  let result = ([1 2] | zip [3 4])
  assert ($result == [[1, 3], [2, 4]])
}

# This is the custom command 2 for zip:

#[test]
def zip_zip_two_ranges_2 [] {
  let result = (1..3 | zip 4..6)
  assert ($result == [[1, 4], [2, 5], [3, 6]])
}

# This is the custom command 3 for zip:

#[test]
def zip_rename_ogg_files_to_match_an_existing_list_of_filenames_3 [] {
  let result = (glob *.ogg | zip ['bang.ogg', 'fanfare.ogg', 'laser.ogg'] | each {|| mv $in.0 $in.1 })
  assert ($result == )
}


