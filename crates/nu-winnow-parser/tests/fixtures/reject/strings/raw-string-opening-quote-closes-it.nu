# As in nu's lexer, the opening quote may close the raw string: `r#'#` is its raw part,
# and `foo'#` opens a quote that never closes.
r#'#foo'#
