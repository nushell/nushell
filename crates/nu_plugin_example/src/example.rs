use nu_plugin::{EvaluatedCall, LabeledError};
use nu_protocol::{Record, Value};
pub struct Example;

impl Example {
    pub fn config(
        &self,
        config: &Option<Value>,
        call: &EvaluatedCall,
    ) -> Result<Value, LabeledError> {
        match config {
            Some(config) => Ok(config.clone()),
            None => Err(LabeledError {
                label: "No config sent".into(),
                msg: "Configuration for this plugin was not found in `$env.config.plugins.example`"
                    .into(),
                span: Some(call.head),
            }),
        }
    }

    fn print_values(
        &self,
        index: u32,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<(), LabeledError> {
        // Note. When debugging your plugin, you may want to print something to the console
        // Use the eprintln macro to print your messages. Trying to print to stdout will
        // cause a decoding error for your message
        eprintln!("Calling test {index} signature");
        eprintln!("value received {input:?}");

        // To extract the arguments from the Call object you can use the functions req, has_flag,
        // opt, rest, and get_flag
        //
        // Note that plugin calls only accept simple arguments, this means that you can
        // pass to the plug in Int and String. This should be improved when the plugin has
        // the ability to call back to NuShell to extract more information
        // Keep this in mind when designing your plugin signatures
        let a: i64 = call.req(0)?;
        let b: String = call.req(1)?;
        let flag = call.has_flag("flag")?;
        let opt: Option<i64> = call.opt(2)?;
        let named: Option<String> = call.get_flag("named")?;
        let rest: Vec<String> = call.rest(3)?;

        eprintln!("Required values");
        eprintln!("a: {a:}");
        eprintln!("b: {b:}");
        eprintln!("flag: {flag:}");
        eprintln!("rest: {rest:?}");

        if let Some(v) = opt {
            eprintln!("Found optional value opt: {v:}")
        } else {
            eprintln!("No optional value found")
        }

        if let Some(v) = named {
            eprintln!("Named value: {v:?}")
        } else {
            eprintln!("No named value found")
        }

        Ok(())
    }

    pub fn test1(&self, call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
        self.print_values(1, call, input)?;

        Ok(Value::nothing(call.head))
    }

    pub fn test2(&self, call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
        self.print_values(2, call, input)?;

        let cols = vec!["one".to_string(), "two".to_string(), "three".to_string()];

        let vals = (0..10i64)
            .map(|i| {
                let vals = (0..3)
                    .map(|v| Value::int(v * i, call.head))
                    .collect::<Vec<Value>>();

                Value::record(
                    Record::from_raw_cols_vals_unchecked(cols.clone(), vals),
                    call.head,
                )
            })
            .collect::<Vec<Value>>();

        Ok(Value::list(vals, call.head))
    }

    pub fn test3(&self, call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
        self.print_values(3, call, input)?;

        Err(LabeledError {
            label: "ERROR from plugin".into(),
            msg: "error message pointing to call head span".into(),
            span: Some(call.head),
        })
    }
}
