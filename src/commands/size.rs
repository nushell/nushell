use crate::errors::ShellError;
use crate::object::dict::Dictionary;
use crate::object::Value;
use crate::prelude::*;
use std::fs::File;
use std::io::prelude::*;

pub fn size(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.positional.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Size requires a filepath",
            "needs path",
            args.name_span,
        ));
    }
    let cwd = args
        .env
        .lock()
        .unwrap()
        .front()
        .unwrap()
        .path()
        .to_path_buf();

    let mut contents = String::new();

    let mut list = VecDeque::new();
    for name in args.positional {
        let name = name.as_string()?;
        let path = cwd.join(&name);
        let mut file = File::open(path)?;
        file.read_to_string(&mut contents)?;
        list.push_back(count(&name, &contents));
        contents.clear();
    }

    Ok(list.boxed())
}

fn count(name: &str, contents: &str) -> ReturnValue {
    let mut lines: i64 = 0;
    let mut words: i64 = 0;
    let mut chars: i64 = 0;
    let bytes = contents.len() as i64;
    let mut end_of_word = true;

    for c in contents.chars() {
        chars += 1;

        match c {
            '\n' => {
                lines += 1;
                end_of_word = true;
            }
            ' ' => end_of_word = true,
            _ => {
                if end_of_word {
                    words += 1;
                }
                end_of_word = false;
            }
        }
    }

    let mut dict = Dictionary::default();
    dict.add("name", Value::string(name.to_owned()));
    dict.add("lines", Value::int(lines));
    dict.add("words", Value::int(words));
    dict.add("chars", Value::int(chars));
    dict.add("max length", Value::int(bytes));

    ReturnValue::Value(Value::Object(dict))
}
