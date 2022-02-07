mod call;
mod plugin_call;
mod signature;
mod value;

use nu_protocol::ShellError;

use crate::{plugin::PluginEncoder, protocol::PluginResponse};

#[derive(Clone)]
pub struct CapnpSerializer;

impl PluginEncoder for CapnpSerializer {
    fn encode_call(
        &self,
        plugin_call: &crate::protocol::PluginCall,
        writer: &mut impl std::io::Write,
    ) -> Result<(), nu_protocol::ShellError> {
        plugin_call::encode_call(plugin_call, writer)
    }

    fn decode_call(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<crate::protocol::PluginCall, nu_protocol::ShellError> {
        plugin_call::decode_call(reader)
    }

    fn encode_response(
        &self,
        plugin_response: &PluginResponse,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError> {
        plugin_call::encode_response(plugin_response, writer)
    }

    fn decode_response(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<PluginResponse, ShellError> {
        plugin_call::decode_response(reader)
    }
}
