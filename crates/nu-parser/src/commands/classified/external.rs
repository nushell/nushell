use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebug, Span, Tag};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalArg {
    pub arg: String,
    pub tag: Tag,
}

impl std::ops::Deref for ExternalArg {
    type Target = str;

    fn deref(&self) -> &str {
        &self.arg
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalArgs {
    pub list: Vec<ExternalArg>,
    pub span: Span,
}

impl ExternalArgs {
    pub fn iter(&self) -> impl Iterator<Item = &ExternalArg> {
        self.list.iter()
    }
}

impl std::ops::Deref for ExternalArgs {
    type Target = [ExternalArg];

    fn deref(&self) -> &[ExternalArg] {
        &self.list
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalCommand {
    pub name: String,

    pub name_tag: Tag,
    pub args: ExternalArgs,
}

impl PrettyDebug for ExternalCommand {
    fn pretty(&self) -> DebugDocBuilder {
        b::typed(
            "external command",
            b::description(&self.name)
                + b::preceded(
                    b::space(),
                    b::intersperse(
                        self.args.iter().map(|a| b::primitive(a.arg.to_string())),
                        b::space(),
                    ),
                ),
        )
    }
}

impl HasSpan for ExternalCommand {
    fn span(&self) -> Span {
        self.name_tag.span.until(self.args.span)
    }
}
