# Implementing a Command

We will be learning how to write a command and see what's required to do so in small steps. For this example we are interested writing a command named `take-two` that takes two lucky values coming in.

Let's generate a table with row literal syntax first, like so:

```
> echo [[repository, LOC]; [nushell, 11161984] [scripts, 500] [nushell_book, 200]]
──────────────┬──────────
 repository   │ LOC
──────────────┼──────────
 nushell      │ 11161984
 scripts      │      500
 nushell_book │      200
──────────────┴──────────
```

_Note: values are passed around in the stream. A generated table like the example above is three row values. See the [values chapter](https://www.nushell.sh/contributor-book/values.html) to learn more about row value types._

We want to `take-two` rows from the generated table. Here is what we would like to do:

```
> echo [[repository, LOC]; [nushell, 11161984] [scripts, 500] [nushell_book, 200]] | take-two
──────────────┬──────────
 repository   │ LOC
──────────────┼──────────
 nushell      │ 11161984
 scripts      │      500
──────────────┴──────────
```

See the `take-two` command in the pipeline above? That's what we want.

Before starting writing the command it's important to understand that everything in the stream are values. Sometimes you will see a table with no columns and the reason is that none of the values coming in are row value types. For this case we call it simply a _list_, like so:

```
> echo [[repository, LOC]; [nushell, 11161984] [scripts, 500] [nushell_book, 200]] | get repository
──────────────
 nushell
 scripts
 nushell_book
──────────────
```

The example above Nu sees three values of type String (the call `get repository` returns them).

What happens if the stream has row value types and other value types? Let's find out:

```
> echo [[repository, LOC]; [nushell, 11161984] [scripts, 500] [nushell_book, 200]] | append 10
──────────────┬──────────
 repository   │ LOC
──────────────┼──────────
 nushell      │ 11161984
 scripts      │      500
 nushell_book │      200
──────────────┴──────────
────
 10
────
```

When the pipeline finishes, as soon as Nu sees a value that is of type row it will try to display it as a table with columns. The reason is that a value of type row represents rows having column cell headers and cell values.

We appended the value 10 using `append`. Nu sees **four** values in the stream, **three** of them being row types and **one** being an int value type. Therefore we see a table printed (the three row values) and a list (one int value) from the **stream**. This matters because some commands expect and assume all values coming in are row types and some do not.

For the command `take-two` we are about to implement we don't care what kind of values are coming in since we just want to **take two** values.

## Command implementation steps

To guide us in our command implementation journey we will start registering the command, as we do so we will be driving out the command.

### Register the command

All the internal commands reside in `crates/nu-command`. For our command named `take-two` we can create the source file `take_two.rs` and place it in `src/commands/take_two.rs`. Go ahead and create it:

```
> touch crates/nu-command/src/commands/take_two.rs
```

Edit the source file and add the following:

```rust
pub struct Command;
```

Before registering it to the context you need to make it available by declaring the module and re-export it in `crates/nu-command/src/commands.rs`. The commands we write are modules already (`take_two.rs` is the `take_two` module). In case you are not familiar with rust modules [read this refresher](https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html). Let's go ahead and declare it and then re-export the command's `Command` struct as `TakeTwo`, like so:

_nu-command/src/commands.rs_
```rust
# ...
pub(crate) mod take_two;
# ...

# ...
pub(crate) use take_two::Command as TakeTwo;
# ...
```

We should be ready now to register it. You need to add the command (in this case the struct will be in scope named as `TakeTwo` from the previous snippet) where they are added one by one to the context.

_src/commands/default_context.rs_
```rust
context.add_commands(vec![
    # ...
    whole_stream_command(TakeTwo),
    # ...
);
```

We will now resort to the rust compiler to make sure things are wired up correctly. Under `crates/nu-command` run:

```
> cargo build
   Compiling nu-command v0.29.2 (/Users/nu/nushell/crates/nu-command)
error[E0277]: the trait bound `take_two::Command: nu_engine::WholeStreamCommand` is not satisfied
   --> crates/nu-command/src/commands/default_context.rs:13:34
    |
13  |             whole_stream_command(TakeTwo),
    |                                  ^^^^^^^ the trait `nu_engine::WholeStreamCommand` is not implemented for `take_two::Command`
    |
   ::: /Users/nu/nushell/crates/nu-engine/src/whole_stream_command.rs:273:43
    |
273 | pub fn whole_stream_command(command: impl WholeStreamCommand + 'static) -> Command {
    |                                           ------------------ required by this bound in `whole_stream_command`

error[E0277]: the trait bound `take_two::Command: nu_engine::WholeStreamCommand` is not satisfied
   --> crates/nu-command/src/commands/default_context.rs:13:34
    |
13  |             whole_stream_command(TakeTwo),
    |                                  ^^^^^^^ the trait `nu_engine::WholeStreamCommand` is not implemented for `take_two::Command`
    |
   ::: /Users/nu/nushell/crates/nu-engine/src/whole_stream_command.rs:273:43
    |
273 | pub fn whole_stream_command(command: impl WholeStreamCommand + 'static) -> Command {
    |                                           ------------------ required by this bound in `whole_stream_command`

error: aborting due to previous error
```

That's not certainly what we expected.

It happens that Nu needs to know more about the command, including it's name and signature. We didn't do any of that and the compiler is telling us so. We seem to be missing implementing the `WholeStreamCommand` trait. This is the trait that, when implemented, allows us to state it's name, signature, and the like. 

## WholeStreamCommand

Every internal command must implement the `WholeStreamCommand` trait. Let's take a look at the required functions:

```rust
pub trait WholeStreamCommand: Send + Sync {
    fn name(&self) -> &str;
    fn signature(&self) -> Signature;
    fn usage(&self) -> &str;
}
```

_Note: For simplicity, we've removed the default implementations of the trait. If you want to read more about the trait you [can read the code here](https://github.com/nushell/nushell/blob/e09e3b01d6523309b901fb396b416146b53c2b7f/crates/nu-engine/src/whole_stream_command.rs)_

We will need to implement the required functions `name` for telling Nu the name of the command, `signature` to tell the command's signature, and `usage`. Before we start, we will also write an example of the usage implementing `examples` (not shown above).

Ready? Go ahead and edit `crates/nu-command/src/commands/take_two.rs`:

```rust
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_protocol::Signature;

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "take-two"
    }

    fn usage(&self) -> &str {
        "takes two values"
    }

    fn signature(&self) -> Signature {
        Signature::build("take-two")
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Takes two values",
                example: "echo [11161984 500 200] | take-two",
                result: None,
            },
        ]
    }
}
```

Let's try again by running `cargo build`:

```
cargo build
   Compiling nu-command v0.29.2 (/Users/nu/nushell/crates/nu-command)
    Finished dev [unoptimized + debuginfo] target(s) in 10.56s
```

Great!

Since we have been working in `crates/nu-command` and building there to shorten the build time as we work on the command, you may want to run a full build now by doing `cargo build --all --features=extra`:

Let's try entering `help take-two`:

```
> help take-two
takes two values

Usage:
  > take-two {flags}

Flags:
  -h, --help: Display this help message

Examples:
  Takes two values
  > echo [11161984 500 200] | take-two
```

We get a help description of our newly implemented `take-two` command for free. This works because we implemented `examples`. It's good practice to do so for documentation (and as we will soon see, unit testing) purposes.

Now let's run `echo [11161984 500 200] | take-two`

```
> echo [11161984 500 200] | take-two
error: Error: Unimplemented: take-two does not implement run or run_with_actions
```

That was unexpected. What happened?

We did not implement `take-two`'s logic anywhere. Nu is telling us the command does not implement `run` or `run_with_actions`. These functions are in the `WholeStreamCommand` trait and the default implementation gives the error. Since our command won't be doing any actions we can go ahead and implement `run` for the logic, let's do that:

```rust
use nu_errors::ShellError;
# ...

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(args.input.take(2).into_output_stream())
    }

# ...
```

Compile and try one more time:

```
> echo [11161984 500 200] | take-two
───┬──────────
 0 │ 11161984
 1 │      500
───┴──────────
```

Not bad. We created first a list of int values and took two of them. Let's create row type values and take two of them now:

```
> echo [[LOC]; [11161984] [500] [200]] | take-two
───┬──────────
 # │ LOC
───┼──────────
 0 │ 11161984
 1 │      500
───┴──────────
```

The command is ready.