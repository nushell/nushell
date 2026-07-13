use std::fmt::Display;

/// Metadata about the two kinds of Nushell configuration files.
///
/// Each variant knows its file name, its embedded default content, and
/// its scaffold template.
///
/// This was moved here from `nu-utils` because it is config infrastructure,
/// not a general utility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigFileKind {
    Config,
    Env,
}

impl ConfigFileKind {
    /// The compiled-in default content (evaluated before the user's file).
    pub const fn default(self) -> &'static str {
        match self {
            Self::Config => include_str!("../default_files/default_config.nu"),
            Self::Env => include_str!("../default_files/default_env.nu"),
        }
    }

    /// The scaffold content written when the file does not exist on first
    /// startup.
    pub const fn scaffold(self) -> &'static str {
        match self {
            Self::Config => include_str!("../default_files/scaffold_config.nu"),
            Self::Env => include_str!("../default_files/scaffold_env.nu"),
        }
    }

    /// The full doc-commented template written by `config nu` / `config env`.
    pub const fn doc(self) -> &'static str {
        match self {
            Self::Config => include_str!("../default_files/doc_config.nu"),
            Self::Env => include_str!("../default_files/doc_env.nu"),
        }
    }

    /// Human-readable name: `"Config"` or `"Environment config"`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Config => "Config",
            Self::Env => "Environment config",
        }
    }

    /// File name: `"config.nu"` or `"env.nu"`.
    pub const fn path(self) -> &'static str {
        match self {
            Self::Config => "config.nu",
            Self::Env => "env.nu",
        }
    }

    /// Compiled-in default file name: `"default_config.nu"` or
    /// `"default_env.nu"`.
    pub const fn default_path(self) -> &'static str {
        match self {
            Self::Config => "default_config.nu",
            Self::Env => "default_env.nu",
        }
    }
}

impl Display for ConfigFileKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
