use crate::commands::each::group::run_block_on_vec;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
//use itertools::Itertools;
use nu_errors::ShellError;
use nu_protocol::{hir::CapturedBlock, Primitive, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;
use serde::Deserialize;

pub struct EachWindow;

#[derive(Deserialize)]
pub struct EachWindowArgs {
    window_size: Tagged<usize>,
    block: CapturedBlock,
    stride: Option<Tagged<usize>>,
}

#[async_trait]
impl WholeStreamCommand for EachWindow {
    fn name(&self) -> &str {
        "each window"
    }

    fn signature(&self) -> Signature {
        Signature::build("each window")
            .required("window_size", SyntaxShape::Int, "the size of each window")
            .named(
                "stride",
                SyntaxShape::Int,
                "the number of rows to slide over between windows",
                Some('s'),
            )
            .required(
                "block",
                SyntaxShape::Block,
                "the block to run on each group",
            )
    }

    fn usage(&self) -> &str {
        "Runs a block on sliding windows of `window_size` rows of a table at a time."
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Echo the sum of each window",
            example: "echo [1 2 3 4] | each window 2 { echo $it | math sum }",
            result: None,
        }]
    }

    async fn run(&self, raw_args: CommandArgs) -> Result<OutputStream, ShellError> {
        let context = Arc::new(EvaluationContext::from_args(&raw_args));
        let (each_args, mut input): (EachWindowArgs, _) = raw_args.process().await?;
        let block = Arc::new(Box::new(each_args.block));

        let mut window: Vec<_> = input
            .by_ref()
            .take(*each_args.window_size - 1)
            .collect::<Vec<_>>()
            .await;

        // `window` must start with dummy values, which will be removed on the first iteration
        let stride = each_args.stride.map(|x| *x).unwrap_or(1);
        window.insert(0, UntaggedValue::Primitive(Primitive::Nothing).into());

        Ok(input
            .enumerate()
            .then(move |(i, input)| {
                // This would probably be more efficient if `last` was a VecDeque
                // But we can't have that because it needs to be put into a Table
                window.remove(0);
                window.push(input);

                let block = block.clone();
                let context = context.clone();
                let local_window = window.clone();

                async move {
                    if i % stride == 0 {
                        Some(run_block_on_vec(local_window, block, context).await)
                    } else {
                        None
                    }
                }
            })
            .filter_map(|x| async { x })
            .flatten()
            .to_output_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::EachWindow;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(EachWindow {})
    }
}
