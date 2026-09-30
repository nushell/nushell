mod parser;
mod render;

/// A parsed man page containing a program description, option descriptions and
/// subcommand descriptions.
pub struct ParsedManPage {
    /// The short (usually one line) description from the `NAME` section of the 
    /// man page.
    pub description: Option<String>,
    /// Command options and their descriptions.
    pub options: Vec<ManEntry>,
    /// Subcommands/arguments and their descriptions.
    pub subcommands: Vec<ManEntry>,
}

pub struct ManEntry {
    /// The "raw" tag of the option/subcommand:
    /// 
    /// example for `man git`
    /// 
    /// options: `"-v, --version"`
    /// 
    /// subcommands: `"git-add(1)"`
    pub tag: String,
    /// The separated keys of the option/subcommand:
    /// 
    /// example for `man git`
    /// 
    /// options: `["-v", "--version"]`
    /// 
    /// subcommands: `["add"]`
    pub keys: Vec<String>,
    /// The description of the option/subcommand:
    /// 
    /// example for `man git`
    /// 
    /// options: `"Prints the Git suite version that the git program came from."`
    pub description: String,
}

impl ParsedManPage {
    /// Lookup a parsed ManEntry for an option by its name.
    pub fn option(&self, option: &str) -> Option<&ManEntry> {
        self.options.iter().find(|opt| opt.keys.contains(&option.to_string()))
    }

    /// Lookup a parsed ManEntry for a subcommand by its name.
    pub fn subcommand(&self, subcommand: &str) -> Option<&ManEntry> {
        self.subcommands.iter().find(|sub| sub.keys.contains(&subcommand.to_string()))
    }
}