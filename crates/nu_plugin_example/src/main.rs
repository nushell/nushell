use nu_plugin::{serve_plugin, EvaluatedCall, Plugin};
use nu_protocol::{Category, ShellError, Signature, SyntaxShape, Value};

fn main() {
    serve_plugin(&mut Example {})
}

struct Example {}

impl Plugin for Example {
    fn signature(&self) -> Vec<Signature> {
        // It is possible to declare multiple signature in a plugin
        // Each signature will be converted to a command declaration once the
        // plugin is registered to nushell
        vec![
            Signature::build("test-1")
                .desc("Signature test 1 for plugin. Returns Value::Nothing")
                .required("a", SyntaxShape::Int, "required integer value")
                .required("b", SyntaxShape::String, "required string value")
                .switch("flag", "a flag for the signature", Some('f'))
                .optional("opt", SyntaxShape::Int, "Optional number")
                .named("named", SyntaxShape::String, "named string", Some('n'))
                .rest("rest", SyntaxShape::String, "rest value string")
                .category(Category::Experimental),
            Signature::build("test-2")
                .desc("Signature test 2 for plugin. Returns list of records")
                .required("a", SyntaxShape::Int, "required integer value")
                .required("b", SyntaxShape::String, "required string value")
                .switch("flag", "a flag for the signature", Some('f'))
                .optional("opt", SyntaxShape::Int, "Optional number")
                .named("named", SyntaxShape::String, "named string", Some('n'))
                .rest("rest", SyntaxShape::Int, "rest value int")
                .category(Category::Experimental),
        ]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, ShellError> {
        // You can use the name to identify what plugin signature was called
        match name {
            "test-1" => test1(call, input),
            "test-2" => test2(call, input),
            _ => Err(ShellError::LabeledError(
                "Plugin call with wrong name signature".into(),
                "using the wrong signature".into(),
                call.head,
            )),
        }
    }
}

fn test1(call: &EvaluatedCall, input: &Value) -> Result<Value, ShellError> {
    // Note. When debugging your plugin, you may want to print something to the console
    // Use the eprintln macro to print your messages. Trying to print to stdout will
    // cause a decoding error for your message
    eprintln!("Calling test1 signature");
    eprintln!("value received {:?}", input);

    // To extract the arguments from the Call object you can use the functions req, has_flag,
    // opt, rest, and get_flag
    //
    // Note that plugin calls only accept simple arguments, this means that you can
    // pass to the plug in Int and String. This should be improved when the plugin has
    // the ability to call back to NuShell to extract more information
    // Keep this in mind when designing your plugin signatures
    let a: i64 = call.req(0)?;
    let b: String = call.req(1)?;
    let flag = call.has_flag("flag");
    let opt: Option<i64> = call.opt(2)?;
    let named: Option<String> = call.get_flag("named")?;
    let rest: Vec<String> = call.rest(3)?;

    eprintln!("Required values");
    eprintln!("a: {:}", a);
    eprintln!("b: {:}", b);
    eprintln!("flag: {:}", flag);
    eprintln!("rest: {:?}", rest);

    match opt {
        Some(v) => eprintln!("Found optional value opt: {:}", v),
        None => eprintln!("No optional value found"),
    }

    match named {
        Some(v) => eprintln!("Named value: {:?}", v),
        None => eprintln!("No named value found"),
    }

    Ok(Value::Nothing { span: call.head })
}

fn test2(call: &EvaluatedCall, input: &Value) -> Result<Value, ShellError> {
    eprintln!("Calling test1 signature");
    eprintln!("value received {:?}", input);

    eprintln!("Arguments received");
    let a: i64 = call.req(0)?;
    let b: String = call.req(1)?;
    let flag = call.has_flag("flag");
    let opt: Option<i64> = call.opt(2)?;
    let named: Option<String> = call.get_flag("named")?;
    let rest: Vec<i64> = call.rest(3)?;

    eprintln!("Required values");
    eprintln!("a: {:}", a);
    eprintln!("b: {:}", b);
    eprintln!("flag: {:}", flag);
    eprintln!("rest: {:?}", rest);

    match opt {
        Some(v) => eprintln!("Found optional value opt: {:}", v),
        None => eprintln!("No optional value found"),
    }

    match named {
        Some(v) => eprintln!("Named value: {:?}", v),
        None => eprintln!("No named value found"),
    }

    let cols = vec!["one".to_string(), "two".to_string(), "three".to_string()];

    let vals = (0..10i64)
        .map(|i| {
            let vals = (0..3)
                .map(|v| Value::Int {
                    val: v * i,
                    span: call.head,
                })
                .collect::<Vec<Value>>();

            Value::Record {
                cols: cols.clone(),
                vals,
                span: call.head,
            }
        })
        .collect::<Vec<Value>>();

    Ok(Value::List {
        vals,
        span: call.head,
    })
}
