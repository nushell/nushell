use std assert

# Parameter name:
# sig type   : string
# name       : preview-body
# type       : named
# shape      : int
# description: How many bytes of the body to preview


# This is the custom command 1 for from_eml:

#[test]
def from_eml_convert_eml_structured_data_into_record_1 [] {
  let result = ('From: test@email.com
Subject: Welcome
To: someone@somewhere.com
Test' | from eml)
  assert ($result == {Subject: Welcome, From: {Name: , Address: test@email.com}, To: {Name: , Address: someone@somewhere.com}, Body: Test})
}

# This is the custom command 2 for from_eml:

#[test]
def from_eml_convert_eml_structured_data_into_record_2 [] {
  let result = ('From: test@email.com
Subject: Welcome
To: someone@somewhere.com
Test' | from eml -b 1)
  assert ($result == {Subject: Welcome, From: {Name: , Address: test@email.com}, To: {Name: , Address: someone@somewhere.com}, Body: T})
}


