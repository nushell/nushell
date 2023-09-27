use std assert


# This is the custom command 1 for from_vcf:

#[test]
def from_vcf_converts_ics_formatted_string_to_table_1 [] {
  let result = ('BEGIN:VCARD
N:Foo
FN:Bar
EMAIL:foo@bar.com
END:VCARD' | from vcf)
  assert ($result == [{properties: [{name: N, value: Foo, params: }, {name: FN, value: Bar, params: }, {name: EMAIL, value: foo@bar.com, params: }]}])
}


