# a completer never sees text past the cursor, so the last token it gets ends there
def comp_with_span [token] {
  let end = $token.span.end
  [{
      value: "foo",
      span: {
          start: ($end - 1),
          end: $end,
      }
  }]
}
def cust_command [--foo: string@comp_with_span, ...rest: string@comp_with_span] { }

cust_command foo  foo --foo foo
