use nu_errors::ShellError;
use nu_source::Text;

use crate::EvaluationContext;

pub fn maybe_print_errors(context: &EvaluationContext, source: Text) -> bool {
    let errors = context.current_errors.clone();
    let mut errors = errors.lock();

    if errors.len() > 0 {
        let error = errors[0].clone();
        *errors = vec![];

        print_err(error, &source, context);
        true
    } else {
        false
    }
}

pub fn print_err(err: ShellError, source: &Text, ctx: &EvaluationContext) {
    if let Some(diag) = err.into_diagnostic() {
        let source = source.to_string();
        let mut files = codespan_reporting::files::SimpleFiles::new();
        files.add("shell", source);

        let writer = ctx.host.lock().err_termcolor();
        let config = codespan_reporting::term::Config::default();

        let _ = std::panic::catch_unwind(move || {
            let _ = codespan_reporting::term::emit(&mut writer.lock(), &config, &files, &diag);
        });
    }
}
