# custom completion should be called with correct position
def comp_with_span [context, pos] {
  let end = $context | str length
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
