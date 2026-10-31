# The block lexer reads the `#` after the vertical tab, inside the brackets, as a comment to
# the end of the line. The list's items are lexed again without the brackets, where the
# space after it ends the word `1<VT>#`, so the `]` on that line closes no `[`: an error, as
# in `[1 2]]`.
[1# x ]
 2]
