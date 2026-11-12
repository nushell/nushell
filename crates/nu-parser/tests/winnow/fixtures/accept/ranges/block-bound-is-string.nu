# A block is no value of the `number` shape, so these are strings, as in nu; the dots of
# a spread split its braces, which then do not close.
print 5..{ls} 5..{ ls } 5..{...$in}
