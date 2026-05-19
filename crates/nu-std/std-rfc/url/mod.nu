# Quickly edit sections of a url without needing to `url parse` and `url join`

# Helper function to reuse code
def with-field [field: string, value: any]: string -> string {
  url parse
  | update $field $value
  | url join
}

# Replace the host of input url
export def with-host [host: string]: string -> string {
    with-field host $host
}

# Set or replace the port of input url
export def with-port [port: oneof<int, string>]: string -> string {
    with-field port $port
}

# Replace the path of input url
export def with-path [path: string]: string -> string {
    with-field path $path
}

# Replace the fragment of input url
export def with-fragment [fragment: string]: string -> string {
    with-field fragment $fragment
}

# Set or replace query parameters of input url
#
# Note that they can be provided as a record of key value pairs, or in table form with key and value columns.
export def with-params [query: oneof<record, table<key: string, value: any>>]: string -> string {
    url parse
    | update params $query
    | update query { $query | url build-query }
    | url join
}
