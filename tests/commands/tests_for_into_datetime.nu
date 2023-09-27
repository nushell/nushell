use std assert

# Parameter name:
# sig type   : int
# name       : timezone
# type       : named
# shape      : string
# description: Specify timezone if the input is a Unix timestamp. Valid options: 'UTC' ('u') or 'LOCAL' ('l')

# Parameter name:
# sig type   : int
# name       : offset
# type       : named
# shape      : int
# description: Specify timezone by offset from UTC if the input is a Unix timestamp, like '+8', '-4'

# Parameter name:
# sig type   : int
# name       : format
# type       : named
# shape      : string
# description: Specify expected format of INPUT string to parse to datetime. Use --list to see options

# Parameter name:
# sig type   : int
# name       : list
# type       : switch
# shape      : 
# description: Show all possible variables for use in --format flag

# Parameter name:
# sig type   : list<string>
# name       : timezone
# type       : named
# shape      : string
# description: Specify timezone if the input is a Unix timestamp. Valid options: 'UTC' ('u') or 'LOCAL' ('l')

# Parameter name:
# sig type   : list<string>
# name       : offset
# type       : named
# shape      : int
# description: Specify timezone by offset from UTC if the input is a Unix timestamp, like '+8', '-4'

# Parameter name:
# sig type   : list<string>
# name       : format
# type       : named
# shape      : string
# description: Specify expected format of INPUT string to parse to datetime. Use --list to see options

# Parameter name:
# sig type   : list<string>
# name       : list
# type       : switch
# shape      : 
# description: Show all possible variables for use in --format flag

# Parameter name:
# sig type   : record
# name       : timezone
# type       : named
# shape      : string
# description: Specify timezone if the input is a Unix timestamp. Valid options: 'UTC' ('u') or 'LOCAL' ('l')

# Parameter name:
# sig type   : record
# name       : offset
# type       : named
# shape      : int
# description: Specify timezone by offset from UTC if the input is a Unix timestamp, like '+8', '-4'

# Parameter name:
# sig type   : record
# name       : format
# type       : named
# shape      : string
# description: Specify expected format of INPUT string to parse to datetime. Use --list to see options

# Parameter name:
# sig type   : record
# name       : list
# type       : switch
# shape      : 
# description: Show all possible variables for use in --format flag

# Parameter name:
# sig type   : string
# name       : timezone
# type       : named
# shape      : string
# description: Specify timezone if the input is a Unix timestamp. Valid options: 'UTC' ('u') or 'LOCAL' ('l')

# Parameter name:
# sig type   : string
# name       : offset
# type       : named
# shape      : int
# description: Specify timezone by offset from UTC if the input is a Unix timestamp, like '+8', '-4'

# Parameter name:
# sig type   : string
# name       : format
# type       : named
# shape      : string
# description: Specify expected format of INPUT string to parse to datetime. Use --list to see options

# Parameter name:
# sig type   : string
# name       : list
# type       : switch
# shape      : 
# description: Show all possible variables for use in --format flag

# Parameter name:
# sig type   : table
# name       : timezone
# type       : named
# shape      : string
# description: Specify timezone if the input is a Unix timestamp. Valid options: 'UTC' ('u') or 'LOCAL' ('l')

# Parameter name:
# sig type   : table
# name       : offset
# type       : named
# shape      : int
# description: Specify timezone by offset from UTC if the input is a Unix timestamp, like '+8', '-4'

# Parameter name:
# sig type   : table
# name       : format
# type       : named
# shape      : string
# description: Specify expected format of INPUT string to parse to datetime. Use --list to see options

# Parameter name:
# sig type   : table
# name       : list
# type       : switch
# shape      : 
# description: Show all possible variables for use in --format flag


# This is the custom command 1 for into_datetime:

#[test]
def into_datetime_convert_any_standard_timestamp_string_to_datetime_1 [] {
  let result = ('27.02.2021 1:55 pm +0000' | into datetime)
  assert ($result == Sat, 27 Feb 2021 13:55:00 +0000 (2 years ago))
}

# This is the custom command 2 for into_datetime:

#[test]
def into_datetime_convert_any_standard_timestamp_string_to_datetime_2 [] {
  let result = ('2021-02-27T13:55:40.2246+00:00' | into datetime)
  assert ($result == Sat, 27 Feb 2021 13:55:40 +0000 (2 years ago))
}

# This is the custom command 3 for into_datetime:

#[test]
def into_datetime_convert_non_standard_timestamp_string_to_datetime_using_a_custom_format_3 [] {
  let result = ('20210227_135540+0000' | into datetime -f '%Y%m%d_%H%M%S%z')
  assert ($result == Sat, 27 Feb 2021 13:55:40 +0000 (2 years ago))
}

# This is the custom command 4 for into_datetime:

#[test]
def into_datetime_convert_nanosecond_precision_unix_timestamp_to_a_datetime_with_offset_from_utc_4 [] {
  let result = (1614434140123456789 | into datetime --offset -5)
  assert ($result == Sat, 27 Feb 2021 13:55:40 +0000 (2 years ago))
}

# This is the custom command 5 for into_datetime:

#[test]
def into_datetime_convert_standard_seconds_unix_timestamp_to_a_utc_datetime_5 [] {
  let result = (1614434140 * 1_000_000_000 | into datetime)
  assert ($result == Sat, 27 Feb 2021 13:55:40 +0000 (2 years ago))
}

# This is the custom command 6 for into_datetime:

#[test]
def into_datetime_convert_list_of_timestamps_to_datetimes_6 [] {
  let result = (["2023-03-30 10:10:07 -05:00", "2023-05-05 13:43:49 -05:00", "2023-06-05 01:37:42 -05:00"] | into datetime)
  assert ($result == [Thu, 30 Mar 2023 10:10:07 -0500 (6 months ago), Fri, 5 May 2023 13:43:49 -0500 (4 months ago), Mon, 5 Jun 2023 01:37:42 -0500 (3 months ago)])
}


