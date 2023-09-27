use std assert

# Parameter name:
# sig type   : datetime
# name       : format string
# type       : positional
# shape      : string
# description: the desired format date

# Parameter name:
# sig type   : datetime
# name       : list
# type       : switch
# shape      : 
# description: lists strftime cheatsheet

# Parameter name:
# sig type   : string
# name       : format string
# type       : positional
# shape      : string
# description: the desired format date

# Parameter name:
# sig type   : string
# name       : list
# type       : switch
# shape      : 
# description: lists strftime cheatsheet


# This is the custom command 1 for format_date:

#[test]
def format_date_format_a_given_date_time_using_the_default_format_rfc_2822_1 [] {
  let result = ('2021-10-22 20:00:12 +01:00' | into datetime | format date)
  assert ($result == Fri, 22 Oct 2021 20:00:12 +0100)
}

# This is the custom command 2 for format_date:

#[test]
def format_date_format_a_given_date_time_as_a_string_using_the_default_format_rfc_2822_2 [] {
  let result = ("2021-10-22 20:00:12 +01:00" | format date)
  assert ($result == Fri, 22 Oct 2021 20:00:12 +0100)
}

# This is the custom command 3 for format_date:

#[test]
def format_date_format_the_current_date_time_using_a_given_format_string_3 [] {
  let result = (date now | format date "%Y-%m-%d %H:%M:%S")
  assert ($result == )
}

# This is the custom command 4 for format_date:

#[test]
def format_date_format_the_current_date_using_a_given_format_string_4 [] {
  let result = (date now | format date "%Y-%m-%d %H:%M:%S")
  assert ($result == )
}

# This is the custom command 5 for format_date:

#[test]
def format_date_format_a_given_date_using_a_given_format_string_5 [] {
  let result = ("2021-10-22 20:00:12 +01:00" | format date "%Y-%m-%d")
  assert ($result == 2021-10-22)
}


