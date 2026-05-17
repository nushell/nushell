export def foo [] { "foo" }

export alias bar = echo "bar"

export-env { $env.BAZ = "baz" }
