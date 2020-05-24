# trim

Trim leading and following whitespace from text data

## Example

```shell
> echo "    Hello world"
    Hello world
> echo "    Hello world" | trim
Hello world
```

Trim can also be passed a list or table of text, for which it will trim each item individually.
It will fail if any element in the list or table is not of type String.
```shell
> open greetings.json | to json
["  hi  ", "  hello  ", "  wassup  "]
> open greetings.json | trim | to json
["hi", "hello", "wassup"]
```
```shell
> cargo search shells --limit 10 | lines | parse "{crate_name} = {version} #{description}"
━━━┯━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │ crate_name         │ version                     │ description
───┼────────────────────┼─────────────────────────────┼──────────────────────────────────────────────────────────────────────────────────
 0 │ shells             │ "0.2.0"                     │  Sugar-coating for invoking shell commands directly from Rust.
 1 │ ion-shell          │ "0.0.0"                     │  The Ion Shell
 2 │ shell-words        │ "0.1.0"                     │  Process command line according to parsing rules of UNIX shell
 3 │ nu                 │ "0.5.0"                     │  A shell for the GitHub era
 4 │ dotenv-shell       │ "1.0.1"                     │  Launch a new shell (or another program) with your loaded dotenv
 5 │ shell_completion   │ "0.0.1"                     │  Write shell completion scripts in pure Rust
 6 │ shell-hist         │ "0.2.0"                     │  A CLI tool for inspecting shell history
 7 │ tokei              │ "10.0.1"                    │  A utility that allows you to count code, quickly.
 8 │ rash-shell         │ "0.1.0"                     │  A bourne-compatible shell inspired by dash
 9 │ rust_keylock_shell │ "0.10.0"                    │  Shell access to the rust-keylock. rust-keylock is a password manager with goals
   │                    │                             │ to be Secure, …
━━━┷━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> cargo search shells --limit 10 | lines | parse "{crate_name} = {version} #{description}" | trim
━━━┯━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │ crate_name         │ version  │ description
───┼────────────────────┼──────────┼──────────────────────────────────────────────────────────────────────────────────
 0 │ shells             │ "0.2.0"  │  Sugar-coating for invoking shell commands directly from Rust.
 1 │ ion-shell          │ "0.0.0"  │  The Ion Shell
 2 │ shell-words        │ "0.1.0"  │  Process command line according to parsing rules of UNIX shell
 3 │ nu                 │ "0.5.0"  │  A shell for the GitHub era
 4 │ dotenv-shell       │ "1.0.1"  │  Launch a new shell (or another program) with your loaded dotenv
 5 │ shell_completion   │ "0.0.1"  │  Write shell completion scripts in pure Rust
 6 │ shell-hist         │ "0.2.0"  │  A CLI tool for inspecting shell history
 7 │ tokei              │ "10.0.1" │  A utility that allows you to count code, quickly.
 8 │ rash-shell         │ "0.1.0"  │  A bourne-compatible shell inspired by dash
 9 │ rust_keylock_shell │ "0.10.0" │  Shell access to the rust-keylock. rust-keylock is a password manager with goals
   │                    │          │ to be Secure, …
━━━┷━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
