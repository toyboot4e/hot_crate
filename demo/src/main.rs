/*!

NOTE: This example migh work if you use `libloading` directory, too, but it's proablly not the case
in real applications.

*/

use std::{thread::sleep, time::Duration};

use hot_crate::{
    libloading::Symbol,
    HotCrate,
    Utf8PathBuf,
};

use plugin_api::Plugin;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Utf8PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    let mut plugin = HotCrate::load(&root.join("Cargo.toml"), &root.join("plugins/Cargo.toml"))?;

    loop {
        let load: Symbol<extern "C" fn() -> Box<dyn Plugin>> =
            unsafe { plugin.lib().get(b"load") }.unwrap();

        println!("current plugin: {:?}", load());

        sleep(Duration::from_secs(1));

        plugin.try_reload().unwrap();
    }
}
