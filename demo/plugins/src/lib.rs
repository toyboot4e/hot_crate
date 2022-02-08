use std::any::TypeId;

use plugin_api::Plugin;

#[derive(Debug)]
pub struct PluginA {
    _ty: TypeId,
}

impl Plugin for PluginA {}

#[derive(Debug)]
pub struct PluginB {}

impl Plugin for PluginB {}

#[no_mangle]
pub extern "C" fn load() -> Box<dyn Plugin> {
    // change "current plugin"
    Box::new(PluginA {
        _ty: TypeId::of::<plugin_api::X>(),
    })
    // Box::new(PluginB {})
}
