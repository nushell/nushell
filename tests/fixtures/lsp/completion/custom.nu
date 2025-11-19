# custom completion should be called with correct position
def comp_with_span [context, pos] {
  [{
      value: "foo",
      span: {
          start: ($pos - 1),
          end: $pos
      }
  }]
}

def cust_command [--foo: string@comp_with_span, ...rest: string@comp_with_span] { }

cust_command foo  foo --foo foo
