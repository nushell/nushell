use std assert


# This is the custom command 1 for date_to-record:

#[test]
def date_to-record_convert_the_current_date_into_a_record_1 [] {
  let result = (date to-record)
  assert ($result == )
}

# This is the custom command 2 for date_to-record:

#[test]
def date_to-record_convert_the_current_date_into_a_record_2 [] {
  let result = (date now | date to-record)
  assert ($result == )
}

# This is the custom command 3 for date_to-record:

#[test]
def date_to-record_convert_a_date_string_into_a_record_3 [] {
  let result = ('2020-04-12T22:10:57.123+02:00' | date to-record)
  assert ($result == {year: 2020, month: 4, day: 12, hour: 22, minute: 10, second: 57, nanosecond: 123000000, timezone: +02:00})
}

# This is the custom command 4 for date_to-record:

#[test]
def date_to-record_convert_a_date_into_a_record_4 [] {
  let result = ('2020-04-12 22:10:57 +0200' | into datetime | date to-record)
  assert ($result == {year: 2020, month: 4, day: 12, hour: 22, minute: 10, second: 57, nanosecond: 0, timezone: +02:00})
}


