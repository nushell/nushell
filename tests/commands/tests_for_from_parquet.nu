use std assert

# Parameter name:
# sig type   : binary
# name       : metadata
# type       : switch
# shape      : 
# description: Convert metadata from .parquet binary into table


# This is the custom command 1 for from_parquet:

#[test]
def from_parquet_convert_from_parquet_binary_into_table_1 [] {
  let result = (open --raw file.parquet | from parquet)
  assert ($result == )
}

# This is the custom command 2 for from_parquet:

#[test]
def from_parquet_convert_from_parquet_binary_into_table_2 [] {
  let result = (open file.parquet)
  assert ($result == )
}

# This is the custom command 3 for from_parquet:

#[test]
def from_parquet_convert_metadata_from_parquet_binary_into_table_3 [] {
  let result = (open -r file.parquet | from parquet --metadata)
  assert ($result == )
}


