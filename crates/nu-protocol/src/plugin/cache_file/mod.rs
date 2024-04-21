use std::{
    io::{Read, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::{PluginIdentity, PluginSignature, ShellError, Span};

// This has a big impact on performance
const BUFFER_SIZE: usize = 65536;

// Chose settings at the low end, because we're just trying to get the maximum speed
const COMPRESSION_QUALITY: u32 = 1;
const WIN_SIZE: u32 = 20; // recommended 20-22

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginCacheFile {
    /// The Nushell version that last updated the file.
    pub nushell_version: String,

    /// The installed plugins.
    pub plugins: Vec<PluginCacheItem>,
}

impl Default for PluginCacheFile {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginCacheFile {
    /// Create a new, empty plugin cache file.
    pub fn new() -> PluginCacheFile {
        PluginCacheFile {
            nushell_version: env!("CARGO_PKG_VERSION").to_owned(),
            plugins: vec![],
        }
    }

    /// Read the plugin cache file from a reader, e.g. [`File`](std::fs::File).
    pub fn read_from(
        reader: impl Read,
        error_span: Option<Span>,
    ) -> Result<PluginCacheFile, ShellError> {
        // Format is brotli compressed messagepack
        let brotli_reader = brotli::Decompressor::new(reader, BUFFER_SIZE);

        rmp_serde::from_read(brotli_reader).map_err(|err| ShellError::GenericError {
            error: format!("Failed to load plugin file: {err}"),
            msg: "plugin file load attempted here".into(),
            span: error_span,
            help: Some(
                "it may be corrupt. Try deleting it and registering your plugins again".into(),
            ),
            inner: vec![],
        })
    }

    /// Write the plugin cache file to a writer, e.g. [`File`](std::fs::File).
    ///
    /// The `nushell_version` will be updated to the current version before writing.
    pub fn write_to(
        &mut self,
        writer: impl Write,
        error_span: Option<Span>,
    ) -> Result<(), ShellError> {
        // Update the Nushell version before writing
        self.nushell_version = env!("CARGO_PKG_VERSION").to_owned();

        // Format is brotli compressed messagepack
        let mut brotli_writer =
            brotli::CompressorWriter::new(writer, BUFFER_SIZE, COMPRESSION_QUALITY, WIN_SIZE);

        rmp_serde::encode::write_named(&mut brotli_writer, self)
            .map_err(|err| err.to_string())
            .and_then(|_| brotli_writer.flush().map_err(|err| err.to_string()))
            .map_err(|err| ShellError::GenericError {
                error: "Failed to save plugin file".into(),
                msg: "plugin file save attempted here".into(),
                span: error_span,
                help: Some(err.to_string()),
                inner: vec![],
            })
    }

    /// Insert or update a plugin in the plugin cache file.
    pub fn upsert_plugin(&mut self, item: PluginCacheItem) {
        if let Some(existing_item) = self.plugins.iter_mut().find(|p| p.name == item.name) {
            *existing_item = item;
        } else {
            self.plugins.push(item);

            // Sort the plugins for consistency
            self.plugins
                .sort_by(|item1, item2| item1.name.cmp(&item2.name));
        }
    }

    /// Remove a plugin from the plugin cache file by name.
    pub fn remove_plugin(&mut self, name: &str) {
        self.plugins.retain_mut(|item| item.name != name)
    }
}

/// A single plugin definition from a [`PluginCacheFile`].
///
/// Contains the information necessary for the [`PluginIdentity`], as well as possibly valid data
/// about the plugin including the cached command signatures.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginCacheItem {
    /// The name of the plugin, as would show in `plugin list`. This does not include the file
    /// extension or the `nu_plugin_` prefix.
    pub name: String,

    /// The path to the file.
    pub filename: PathBuf,

    /// The shell program used to run the plugin, if applicable.
    pub shell: Option<PathBuf>,

    /// Additional data that might be invalid so that we don't fail to load the whole plugin file
    /// if there's a deserialization error.
    #[serde(flatten)]
    pub data: PluginCacheItemData,
}

impl PluginCacheItem {
    /// Create a [`PluginCacheItem`] from an identity and signatures.
    pub fn new(identity: &PluginIdentity, mut commands: Vec<PluginSignature>) -> PluginCacheItem {
        // Sort the commands for consistency
        commands.sort_by(|cmd1, cmd2| cmd1.sig.name.cmp(&cmd2.sig.name));

        PluginCacheItem {
            name: identity.name().to_owned(),
            filename: identity.filename().to_owned(),
            shell: identity.shell().map(|p| p.to_owned()),
            data: PluginCacheItemData::Valid { commands },
        }
    }
}

/// Possibly valid data about a plugin in a [`PluginCacheFile`]. If deserialization fails, it will
/// be `Invalid`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PluginCacheItemData {
    Valid {
        /// Signatures and examples for each command provided by the plugin.
        commands: Vec<PluginSignature>,
    },
    #[serde(
        serialize_with = "serialize_invalid",
        deserialize_with = "deserialize_invalid"
    )]
    Invalid,
}

fn serialize_invalid<S>(serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    ().serialize(serializer)
}

fn deserialize_invalid<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde::de::IgnoredAny::deserialize(deserializer)?;
    Ok(())
}

#[cfg(test)]
mod tests;
