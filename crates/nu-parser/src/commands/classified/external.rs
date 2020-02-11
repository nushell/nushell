use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebug, Span, Tag};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalArg {
    pub arg: String,
    pub tag: Tag,
}

impl ExternalArg {
    pub fn has(&self, name: &str) -> bool {
        self.arg == name
    }

    pub fn is_it(&self) -> bool {
        self.has("$it")
    }

    pub fn is_nu(&self) -> bool {
        self.has("$nu")
    }

    pub fn looks_like_it(&self) -> bool {
        self.arg.starts_with("$it") && (self.arg.starts_with("$it.") || self.is_it())
    }

    pub fn looks_like_nu(&self) -> bool {
        self.arg.starts_with("$nu") && (self.arg.starts_with("$nu.") || self.is_nu())
    }
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

impl ExternalCommand {
    pub fn has_it_argument(&self) -> bool {
        self.args.iter().any(|arg| arg.looks_like_it())
    }

    pub fn has_nu_argument(&self) -> bool {
        self.args.iter().any(|arg| arg.looks_like_nu())
    }
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
