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

/// FIXME: Not FFI-safe?
#[no_mangle]
pub extern "C" fn load_plugin() -> Box<dyn Plugin> {
    // change "current plugin"
    Box::new(PluginA {
        _typeid_of_x: TypeId::of::<plugin_api::X>(),
    })
    // Box::new(PluginB {})
}
