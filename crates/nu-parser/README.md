# nu-parser, the Nushell parser

Nushell's parser is a type-directed parser, meaning that the parser will use type information available during parse time to configure the parser. This allows it to handle a broader range of techniques to handle the arguments of a command.

Nushell's base language is whitespace-separated tokens with the command (Nushell's term for a function) name in the head position:

```
head1 arg1 arg2 | head2
```

## Lexing

The first job of the parser is to a lexical analysis to find where the tokens start and end in the input. This turns the above into:

```
<item: "head1">, <item: "arg1">, <item: "arg2">, <pipe>, <item: "head2">
```

At this point, the parser has little to no understanding of the shape of the command or how to parse its arguments.

## Lite parsing

As nushell is a language of pipelines, pipes form a key role in both separating commands from each other as well as denoting the flow of information between commands. The lite parse phase, as the name suggests, helps to group the lexed tokens into units.

The above tokens are converted the following during the lite parse phase:

```
Pipeline:
  Command #1:
    <item: "head1">, <item: "arg1">, <item: "arg2">
  Command #2:
    <item: "head2">
```

## Parsing

The real magic begins to happen when the parse moves on to the parsing stage. At this point, it traverses the lite parse tree and for each command makes a decision:

* If the command looks like an internal/external command literal: eg) `foo` or `/usr/bin/ls`, it parses it as an internal or external command
* Otherwise, it parses the command as part of a mathematical expression

### Types/shapes

Each command has a shape assigned to each of the arguments in reads in. These shapes help define how the parser will handle the parse.

For example, if the command is written as:

```sql
where $x > 10
```

When the parsing happens, the parser will look up the `where` command and find its Signature. The Signature states what flags are allowed and what positional arguments are allowed (both required and optional). Each argument comes with it a Shape that defines how to parse values to get that position.

In the above example, if the Signature of `where` said that it took three String values, the result would be:

```
CallInfo:
  Name: `where`
  Args:
    Expression($x), a String
    Expression(>), a String
    Expression(10), a String
```

Or, the Signature could state that it takes in three positional arguments: a Variable, an Operator, and a Number, which would give:

```
CallInfo:
  Name: `where`
  Args:
    Expression($x), a Variable
    Expression(>), an Operator
    Expression(10), a Number
```

Note that in this case, each would be checked at compile time to confirm that the expression has the shape requested. For example, `"foo"` would fail to parse as a Number.

Finally, some Shapes can consume more than one token. In the above, if the `where` command stated it took in a single required argument, and that the Shape of this argument was a MathExpression, then the parser would treat the remaining tokens as part of the math expression.

```
CallInfo:
  Name: `where`
  Args:
    MathExpression:
      Op: >
      LHS: Expression($x)
      RHS: Expression(10)
```

When the command runs, it will now be able to evaluate the whole math expression as a single step rather than doing any additional parsing to understand the relationship between the parameters.

## Making space

As some Shapes can consume multiple tokens, it's important that the parser allow for multiple Shapes to coexist as peacefully as possible.

The simplest way it does this is to ensure there is at least one token for each required parameter. If the Signature of the command says that it takes a MathExpression and a Number as two required arguments, then the parser will stop the math parser one token short. This allows the second Shape to consume the final token.

Another way that the parser makes space is to look for Keyword shapes in the Signature. A Keyword is a word that's special to this command. For example in the `if` command, `else` is a keyword. When it is found in the arguments, the parser will use it as a signpost for where to make space for each Shape. The tokens leading up to the `else` will then feed into the parts of the Signature before the `else`, and the tokens following are consumed by the `else` and the Shapes that follow.

