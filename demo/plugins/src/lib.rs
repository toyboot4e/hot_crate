use std::any::TypeId;

use plugin_api::Plugin;

#[derive(Debug)]
pub struct PluginA {
    _typeid_of_x: TypeId,
}

impl Plugin for PluginA {}

#[derive(Debug)]
pub struct PluginB {}

impl Plugin for PluginB {}

#[no_mangle]
pub extern "C" fn load_plugin() -> Box<Box<dyn Plugin>> {
    Box::new(Box::new(PluginA {
        _typeid_of_x: TypeId::of::<plugin_api::X>(),
    }))
}
