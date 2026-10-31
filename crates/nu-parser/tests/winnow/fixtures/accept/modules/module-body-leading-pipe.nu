# nu lexes a module body as it is, without looking at its first tokens, so a
# leading `|` is a leading pipe.
module foo { |def x [] {} }
