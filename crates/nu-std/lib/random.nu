# Generate a random boolean list.
export def "random-list bool" [
    list_length: int, # A length of the list
    bias: float = 0.5 # A probability of "true"
] {
    if $bias < 0 or $bias > 1 {
        error make {
            msg: "invalid probability: must be between 0 and 1"
        }
    }
    if $list_length < 0  {
        error make {
            msg: "invalid list length: must be greater than 0"
        }
    }

    1..$list_length | each {|it|
        random bool --bias $bias
    }
}

# Generate a random char list.
export def "random-list chars" [
    list_length: int, # A length of the list
    length: int = 5 # A length of the string
] {
    if $list_length < 0  {
        error make {
            msg: "invalid list length: must be greater than 0"
        }
    }
    if $length < 0  {
        error make {
            msg: "invalid string length: must be greater than 0"
        }
    }

    1..$list_length | each {|it|
        random chars --length $length
    }
}

# Generate a random decimal list.
export def "random-list decimal" [
    list_length: int, # A length of the list
    range: range = 1..10 # A range of the value
] {
    if $list_length < 0  {
        error make {
            msg: "invalid list length: must be greater than 0"
        }
    }

    1..$list_length | each {|it|
        random decimal $range
    }
}

# Generate a random dice list.
export def "random-list dice" [
    list_length: int, # A length of the list
    roll_count: int = 6, # A roll count
    side_count: int = 6 # A side count
] {
    if $list_length < 0  {
        error make {
            msg: "invalid list length: must be greater than 0"
        }
    }
    if $roll_count < 0  {
        error make {
            msg: "invalid roll count: must be greater than 0"
        }
    }
    if $side_count < 0  {
        error make {
            msg: "invalid side count: must be greater than 0"
        }
    }

    1..$list_length | each {|it|
        random dice --dice $roll_count --sides $side_count
    }
}

# Generate a random integer list.
export def "random-list integer" [
    list_length: int # A length of the list
    range: range = 1..10 # A range of the value
] {
    if $list_length < 0  {
        error make {
            msg: "invalid list length: must be greater than 0"
        }
    }

    1..$list_length | each {|it|
        random integer $range
    }
}

# Generate a random uuid list.
export def "random-list uuid" [
    list_length: int # A length of the list
] {
    if $list_length < 0  {
        error make {
            msg: "invalid list length: must be greater than 0"
        }
    }

    1..$list_length | each {|it|
        random uuid
    }
}
