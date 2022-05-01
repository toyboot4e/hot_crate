//! NOTE: This example migh work if you use `libloading` directory, too, but it's proablly not the
//! case in real applications.

use std::{thread::sleep, time::Duration};

use hot_crate::{HotCrate, Utf8PathBuf};

use plugin_api::Plugin;

pub type LoadFn<'a> = hot_crate::libloading::Symbol<'a, extern "C" fn() -> Box<Box<dyn Plugin>>>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let root = Utf8PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    let mut plugin = HotCrate::load(&root.join("Cargo.toml"), &root.join("plugins/Cargo.toml"))?;
    print_current_plugin(&mut plugin);

    loop {
        sleep(Duration::from_secs(1));

        if plugin.try_reload().unwrap() {
            println!("RELOADED!");
            print_current_plugin(&mut plugin);
        }
    }
}

fn print_current_plugin(plugin: &mut HotCrate) {
    let load: LoadFn = unsafe { plugin.lib().get(b"load_plugin") }.unwrap();
    let plugin = load();

    println!(
        "TypeId of `plugin_api::X` from main: {:?}",
        std::any::TypeId::of::<plugin_api::X>()
    );

    println!("current plugin: {:?}", plugin);
}
