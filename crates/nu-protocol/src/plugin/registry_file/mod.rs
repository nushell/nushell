use std::{
    io::{Read, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::{PluginIdentity, PluginMetadata, PluginSignature, ShellError, Span};

// This has a big impact on performance
const BUFFER_SIZE: usize = 65536;

// Chose settings at the low end, because we're just trying to get the maximum speed
const COMPRESSION_QUALITY: u32 = 3; // 1 can be very bad
const WIN_SIZE: u32 = 20; // recommended 20-22

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginRegistryFile {
    /// The Nushell version that last updated the file.
    pub nushell_version: String,

    /// The installed plugins.
    pub plugins: Vec<PluginRegistryItem>,
}

impl Default for PluginRegistryFile {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistryFile {
    /// Create a new, empty plugin registry file.
    pub fn new() -> PluginRegistryFile {
        PluginRegistryFile {
            nushell_version: env!("CARGO_PKG_VERSION").to_owned(),
            plugins: vec![],
        }
    }

    /// Read the plugin registry file from a reader, e.g. [`File`](std::fs::File).
    pub fn read_from(
        reader: impl Read,
        error_span: Option<Span>,
    ) -> Result<PluginRegistryFile, ShellError> {
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

    /// Write the plugin registry file to a writer, e.g. [`File`](std::fs::File).
    ///
    /// The `nushell_version` will be updated to the current version before writing.
    pub fn write_to(
        &mut self,
        writer: impl Write,
        error_span: Option<Span>,
    ) -> Result<(), ShellError> {
        // Update the Nushell version before writing
        env!("CARGO_PKG_VERSION").clone_into(&mut self.nushell_version);

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

    /// Insert or update a plugin in the plugin registry file.
    pub fn upsert_plugin(&mut self, item: PluginRegistryItem) {
        if let Some(existing_item) = self.plugins.iter_mut().find(|p| p.name == item.name) {
            *existing_item = item;
        } else {
            self.plugins.push(item);

            // Sort the plugins for consistency
            self.plugins
                .sort_by(|item1, item2| item1.name.cmp(&item2.name));
        }
    }
}

/// A single plugin definition from a [`PluginRegistryFile`].
///
/// Contains the information necessary for the [`PluginIdentity`], as well as possibly valid data
/// about the plugin including the registered command signatures.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginRegistryItem {
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
    pub data: PluginRegistryItemData,
}

impl PluginRegistryItem {
    /// Create a [`PluginRegistryItem`] from an identity, metadata, and signatures.
    pub fn new(
        identity: &PluginIdentity,
        metadata: PluginMetadata,
        mut commands: Vec<PluginSignature>,
    ) -> PluginRegistryItem {
        // Sort the commands for consistency
        commands.sort_by(|cmd1, cmd2| cmd1.sig.name.cmp(&cmd2.sig.name));

        PluginRegistryItem {
            name: identity.name().to_owned(),
            filename: identity.filename().to_owned(),
            shell: identity.shell().map(|p| p.to_owned()),
            data: PluginRegistryItemData::Valid { metadata, commands },
        }
    }
}

/// Possibly valid data about a plugin in a [`PluginRegistryFile`]. If deserialization fails, it will
/// be `Invalid`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PluginRegistryItemData {
    Valid {
        /// Metadata for the plugin, including its version.
        #[serde(default)]
        metadata: PluginMetadata,
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
