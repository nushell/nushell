# Meaningful Primitive Tokens

-   `int`
-   `decimal`
-   `op::name`
-   `dot`
-   `dotdot`
-   `string`
-   `var::it`
-   `var::other`
-   `external-command`
-   `pattern::glob`
-   `word`
-   `comment`
-   `whitespace`
-   `separator`
-   `longhand-flag`
-   `shorthand-flag`

# Grouped Tokens

-   `(call head ...tail)`
-   `(list ...nodes)`
-   `(paren ...nodes)`
-   `(square ...nodes)`
-   `(curly ...nodes)`
-   `(pipeline ...elements) where elements: pipeline-element`
-   `(pipeline-element pipe? token)`

# Atomic Tokens

-   `(unit number unit) where number: number, unit: unit`

# Expression

```
start(ExpressionStart) continuation(ExpressionContinuation)* ->
```

## ExpressionStart

```
word -> String
unit -> Unit
number -> Number
string -> String
var::it -> Var::It
var::other -> Var::Other
pattern::glob -> Pattern::Glob
square -> Array
```

## TightExpressionContinuation

```
dot AnyExpression -> Member
dodot AnyExpression -> RangeContinuation
```

## InfixExpressionContinuation

```
whitespace op whitespace AnyExpression -> InfixContinuation
```

## Member

```
int -> Member::Int
word -> Member::Word
string -> Member::String
```
