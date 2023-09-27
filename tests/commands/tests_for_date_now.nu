use std assert


# This is the custom command 1 for date_now:

#[test]
def date_now_get_the_current_date_and_display_it_in_a_given_format_string_1 [] {
  let result = (date now | format date "%Y-%m-%d %H:%M:%S")
  assert ($result == )
}

# This is the custom command 2 for date_now:

#[test]
def date_now_get_the_time_duration_from_2019_04_30_to_now_2 [] {
  let result = ((date now) - 2019-05-01)
  assert ($result == )
}

# This is the custom command 3 for date_now:

#[test]
def date_now_get_the_time_duration_since_a_more_accurate_time_3 [] {
  let result = ((date now) - 2019-05-01T04:12:05.20+08:00)
  assert ($result == )
}

# This is the custom command 4 for date_now:

#[test]
def date_now_get_current_time_in_full_rfc3339_format_with_timezone_4 [] {
  let result = (date now | debug)
  assert ($result == )
}


