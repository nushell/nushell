use nu_source::Text;

use crate::EvaluationContext;

pub fn maybe_print_errors(context: &EvaluationContext, source: Text) -> bool {
    let errors = context.engine_state.current_errors.clone();
    let mut errors = errors.lock();

    if errors.len() > 0 {
        let error = errors[0].clone();
        *errors = vec![];

        context.engine_state.host.lock().print_err(error, &source);
        true
    } else {
        false
    }
}
