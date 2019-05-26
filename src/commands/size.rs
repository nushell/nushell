use crate::errors::ShellError;
use crate::object::dict::Dictionary;
use crate::object::Value;
use crate::prelude::*;
use std::fs::File;
use std::io::prelude::*;

pub fn size(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.args.is_empty() {
        return Err(ShellError::string("size requires at least one file"));
    }
    let cwd = args.env.lock().unwrap().cwd().to_path_buf();

    let mut contents = String::new();
    let mut total_lines = 0;
    let mut total_words = 0;
    let mut total_chars = 0;
    let mut total_bytes = 0;

    let mut list = VecDeque::new();
    for name in args.args {
        let name = name.as_string()?;
        let path = cwd.join(&name);
        let mut file = File::open(path)?;

        file.read_to_string(&mut contents)?;
        let (lines, words, chars, bytes) = count(&contents);

        total_lines += lines;
        total_words += words;
        total_chars += chars;
        total_bytes += bytes;

        list.push_back(dict(&name, lines, words, chars, bytes));
        contents.clear();
    }
    list.push_back(dict(
        &"total".to_string(),
        total_lines,
        total_words,
        total_chars,
        total_bytes,
    ));

    Ok(list.boxed())
}

fn count(contents: &str) -> (i64, i64, i64, i64) {
    let mut lines: i64 = 0;
    let mut words: i64 = 0;
    let mut chars: i64 = 0;
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

    (lines, words, chars, contents.len() as i64)
}

fn dict(name: &str, lines: i64, words: i64, chars: i64, bytes: i64) -> ReturnValue {
    let mut dict = Dictionary::default();
    dict.add("name", Value::string(name.to_owned()));
    dict.add("lines", Value::int(lines));
    dict.add("words", Value::int(words));
    dict.add("chars", Value::int(chars));
    dict.add("max length", Value::int(bytes));

    ReturnValue::Value(Value::Object(dict))
}
