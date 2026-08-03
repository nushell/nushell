# custom completion is called with the uniform input record
def comp_with_span [input: record] {
  let end = $input.cursor
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
