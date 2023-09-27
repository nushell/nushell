use std assert

# Parameter name:
# sig type   : datetime
# name       : time zone
# type       : positional
# shape      : string
# description: time zone description

# Parameter name:
# sig type   : string
# name       : time zone
# type       : positional
# shape      : string
# description: time zone description


# This is the custom command 1 for date_to-timezone:

#[test]
def date_to-timezone_get_the_current_date_in_utc0500_1 [] {
  let result = (date now | date to-timezone '+0500')
  assert ($result == )
}

# This is the custom command 2 for date_to-timezone:

#[test]
def date_to-timezone_get_the_current_local_date_2 [] {
  let result = (date now | date to-timezone local)
  assert ($result == )
}

# This is the custom command 3 for date_to-timezone:

#[test]
def date_to-timezone_get_the_current_date_in_hawaii_3 [] {
  let result = (date now | date to-timezone US/Hawaii)
  assert ($result == )
}

# This is the custom command 4 for date_to-timezone:

#[test]
def date_to-timezone_get_the_current_date_in_hawaii_4 [] {
  let result = ("2020-10-10 10:00:00 +02:00" | date to-timezone "+0500")
  assert ($result == Sat, 10 Oct 2020 13:00:00 +0500 (2 years ago))
}

# This is the custom command 5 for date_to-timezone:

#[test]
def date_to-timezone_get_the_current_date_in_hawaii_from_a_datetime_object_5 [] {
  let result = ("2020-10-10 10:00:00 +02:00" | into datetime | date to-timezone "+0500")
  assert ($result == Sat, 10 Oct 2020 13:00:00 +0500 (2 years ago))
}


