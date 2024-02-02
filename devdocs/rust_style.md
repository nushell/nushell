# Our Rust style

## Table of contents
- [Introduction](#introduction)
- [Generally discouraged](#generally-discouraged)
- [Things we want to get better at](#things-we-want-to-get-better-at)

## Introduction
To make the collaboration on a project the scale of Nushell easy, we want to work towards a style of Rust code that can easily be understood by all of our contributors. We conservatively rely on most of [`clippy`s suggestions](https://github.com/rust-lang/rust-clippy) to get to the holy grail of "idiomatic" code. Good code in our eyes is not the most clever use of all available language features or with the most unique personal touch but readable and strikes a balance between being concise, and also unsurprising and explicit in the places where it matters.
One example of this philosophy is that we generally avoid to fight the borrow-checker in our data model but rather try to get to a correct and simple solution first and then figure out where we should reuse data to achieve the necessary performance. As we are still pre-1.0 this served us well to be able to quickly refactor or change larger parts of the code base.

## Generally discouraged
### `+nightly` language features or things only available in the most recent `+stable`
To make life for the people easier that maintain the Nushell packages in various distributions with their own release cycle of `rustc` we typically rely on slightly older Rust versions. We do not make explicit guarantees how far back in the past we live but you can find out in our [`rust-toolchain.toml`](https://github.com/nushell/nushell/blob/main/rust-toolchain.toml)
(As a rule of thumb this has been typically been approximately 2 releases behind the newest stable compiler.)
The use of nightly features is prohibited.

### Panicking
As Nushell aims to provide a reliable foundational way for folks to interact with their computer, we cannot carelessly crash the execution of their work by panicking Nushell.
Thus panicking is not an allowed error handling strategy for anything that could be triggered by user input OR behavior of the outside system. If Nushell panics this is a bug or we are against all odds already in an unrecoverable state (The system stopped cooperating, we went out of memory). The use of `.unwrap()` is thus outright banned and any uses of `.expect()` or related panicking macros like `unreachable!` should include a helpful description which assumptions have been violated.

### `unsafe` code
For any use of `unsafe` code we need to require even higher standards and additional review. If you add or alter `unsafe` blocks you have to be familiar with the promises you need to uphold as found in the [Rustonomicon](https://doc.rust-lang.org/nomicon/intro.html). All `unsafe` uses should include `// SAFETY:` comments explaining how the invariants are upheld and thus alerting you what to watch out for when making a change.
#### FFI with system calls and the outside world
As a shell Nushell needs to interact with system APIs in several places, for which FFI code with unsafe blocks may be necessary. In some cases this can be handled by safe API wrapper crates but in some cases we may choose to directly do those calls.
If you do so you need to document the system behavior on top of the Rust memory model guarantees that you uphold. This means documenting whether using a particular system call is safe to use in a particular context and all failure cases are properly recovered.
#### Implementing self-contained data structures
Another motivation for reaching to `unsafe` code might be to try to implement a particular data structure that is not expressible on safe `std` library APIs. Doing so in the Nushell code base would have to clear a high bar for need based on profiling results. Also you should first do a survey of the [crate ecosystem](https://crates.io) that there doesn't exist a usable well vetted crate that already provides safe APIs to the desired datastructure.
#### Make things go faster by removing checks
This is probably a bad idea if you feel tempted to do so. Don't
### Macros
Another advanced feature people feel tempted to use to work around perceived limitations of Rusts syntax and we are not particularly fans of are custom macros.
They have clear downsides not only in terms of readability if they locally introduce a different syntax. Most tooling apart from the compiler will struggle more with them. This limits for example consistent automatic formatting or automated refactors with `rust-analyzer`.
That you can fluently read `macro_rules!` is less likely than regular code. This can lead people to introduce funky behavior when using a macro. Be it because a macro is not following proper hygiene rules or because it introduces excessive work at compile time.

So we generally discourage the addition of macros. In a lot of cases your macro may start do something that can be expressed with functions or generics in a much more reusable fashion.
The only exceptions we may allow need to demonstrate that the macro can fix something that is otherwise extremely unreadable, error-prone, or consistently worse at compile time.
## Things we want to get better at
These are things we did pretty liberally to get Nushell off the ground, that make things harder for a high quality stable product. You may run across them but shouldn't take them as an endorsed example.
###  Liberal use of third-party dependencies
The amazing variety of crates on [crates.io](https://crates.io) allowed us to quickly get Nushell into a feature rich state but it left us with a bunch of baggage to clean up.
Each dependency introduces a compile time cost and duplicated code can add to the overall binary size. Also vetting more for correct and secure implementations takes unreasonably more time as this is also a continuous process of reacting to updates or potential vulnerabilities.

Thus we only want to accept dependencies that are essential and well tested implementations of a particular requirement of Nushells codebase.
Also as a project for the move to 1.0 we will try to unify among a set of dependencies if they possibly implement similar things in an area. We don't need three different crates with potentially perfect fit for three problems but rather one reliable crate with a maximized overlap between what it provides and what we need.
We will favor crates that are well tested and used and promise to be more stable and still frequently maintained.
### Deeply nested code
As  Nushell uses a lot of enums in its internal data representation there are a lot of `match` expressions. Combined with the need to handle a lot of edge cases and be defensive about any errors this has led to some absolutely hard to read deeply nested code (e.g. in the parser but also in the implementation of several commands).
This can be observed both as a "rightward drift" where the main part of the code is found after many levels of indentations or by long function bodies with several layers of branching with seemingly repeated branching inside the higher branch level.
This can also be exacerbated by "quick" bugfixes/enhancements that may just try to add a special case to catch a previously unexpected condition. The likelihood of introducing a bug in a sea of code duplication is high.
To combat this, consider using the early-`return` pattern to reject invalid data early in one place instead of building a tree through Rust's expression constructs with a lot of duplicated paths. Unpacking data into a type that expresses that the necessary things already have been checked and using functions to properly deal with separate and common behavior can also help.
