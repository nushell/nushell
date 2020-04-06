use nu_parser::{ExternalArg, ExternalArgs, ExternalCommand};
use nu_source::{Span, SpannedItem, Tag, TaggedItem};

pub struct ExternalBuilder {
    name: String,
    args: Vec<String>,
}

impl ExternalBuilder {
    pub fn for_name(name: &str) -> ExternalBuilder {
        ExternalBuilder {
            name: name.to_string(),
            args: vec![],
        }
    }

    pub fn arg(&mut self, value: &str) -> &mut Self {
        self.args.push(value.to_string());
        self
    }

    pub fn build(&mut self) -> ExternalCommand {
        let mut path = crate::fs::binaries();
        path.push(&self.name);

        let name = path.to_string_lossy().to_string().spanned(Span::unknown());

        let args = self
            .args
            .iter()
            .map(|arg| {
                let arg = arg.tagged(Tag::unknown());

                ExternalArg {
                    arg: arg.to_string(),
                    tag: arg.tag,
                }
            })
            .collect::<Vec<_>>();

        ExternalCommand {
            name: name.to_string(),
            name_tag: Tag::unknown(),
            args: ExternalArgs {
                list: args,
                span: name.span,
            },
        }
    }
}
