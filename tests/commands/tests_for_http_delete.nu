use std assert

# Parameter name:
# sig type   : nothing
# name       : URL
# type       : positional
# shape      : string
# description: the URL to fetch the contents from

# Parameter name:
# sig type   : nothing
# name       : user
# type       : named
# shape      : any
# description: the username when authenticating

# Parameter name:
# sig type   : nothing
# name       : password
# type       : named
# shape      : any
# description: the password when authenticating

# Parameter name:
# sig type   : nothing
# name       : data
# type       : named
# shape      : any
# description: the content to post

# Parameter name:
# sig type   : nothing
# name       : content-type
# type       : named
# shape      : any
# description: the MIME type of content to post

# Parameter name:
# sig type   : nothing
# name       : max-time
# type       : named
# shape      : int
# description: timeout period in seconds

# Parameter name:
# sig type   : nothing
# name       : headers
# type       : named
# shape      : any
# description: custom headers you want to add 

# Parameter name:
# sig type   : nothing
# name       : raw
# type       : switch
# shape      : 
# description: fetch contents as text rather than a table

# Parameter name:
# sig type   : nothing
# name       : insecure
# type       : switch
# shape      : 
# description: allow insecure server connections when using SSL

# Parameter name:
# sig type   : nothing
# name       : full
# type       : switch
# shape      : 
# description: returns the full response instead of only the body

# Parameter name:
# sig type   : nothing
# name       : allow-errors
# type       : switch
# shape      : 
# description: do not fail if the server returns an error code


# This is the custom command 1 for http_delete:

#[test]
def http_delete_http_delete_from_examplecom_1 [] {
  let result = (http delete https://www.example.com)
  assert ($result == )
}

# This is the custom command 2 for http_delete:

#[test]
def http_delete_http_delete_from_examplecom_with_username_and_password_2 [] {
  let result = (http delete -u myuser -p mypass https://www.example.com)
  assert ($result == )
}

# This is the custom command 3 for http_delete:

#[test]
def http_delete_http_delete_from_examplecom_with_custom_header_3 [] {
  let result = (http delete -H [my-header-key my-header-value] https://www.example.com)
  assert ($result == )
}

# This is the custom command 4 for http_delete:

#[test]
def http_delete_http_delete_from_examplecom_with_body_4 [] {
  let result = (http delete -d 'body' https://www.example.com)
  assert ($result == )
}

# This is the custom command 5 for http_delete:

#[test]
def http_delete_http_delete_from_examplecom_with_json_body_5 [] {
  let result = (http delete -t application/json -d { field: value } https://www.example.com)
  assert ($result == )
}


