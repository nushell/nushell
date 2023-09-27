use std assert


# This is the custom command 1 for date_list-timezone:

#[test]
def date_list-timezone_show_timezones_that_contains_shanghai_1 [] {
  let result = (date list-timezone | where timezone =~ Shanghai)
  assert ($result == [{timezone: Asia/Shanghai}])
}


