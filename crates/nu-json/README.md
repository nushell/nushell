# nu-json

[![crate](https://img.shields.io/crates/v/nu-json.svg?style=flat-square)](https://crates.io/crates/nu-json)

> a fork of [serde_hjson](https://crates.io/crates/serde-hjson). 

> The changes made to this crate are kept in [CHANGELOG](/crates/nu-json/CHANGELOG.md).


The Rust implementation of Hjson is based on the [Serde JSON Serialization Library](https://github.com/serde-rs/json). 

This crate is a Rust library for parsing and generating Human JSON [Hjson](https://hjson.github.io). It is built upon [Serde](https://github.com/serde-rs/serde), a high performance generic serialization framework.

# Install

This crate works with Cargo and can be found on [crates.io](https://crates.io/crates/nu-json) with a `Cargo.toml` like:

```toml
[dependencies]
serde = "1"
nu-json = "0.76"
```

## From the Commandline

Add with:
```
 cargo add serde
 cargo add nu-json
```

# Usage

```rust
extern crate serde;
extern crate nu_json;

use nu_json::{Map,Value};

fn main() {

    // Now let's look at decoding Hjson data

    let sample_text=r#"
    {
        # specify rate in requests/second
        rate: 1000
        array:
        [
            foo
            bar
        ]
    }"#;

    // Decode and unwrap.
    let mut sample: Map<String, Value> = nu_json::from_str(&sample_text).unwrap();

    // scope to control lifetime of borrow
    {
        // Extract the rate
        let rate = sample.get("rate").unwrap().as_f64().unwrap();
        println!("rate: {}", rate);

        // Extract the array
        let array : &mut Vec<Value> = sample.get_mut("array").unwrap().as_array_mut().unwrap();
        println!("first: {}", array.first().unwrap());

        // Add a value
        array.push(Value::String("baz".to_string()));
    }

    // Encode to Hjson
    let sample2 = nu_json::to_string(&sample).unwrap();
    println!("Hjson:\n{}", sample2);
}
```
# DOCS

At the moment, the documentation on [serde_hjson](https://docs.rs/serde-hjson/0.9.1/serde_hjson/) / [serde_json](https://docs.rs/serde_json/1.0.93/serde_json/) is also relevant for nu-json.
