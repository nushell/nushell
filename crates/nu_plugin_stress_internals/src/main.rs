use std::{
    error::Error,
    ffi::OsStr,
    io::{BufRead, BufReader, Write},
};

use interprocess::local_socket::{
    self, traits::Stream, GenericFilePath, GenericNamespaced, ToFsName, ToNsName,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug)]
struct Options {
    refuse_local_socket: bool,
    advertise_local_socket: bool,
    exit_before_hello: bool,
    exit_early: bool,
    wrong_version: bool,
    local_socket_path: Option<String>,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    eprintln!("stress_internals: args: {args:?}");

    // Parse options from environment variables
    fn has_env(var: &str) -> bool {
        std::env::var(var).is_ok()
    }
    let mut opts = Options {
        refuse_local_socket: has_env("STRESS_REFUSE_LOCAL_SOCKET"),
        advertise_local_socket: has_env("STRESS_ADVERTISE_LOCAL_SOCKET"),
        exit_before_hello: has_env("STRESS_EXIT_BEFORE_HELLO"),
        exit_early: has_env("STRESS_EXIT_EARLY"),
        wrong_version: has_env("STRESS_WRONG_VERSION"),
        local_socket_path: None,
    };

    let (mut input, mut output): (Box<dyn BufRead>, Box<dyn Write>) =
        match args.get(1).map(|s| s.as_str()) {
            Some("--stdio") => (
                Box::new(std::io::stdin().lock()),
                Box::new(std::io::stdout()),
            ),
            Some("--local-socket") => {
                opts.local_socket_path = Some(args[2].clone());
                if opts.refuse_local_socket {
                    std::process::exit(1)
                } else {
                    let name = if cfg!(windows) {
                        OsStr::new(&args[2]).to_ns_name::<GenericNamespaced>()?
                    } else {
                        OsStr::new(&args[2]).to_fs_name::<GenericFilePath>()?
                    };
                    let in_socket = local_socket::Stream::connect(name.clone())?;
                    let out_socket = local_socket::Stream::connect(name)?;

                    (Box::new(BufReader::new(in_socket)), Box::new(out_socket))
                }
            }
            None => {
                eprintln!("Run nu_plugin_stress_internals as a plugin from inside nushell");
                std::process::exit(1)
            }
            _ => {
                eprintln!("Received args I don't understand: {args:?}");
                std::process::exit(1)
            }
        };

    // Send encoding format
    output.write_all(b"\x04json")?;
    output.flush()?;

    // Test exiting without `Hello`
    if opts.exit_before_hello {
        std::process::exit(1)
    }

    // Read `Hello` message
    let mut de = serde_json::Deserializer::from_reader(&mut input);
    let hello: Value = Value::deserialize(&mut de)?;

    assert!(hello.get("Hello").is_some());

    // Send `Hello` message
    write(
        &mut output,
        &json!({
            "Hello": {
                "protocol": "nu-plugin",
                "version": if opts.wrong_version {
                    "0.0.0"
                } else {
                    env!("CARGO_PKG_VERSION")
                },
                "features": if opts.advertise_local_socket {
                    vec![json!({"name": "LocalSocket"})]
                } else {
                    vec![]
                },
            }
        }),
    )?;

    if opts.exit_early {
        // Exit without handling anything other than Hello
        std::process::exit(0);
    }

    // Parse incoming messages
    loop {
        match Value::deserialize(&mut de) {
            Ok(message) => handle_message(&mut output, &opts, &message)?,
            Err(err) => {
                if err.is_eof() {
                    break;
                } else if err.is_io() {
                    std::process::exit(1);
                } else {
                    return Err(err.into());
                }
            }
        }
    }

    Ok(())
}

fn handle_message(
    output: &mut impl Write,
    opts: &Options,
    message: &Value,
) -> Result<(), Box<dyn Error>> {
    if let Some(plugin_call) = message.get("Call") {
        let (id, plugin_call) = (&plugin_call[0], &plugin_call[1]);
        if plugin_call.as_str() == Some("Metadata") {
            write(
                output,
                &json!({
                    "CallResponse": [
                        id,
                        {
                            "Metadata": {
                                "version": env!("CARGO_PKG_VERSION"),
                            }
                        }
                    ]
                }),
            )
        } else if plugin_call.as_str() == Some("Signature") {
            write(
                output,
                &json!({
                    "CallResponse": [
                        id,
                        {
                            "Signature": signatures(),
                        }
                    ]
                }),
            )
        } else if let Some(call_info) = plugin_call.get("Run") {
            if call_info["name"].as_str() == Some("stress_internals") {
                // Just return debug of opts
                let return_value = json!({
                    "String": {
                        "val": format!("{opts:?}"),
                        "span": &call_info["call"]["head"],
                    }
                });
                write(
                    output,
                    &json!({
                        "CallResponse": [
                            id,
                            {
                                "PipelineData": {
                                    "Value": [return_value, null]
                                }
                            }
                        ]
                    }),
                )
            } else {
                Err(format!("unknown call name: {call_info}").into())
            }
        } else {
            Err(format!("unknown plugin call: {plugin_call}").into())
        }
    } else if message.as_str() == Some("Goodbye") {
        std::process::exit(0);
    } else {
        Err(format!("unknown message: {message}").into())
    }
}

fn signatures() -> Vec<Value> {
    vec![json!({
        "sig": {
            "name": "stress_internals",
            "description": "Used to test behavior of plugin protocol",
            "extra_description": "",
            "search_terms": [],
            "required_positional": [],
            "optional_positional": [],
            "rest_positional": null,
            "named": [],
            "input_output_types": [],
            "allow_variants_without_examples": false,
            "is_filter": false,
            "creates_scope": false,
            "allows_unknown_args": false,
            "category": "Experimental",
        },
        "examples": [],
    })]
}

fn write(output: &mut impl Write, value: &Value) -> Result<(), Box<dyn Error>> {
    serde_json::to_writer(&mut *output, value)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}
