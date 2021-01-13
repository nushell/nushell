use nu_engine::EvaluationContext;
use nu_source::Text;

pub fn maybe_print_errors(context: &EvaluationContext, source: Text) -> bool {
    let errors = context.current_errors.clone();
    let mut errors = errors.lock();

    if errors.len() > 0 {
        let error = errors[0].clone();
        *errors = vec![];

        crate::script::print_err(error, &source);
        true
    } else {
        false
    }
}
