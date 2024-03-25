use nu_plugin::{serve_plugin, EngineInterface, MsgPackSerializer, Plugin, PluginCommand};

mod cool_custom_value;
mod second_custom_value;

mod drop_check;
mod generate;
mod generate2;
mod update;
mod update_arg;

use drop_check::{DropCheck, DropCheckValue};
use generate::Generate;
use generate2::Generate2;
use nu_protocol::{CustomValue, LabeledError};
use update::Update;
use update_arg::UpdateArg;

pub struct CustomValuePlugin;

impl Plugin for CustomValuePlugin {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(Generate),
            Box::new(Generate2),
            Box::new(Update),
            Box::new(UpdateArg),
            Box::new(DropCheck),
        ]
    }

    fn custom_value_dropped(
        &self,
        _engine: &EngineInterface,
        custom_value: Box<dyn CustomValue>,
    ) -> Result<(), LabeledError> {
        // This is how we implement our drop behavior for DropCheck.
        if let Some(drop_check) = custom_value.as_any().downcast_ref::<DropCheckValue>() {
            drop_check.notify();
        }
        Ok(())
    }
}

fn main() {
    serve_plugin(&CustomValuePlugin, MsgPackSerializer {})
}
