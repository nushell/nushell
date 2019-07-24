use crate::commands::command::SinkCommandArgs;
use crate::errors::ShellError;
use crate::format::GenericView;
use crate::prelude::*;

pub fn autoview(args: SinkCommandArgs) -> Result<(), ShellError> {
    if args.input.len() > 0 {
        if let Spanned {
            item: Value::Binary(_),
            ..
        } = args.input[0]
        {
            args.ctx.get_sink("binaryview").run(args)?;
        } else if is_single_text_value(&args.input) {
            args.ctx.get_sink("textview").run(args)?;
        } else if equal_shapes(&args.input) {
            args.ctx.get_sink("table").run(args)?;
        } else {
            let mut host = args.ctx.host.lock().unwrap();
            for i in args.input.iter() {
                let view = GenericView::new(&i);
                handle_unexpected(&mut *host, |host| crate::format::print_view(&view, host));
                host.stdout("");
            }
        }
    }

    Ok(())
}

fn equal_shapes(input: &Vec<Spanned<Value>>) -> bool {
    let mut items = input.iter();

    let item = match items.next() {
        Some(item) => item,
        None => return false,
    };

    let desc = item.data_descriptors();

    for item in items {
        if desc != item.data_descriptors() {
            return false;
        }
    }

    true
}

fn is_single_text_value(input: &Vec<Spanned<Value>>) -> bool {
    if input.len() != 1 {
        return false;
    }
    if let Spanned {
        item: Value::Primitive(Primitive::String(_)),
        ..
    } = input[0]
    {
        true
    } else {
        false
    }
}
