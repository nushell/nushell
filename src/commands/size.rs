use crate::errors::ShellError;
use crate::object::{SpannedDictBuilder, Value};
use crate::prelude::*;
use std::fs::File;
use std::io::prelude::*;

pub fn size(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.len() == 0 {
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

    let mut list: VecDeque<ReturnValue> = VecDeque::new();
    for spanned_name in args.positional_iter() {
        let name = spanned_name.as_string()?;
        let path = cwd.join(&name);
        let mut file = File::open(path)?;
        file.read_to_string(&mut contents)?;
        list.push_back(count(&name, &contents, spanned_name).into());
        contents.clear();
    }

    Ok(list.to_output_stream())
}

fn count(name: &str, contents: &str, span: impl Into<Span>) -> Spanned<Value> {
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

    let mut dict = SpannedDictBuilder::new(span);
    dict.insert("name", Value::string(name));
    dict.insert("lines", Value::int(lines));
    dict.insert("words", Value::int(words));
    dict.insert("chars", Value::int(chars));
    dict.insert("max length", Value::int(bytes));

    dict.into_spanned_value()
}
