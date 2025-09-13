use std::{
    collections::BTreeMap,
    sync::{Mutex, atomic::AtomicU64},
};

use handle_custom_value::HandleCustomValue;
use nu_plugin::{EngineInterface, MsgPackSerializer, Plugin, PluginCommand, serve_plugin};

mod cool_custom_value;
mod handle_custom_value;
mod second_custom_value;

mod drop_check;
mod generate;
mod generate2;
mod handle_get;
mod handle_make;
mod handle_update;
mod update;
mod update_arg;

use drop_check::{DropCheck, DropCheckValue};
use generate::Generate;
use generate2::Generate2;
use handle_get::HandleGet;
use handle_make::HandleMake;
use handle_update::HandleUpdate;
use nu_protocol::{CustomValue, LabeledError, Spanned, Value};
use update::Update;
use update_arg::UpdateArg;

#[derive(Default)]
pub struct CustomValuePlugin {
    counter: AtomicU64,
    handles: Mutex<BTreeMap<u64, Value>>,
}

impl CustomValuePlugin {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Plugin for CustomValuePlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(Generate),
            Box::new(Generate2),
            Box::new(Update),
            Box::new(UpdateArg),
            Box::new(DropCheck),
            Box::new(HandleGet),
            Box::new(HandleMake),
            Box::new(HandleUpdate),
        ]
    }

    fn custom_value_to_base_value(
        &self,
        _engine: &EngineInterface,
        custom_value: Spanned<Box<dyn CustomValue>>,
    ) -> Result<Value, LabeledError> {
        // HandleCustomValue depends on the plugin state to get.
        if let Some(handle) = custom_value
            .item
            .as_any()
            .downcast_ref::<HandleCustomValue>()
        {
            Ok(self
                .handles
                .lock()
                .map_err(|err| LabeledError::new(err.to_string()))?
                .get(&handle.0)
                .cloned()
                .unwrap_or_else(|| Value::nothing(custom_value.span)))
        } else {
            custom_value
                .item
                .to_base_value(custom_value.span)
                .map_err(|err| err.into())
        }
    }

    fn custom_value_dropped(
        &self,
        _engine: &EngineInterface,
        custom_value: Box<dyn CustomValue>,
    ) -> Result<(), LabeledError> {
        // This is how we implement our drop behavior.
        if let Some(drop_check) = custom_value.as_any().downcast_ref::<DropCheckValue>() {
            drop_check.notify();
        } else if let Some(handle) = custom_value.as_any().downcast_ref::<HandleCustomValue>()
            && let Ok(mut handles) = self.handles.lock()
        {
            handles.remove(&handle.0);
        }
        Ok(())
    }
}

fn main() {
    serve_plugin(&CustomValuePlugin::default(), MsgPackSerializer {})
}
